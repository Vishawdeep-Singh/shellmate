mod agent;
mod api;
mod events;
mod models;
mod skills;
mod tools;
mod tui;
mod ui;

use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use std::{env, process};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: Option<String>,
    /// The model to use, like anthropic/claude-sonnet-4.5. In the UI, /model switches it.
    #[arg(short = 'm', long, default_value = models::DEFAULT_MODEL)]
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let Some(prompt) = args.prompt else {
        return tui::run(&client, args.model).await;
    };

    if let Some(answer) = agent::run(&client, args.model, prompt).await? {
        ui::print_answer(&answer);
    }
    Ok(())
}
