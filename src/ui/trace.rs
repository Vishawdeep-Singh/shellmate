//! How each `AgentEvent` looks, as a line of coloured pieces:
//!
//! ```text
//! ⚡ slash command expanded inline into 1 skill prompt
//! ✦ I'll count the lines first.
//! ⏺ Skill loc src rs 3
//!   ↳ forked subagent
//!     ⏺ Bash bash .claude/skills/loc/scripts/loc.sh "src" "rs" "3"
//!       ⎿ ✓ Bash  .rs files under src: 5 files, 1203 lines (+4 more lines)
//!   ⎿ ✓ Skill  Skill loc ran in a separate context and returned: ...
//! ```
//!
//! `pieces` decides *what* the line says once. `print` paints it for stderr,
//! and the full-screen UI paints the same pieces with ratatui.

use crossterm::style::{Color, Stylize};

use super::paint;
use super::theme::{MUTED, PURPLE, RED, TEXT, YELLOW, tool_color};
use crate::events::AgentEvent;

/// A run of text in one style.
pub struct Piece {
    pub text: String,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
}

impl Piece {
    fn new(text: impl Into<String>, color: Color) -> Self {
        Piece {
            text: text.into(),
            color,
            bold: false,
            italic: false,
        }
    }

    fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// Prints the event as one coloured line on stderr.
pub fn print(event: &AgentEvent) {
    let line: String = pieces(event).iter().map(paint_piece).collect();
    eprintln!("{line}");
}

fn paint_piece(piece: &Piece) -> String {
    let mut styled = piece.text.as_str().with(piece.color);
    if piece.bold {
        styled = styled.bold();
    }
    if piece.italic {
        styled = styled.italic();
    }
    paint::for_stderr(styled)
}

/// The line for one event, split into styled pieces.
pub fn pieces(event: &AgentEvent) -> Vec<Piece> {
    match event {
        AgentEvent::SlashExpanded { count } => {
            let plural = if *count == 1 { "" } else { "s" };
            vec![Piece::new(
                format!("⚡ slash command expanded inline into {count} skill prompt{plural}"),
                YELLOW,
            )]
        }
        AgentEvent::ModelNote { depth, text } => vec![
            indent(*depth),
            Piece::new("✦ ", YELLOW),
            Piece::new(join_lines(text), MUTED).italic(),
        ],
        AgentEvent::ToolStarted {
            depth,
            tool,
            detail,
        } => {
            let color = tool_color(tool);
            vec![
                indent(*depth),
                Piece::new("⏺ ", color),
                Piece::new(tool, color).bold(),
                Piece::new(format!(" {}", one_line(detail)), TEXT),
            ]
        }
        AgentEvent::ToolFinished {
            depth,
            tool,
            output,
        } => vec![
            indent(*depth),
            Piece::new("  ⎿ ", MUTED),
            Piece::new("✓ ", tool_color(tool)).bold(),
            Piece::new(format!("{tool}  {}", one_line(output)), MUTED),
        ],
        AgentEvent::ToolFailed { depth, tool, error } => vec![
            indent(*depth),
            Piece::new("  ⎿ ", MUTED),
            Piece::new("✗ ", RED).bold(),
            Piece::new(format!("{tool}  {}", one_line(error)), RED),
        ],
        AgentEvent::Forked { depth } => vec![
            indent(*depth),
            Piece::new("  ↳ forked subagent", PURPLE).italic(),
        ],
    }
}

fn indent(depth: usize) -> Piece {
    Piece::new("    ".repeat(depth), TEXT)
}

/// Puts a multi-line note on one line.
fn join_lines(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first useful line of `text`, cut to fit on one line, plus how many lines were left out.
/// Blank lines and Markdown code fences (```) are skipped, since they say nothing on their own.
fn one_line(text: &str) -> String {
    const MAX_CHARS: usize = 100;

    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("```"));

    let Some(first) = lines.next() else {
        return "(no output)".to_string();
    };

    let mut line: String = first.chars().take(MAX_CHARS).collect();
    if first.chars().count() > MAX_CHARS {
        line.push('…');
    }
    let hidden = lines.count();
    if hidden > 0 {
        line.push_str(&format!(" (+{hidden} more lines)"));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(event: AgentEvent) -> String {
        pieces(&event).into_iter().map(|piece| piece.text).collect()
    }

    #[test]
    fn tool_started_line() {
        let event = AgentEvent::ToolStarted {
            depth: 1,
            tool: "Bash".to_string(),
            detail: "ls -la".to_string(),
        };

        assert_eq!(text_of(event), "    ⏺ Bash ls -la");
    }

    #[test]
    fn tool_finished_line_shows_a_preview() {
        let event = AgentEvent::ToolFinished {
            depth: 0,
            tool: "Read".to_string(),
            output: "line one\nline two\n".to_string(),
        };

        assert_eq!(text_of(event), "  ⎿ ✓ Read  line one (+1 more lines)");
    }

    #[test]
    fn model_note_is_joined_onto_one_line() {
        let event = AgentEvent::ModelNote {
            depth: 0,
            text: "Let me check.\n\nThen count.".to_string(),
        };

        assert_eq!(text_of(event), "✦ Let me check. Then count.");
    }

    #[test]
    fn one_line_keeps_short_text() {
        assert_eq!(one_line("hello\n"), "hello");
    }

    #[test]
    fn one_line_counts_hidden_lines() {
        assert_eq!(one_line("a\nb\nc"), "a (+2 more lines)");
    }

    #[test]
    fn one_line_cuts_long_lines() {
        let long = "x".repeat(150);

        assert_eq!(one_line(&long), format!("{}…", "x".repeat(100)));
    }

    #[test]
    fn one_line_marks_empty_output() {
        assert_eq!(one_line("  \n"), "(no output)");
    }

    #[test]
    fn one_line_skips_code_fences_and_blank_lines() {
        assert_eq!(one_line("```\n\nfn main() {}\n```"), "fn main() {}");
    }
}
