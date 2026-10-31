//! The full-screen interactive UI, built with ratatui.
//!
//! - `app`        the state, and how keys and agent events change it
//! - `dropdown`   what the `/` dropdown shows: commands, skills or models
//! - `view`       draws the screen from that state
//! - `transcript` turns chat entries into styled lines
//!
//! The agent runs in a background task, so the screen stays responsive while it works.
//! Four things wake the loop below: a key press, an event from the agent, a finished
//! turn, and a timer tick that animates the spinner. After each one, the screen is redrawn.

mod app;
mod dropdown;
mod transcript;
mod view;

use std::time::Duration;

use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use crossterm::event::{Event, EventStream};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use ratatui::prelude::FromCrossterm;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::agent::{AgentContext, agent_loop};
use crate::api::Message;
use crate::events::{AgentEvent, Reporter};
use crate::models::{self, ModelInfo};
use app::{Action, App};
use dropdown::SkillInfo;

/// A finished conversation turn: the updated history, and the answer or error.
struct Turn {
    messages: Vec<Message>,
    result: Result<Option<String>>,
}

pub async fn run(client: &Client<OpenAIConfig>, model: String) -> Result<()> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let ctx = AgentContext::new(client, model.clone(), Reporter::Channel(event_sender));
    let mut app = App::new(skill_infos(&ctx), model);
    let model_list = fetch_models_in_background(client);

    // Switches to the full screen. `restore` switches back, even after an error.
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, ctx, event_receiver, model_list).await;
    ratatui::restore();
    result
}

async fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    ctx: AgentContext,
    mut agent_events: UnboundedReceiver<AgentEvent>,
    mut model_list: UnboundedReceiver<Result<Vec<ModelInfo>>>,
) -> Result<()> {
    let (turn_sender, mut finished_turns) = mpsc::unbounded_channel();
    let mut messages = ctx.initial_messages();
    let mut keys = EventStream::new();
    let mut ticks = tokio::time::interval(Duration::from_millis(80));

    loop {
        terminal.draw(|frame| view::draw(frame, app))?;

        tokio::select! {
            Some(Ok(Event::Key(key))) = keys.next() => match app.on_key(key) {
                Action::Quit => return Ok(()),
                Action::Submit(prompt) => {
                    let history = std::mem::take(&mut messages);
                    let turn_ctx = AgentContext {
                        model: app.model.clone(),
                        ..ctx.clone()
                    };
                    start_turn(turn_ctx, history, prompt, turn_sender.clone());
                }
                Action::Nothing => {}
            },
            Some(event) = agent_events.recv() => app.on_agent_event(event),
            Some(turn) = finished_turns.recv() => {
                messages = turn.messages;
                app.on_turn_done(turn.result);
            }
            Some(result) = model_list.recv() => app.on_models_loaded(result),
            _ = ticks.tick() => app.tick(),
        }
    }
}

/// Runs one prompt through the agent in the background. The history is moved into
/// the task and comes back with the result, so only one turn can own it at a time.
fn start_turn(
    ctx: AgentContext,
    mut messages: Vec<Message>,
    prompt: String,
    done: UnboundedSender<Turn>,
) {
    tokio::spawn(async move {
        ctx.push_prompt(&mut messages, prompt);
        let result = agent_loop(&mut messages, &ctx).await;
        let _ = done.send(Turn { messages, result });
    });
}

/// Starts loading the model list, so `/model` is ready by the time the user types it.
fn fetch_models_in_background(
    client: &Client<OpenAIConfig>,
) -> UnboundedReceiver<Result<Vec<ModelInfo>>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let client = client.clone();
    tokio::spawn(async move {
        let _ = sender.send(models::fetch(&client).await);
    });
    receiver
}

fn skill_infos(ctx: &AgentContext) -> Vec<SkillInfo> {
    ctx.skills
        .iter()
        .filter_map(|skill| {
            Some(SkillInfo {
                name: skill.name()?.to_string(),
                description: skill.description().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// The palette is defined with crossterm colours. ratatui has its own colour type.
fn color(c: crossterm::style::Color) -> ratatui::style::Color {
    ratatui::style::Color::from_crossterm(c)
}
