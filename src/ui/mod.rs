//! Everything shellmate shows in the terminal lives in this module.
//!
//! - `theme`    the colour palette, in one place
//! - `paint`    turns styled text into a string, or plain text when output is not a terminal
//! - `markdown` renders the model's final answer
//! - `trace`    live progress lines: tool calls, subagents, the model's notes
//! - `chrome`   the REPL banner, the prompt and error messages

mod chrome;
mod markdown;
mod paint;
mod theme;
pub mod trace;

pub use chrome::{banner, error, prompt};
pub use markdown::print_answer;
