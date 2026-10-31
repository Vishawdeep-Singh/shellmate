//! What the agent reports while it works, and where those reports go.
//!
//! The agent never prints directly. It hands an `AgentEvent` to its `Reporter`,
//! and the reporter decides how to show it: as a coloured line on stderr in `-p` mode,
//! or as a message to the full-screen UI.

use tokio::sync::mpsc::UnboundedSender;

use crate::ui::trace;

/// `depth` is 0 for the main agent and 1 for a subagent started by a forked skill.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// The user's slash commands were replaced by `count` skill bodies.
    SlashExpanded { count: usize },
    /// Text the model wrote next to its tool calls, like "Let me check the file first".
    ModelNote { depth: usize, text: String },
    ToolStarted {
        depth: usize,
        tool: String,
        detail: String,
    },
    ToolFinished {
        depth: usize,
        tool: String,
        output: String,
    },
    ToolFailed {
        depth: usize,
        tool: String,
        error: String,
    },
    /// A skill with `context: fork` started its own subagent.
    Forked { depth: usize },
}

#[derive(Clone)]
pub enum Reporter {
    /// Print each event to stderr as it happens.
    Stderr,
    /// Send each event to the full-screen UI.
    Channel(UnboundedSender<AgentEvent>),
}

impl Reporter {
    pub fn report(&self, event: AgentEvent) {
        match self {
            Reporter::Stderr => trace::print(&event),
            // The UI only goes away when the user quits, so a failed send can be ignored.
            Reporter::Channel(sender) => {
                let _ = sender.send(event);
            }
        }
    }
}
