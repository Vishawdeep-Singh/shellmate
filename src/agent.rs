use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};

use crate::api::{self, Message};
use crate::tools;

pub async fn run(client: &Client<OpenAIConfig>, prompt: String) -> Result<Option<String>> {
    let tools = tools::tools();
    let mut messages = vec![Message::User { content: prompt }];

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
