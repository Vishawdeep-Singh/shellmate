use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use std::{env, process};

use crate::tools::{ToolCall, tools};

mod tools;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
enum Message {
    User {
        content: String,
    },
    Assistant(AssistantMessage),
    Tool {
        tool_call_id: String,
        content: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct AssistantMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ToolCall>,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    tools: Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    let client = Client::with_config(config);

    let mut messages = vec![Message::User {
        content: args.prompt,
    }];

    loop {
        let response: ChatResponse = client
            .chat()
            .create_byot(ChatRequest {
                model: "anthropic/claude-haiku-4.5",
                messages: &messages,
                tools: tools(),
            })
            .await?;
        let reply = response
            .choices
            .into_iter()
            .next()
            .ok_or("no messages")?
            .message;

        if reply.tool_calls.is_empty() {
            if let Some(content) = &reply.content {
                println!("{content}");
            }
            return Ok(());
        }
        let results: Vec<Message> = reply
            .tool_calls
            .iter()
            .map(|call| Message::Tool {
                tool_call_id: call.id.clone(),
                content: call.execute(),
            })
            .collect();
        messages.push(Message::Assistant(reply));
        messages.extend(results);
    }
}
