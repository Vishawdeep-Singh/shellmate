use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};

use crate::api::{self, Message};
use std::path::Path;

use crate::skills::Skill;
use crate::tools;

pub async fn run(client: &Client<OpenAIConfig>, prompt: String) -> Result<Option<String>> {
    let tools = tools::tools();
    let skills = Skill::scan_skills(Path::new(".claude/skills"));

    let mut messages = Vec::new();
    if !skills.is_empty() {
        messages.push(Message::System {
            content: build_skill_prompt(&skills),
        });
    }

    messages.extend(
        expand_prompt(prompt, &skills)
            .into_iter()
            .map(|content| Message::User { content }),
    );

    loop {
        let reply = api::complete(client, &messages, &tools).await?;

        if reply.tool_calls.is_empty() {
            return Ok(reply.content);
        }

        let results: Vec<Message> = reply.tool_calls.iter().map(Message::tool_result).collect();
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
        dbg!(remaining, word, rest);

        let Some(name) = word.strip_prefix('/') else {
            dbg!("stop: no slash", word);
            break;
        };
        let Some(skill) = skills.iter().find(|skill| skill.name() == Some(name)) else {
            dbg!("stop: unknown skill", name);
            break;
        };
        found.push(skill);
        remaining = rest;
    }
    dbg!(found.len(), remaining);

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
            format!("You have access to the following skills:\n\n- apple: Deploys apple\n{SKILL_TOOL_HINT}")
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
    #[test]
    fn debug_my_prompt() {
        dbg!(expand("/apple"));
    }
}
