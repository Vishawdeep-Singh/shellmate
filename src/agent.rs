use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use serde_json::Value;

use crate::api::{self, Message};
use std::io::{self, Write};
use std::path::Path;

use crate::skills::Skill;
use crate::tools;

pub struct AgentContext {
    pub tool_definitions: Value,
    pub skills: Vec<Skill>,
    pub client: Client<OpenAIConfig>,
}
impl AgentContext {
    fn new(client: &Client<OpenAIConfig>) -> Self {
        AgentContext {
            tool_definitions: tools::tool_definitions(),
            skills: Skill::scan_skills(Path::new(".claude/skills")),
            client: client.clone(),
        }
    }

    fn initial_messages(&self) -> Vec<Message> {
        let mut messages = Vec::new();
        if !self.skills.is_empty() {
            messages.push(Message::System {
                content: build_skill_prompt(&self.skills),
            });
        }
        messages
    }

    fn push_prompt(&self, messages: &mut Vec<Message>, prompt: String) {
        messages.extend(
            expand_prompt(prompt, &self.skills)
                .into_iter()
                .map(|content| Message::User { content }),
        );
    }
}

pub async fn run(client: &Client<OpenAIConfig>, prompt: String) -> Result<Option<String>> {
    let ctx = AgentContext::new(client);
    let mut messages = ctx.initial_messages();
    ctx.push_prompt(&mut messages, prompt);
    agent_loop(&mut messages, &ctx).await
}

pub async fn repl(client: &Client<OpenAIConfig>) -> Result<()> {
    let ctx = AgentContext::new(client);
    let mut messages = ctx.initial_messages();
    let mut line = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        line.clear();
        let bytes_read = io::stdin().read_line(&mut line)?;
        if bytes_read == 0 {
            println!();
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" {
            break;
        }

        ctx.push_prompt(&mut messages, input.to_string());
        match agent_loop(&mut messages, &ctx).await {
            Ok(Some(answer)) => println!("{answer}"),
            Ok(None) => {}
            Err(err) => eprintln!("error: {err:#}"),
        }
    }
    Ok(())
}

pub async fn agent_loop(messages: &mut Vec<Message>, ctx: &AgentContext) -> Result<Option<String>> {
    loop {
        let reply = api::complete(&ctx.client, messages, &ctx.tool_definitions).await?;

        if reply.tool_calls.is_empty() {
            let answer = reply.content.clone();
            messages.push(Message::Assistant(reply));
            return Ok(answer);
        }

        let futures = reply
            .tool_calls
            .iter()
            .map(|call| Message::tool_result(call, ctx));
        let results: Vec<Message> = futures::future::join_all(futures).await;
        messages.push(Message::Assistant(reply));
        messages.extend(results);
    }
}

fn build_skill_prompt(skills: &[Skill]) -> String {
    let mut prompt = String::from("You have access to the following skills:\n\n");
    for skill in skills {
        let (Some(name), Some(description)) = (skill.name(), skill.description()) else {
            continue;
        };
        prompt.push_str(&format!("- {}: {}\n", name, description));
    }

    prompt.push_str(
        "\nIf a skill matches the user's request, call the Skill tool with its name \
     and follow the instructions it returns.\n",
    );
    prompt
}
fn expand_prompt(prompt: String, skills: &[Skill]) -> Vec<String> {
    let mut remaining = prompt.as_str();
    let mut found: Vec<&Skill> = Vec::new();

    loop {
        remaining = remaining.trim_start();
        let (word, rest) = remaining
            .split_once(char::is_whitespace)
            .unwrap_or((remaining, ""));

        let Some(name) = word.strip_prefix('/') else {
            break;
        };
        let Some(skill) = skills.iter().find(|skill| skill.name() == Some(name)) else {
            break;
        };
        found.push(skill);
        remaining = rest;
    }

    if found.is_empty() {
        return vec![prompt];
    }

    let arguments = remaining.trim();
    found
        .iter()
        .map(|skill| Skill::substitute_arguments(&skill.body, arguments))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn skill(name: &str, description: &str, body: &str) -> Skill {
        Skill {
            frontmatter: HashMap::from([
                ("name".to_string(), name.to_string()),
                ("description".to_string(), description.to_string()),
            ]),
            body: body.to_string(),
            dir: PathBuf::from(format!(".claude/skills/{name}")),
        }
    }

    fn fruit_skills() -> Vec<Skill> {
        vec![
            skill(
                "apple",
                "Deploys apple",
                "Add this exact line to your response: apple-$ARGUMENTS",
            ),
            skill(
                "grape",
                "Tests grape",
                "Add this exact line to your response: grape-$ARGUMENTS",
            ),
        ]
    }

    fn expand(prompt: &str) -> Vec<String> {
        expand_prompt(prompt.to_string(), &fruit_skills())
    }

    // A slash command sends only the filled-in body, without `render`'s header.
    const APPLE: &str = "Add this exact line to your response: apple-";
    const GRAPE: &str = "Add this exact line to your response: grape-";

    const SKILL_TOOL_HINT: &str = "\nIf a skill matches the user's request, call the Skill tool \
        with its name and follow the instructions it returns.\n";

    // ---------- build_skill_prompt ----------

    #[test]
    fn skill_prompt_lists_name_and_description() {
        assert_eq!(
            build_skill_prompt(&fruit_skills()),
            format!(
                "You have access to the following skills:\n\n- apple: Deploys apple\n- grape: Tests grape\n{SKILL_TOOL_HINT}"
            )
        );
    }

    #[test]
    fn skill_prompt_skips_skill_without_description() {
        let mut skills = fruit_skills();
        skills[1].frontmatter.remove("description");

        assert_eq!(
            build_skill_prompt(&skills),
            format!(
                "You have access to the following skills:\n\n- apple: Deploys apple\n{SKILL_TOOL_HINT}"
            )
        );
    }

    // ---------- expand_prompt: not a skill ----------

    #[test]
    fn plain_prompt_is_unchanged() {
        assert_eq!(expand("fix the bug"), vec!["fix the bug"]);
    }

    #[test]
    fn slash_in_the_middle_is_unchanged() {
        assert_eq!(expand("hello /apple"), vec!["hello /apple"]);
    }

    #[test]
    fn unknown_skill_is_unchanged() {
        assert_eq!(expand("/nope 4127"), vec!["/nope 4127"]);
    }

    #[test]
    fn lone_slash_is_unchanged() {
        assert_eq!(expand("/"), vec!["/"]);
    }

    #[test]
    fn partial_skill_name_is_unchanged() {
        assert_eq!(expand("/app 4127"), vec!["/app 4127"]);
    }

    #[test]
    fn no_skills_at_all_is_unchanged() {
        assert_eq!(expand_prompt("/apple".to_string(), &[]), vec!["/apple"]);
    }

    // ---------- expand_prompt: one skill ----------

    #[test]
    fn single_skill_without_arguments() {
        assert_eq!(expand("/apple"), vec![APPLE]);
    }

    #[test]
    fn single_skill_with_arguments() {
        assert_eq!(expand("/apple 4127"), vec![format!("{APPLE}4127")]);
    }

    #[test]
    fn skill_body_without_placeholder_gets_arguments_appended() {
        let skills = vec![skill("plain", "No placeholders", "Say hello")];

        assert_eq!(
            expand_prompt("/plain to bob".to_string(), &skills),
            vec!["Say hello\n\nto bob"]
        );
    }

    // ---------- expand_prompt: several skills ----------

    #[test]
    fn consecutive_skills_each_get_the_arguments() {
        assert_eq!(
            expand("/apple /grape 4127"),
            vec![format!("{APPLE}4127"), format!("{GRAPE}4127")]
        );
    }

    #[test]
    fn skill_after_an_argument_is_not_expanded() {
        assert_eq!(
            expand("/apple 4127 /grape"),
            vec![format!("{APPLE}4127 /grape")]
        );
    }

    #[test]
    fn unknown_skill_ends_the_skill_run() {
        assert_eq!(
            expand("/apple /nope 4127"),
            vec![format!("{APPLE}/nope 4127")]
        );
    }

    #[test]
    fn extra_whitespace_between_skills_is_ignored() {
        assert_eq!(
            expand("  /apple   /grape\t4127  "),
            vec![format!("{APPLE}4127"), format!("{GRAPE}4127")]
        );
    }

    #[test]
    fn same_skill_twice_expands_twice() {
        assert_eq!(
            expand("/apple /apple 1"),
            vec![format!("{APPLE}1"), format!("{APPLE}1")]
        );
    }
}
