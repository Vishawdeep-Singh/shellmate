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
    messages.push(Message::User {
        content: expand_prompt(prompt, &skills),
    });

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
    prompt
}
fn expand_prompt(prompt: String, skills: &[Skill]) -> String {
    let Some(rest) = prompt.strip_prefix('/') else {
        return prompt;
    };
    let (name, args) = rest.split_once(' ').unwrap_or((rest, ""));
    let Some(skill) = skills.iter().find(|skill| skill.name() == Some(name)) else {
        return prompt;
    };
    let args = args.trim();
    if args.is_empty() {
        skill.body.clone()
    } else {
        format!("{}\n\n{}", skill.body, args)
    }
}
