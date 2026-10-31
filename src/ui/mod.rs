//! How shellmate looks, shared by `-p` mode and the full-screen UI.
//!
//! - `theme`    the colour palette, in one place
//! - `paint`    turns styled text into a string, or plain text when output is not a terminal
//! - `markdown` renders the model's final answer
//! - `trace`    how each agent event looks: tool calls, subagents, the model's notes

pub mod markdown;
mod paint;
pub mod theme;
pub mod trace;

pub use markdown::print_answer;
