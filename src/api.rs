use anyhow::{Context, Result};
use async_openai::{Client, config::OpenAIConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tools::ToolCall;

const MODEL: &str = "anthropic/claude-haiku-4.5";

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    User { content: String },
    Assistant(AssistantMessage),
    Tool { tool_call_id: String, content: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

impl Message {
    pub fn tool_result(call: &ToolCall) -> Self {
        Message::Tool {
            tool_call_id: call.id.clone(),
            content: call.execute(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    tools: &'a Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

pub async fn complete(
    client: &Client<OpenAIConfig>,
    messages: &[Message],
    tools: &Value,
) -> Result<AssistantMessage> {
    let response: ChatResponse = client
        .chat()
        .create_byot(ChatRequest {
            model: MODEL,
            messages,
            tools,
        })
        .await?;
    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .context("no choices in response")
}
