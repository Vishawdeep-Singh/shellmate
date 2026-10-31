//! Live progress lines, printed to stderr while the agent works:
//!
//! ```text
//! ⚡ /loc expanded inline into 1 skill prompt
//! ✦ I'll count the lines first.
//! ⏺ Skill loc src rs 3
//!   ↳ forked subagent
//!     ⏺ Bash bash .claude/skills/loc/scripts/loc.sh "src" "rs" "3"
//!       ⎿ ✓ Bash  .rs files under src: 5 files, 1203 lines (+4 more lines)
//!   ⎿ ✓ Skill  Skill loc ran in a separate context and returned: ...
//! ```
//!
//! `depth` is 0 for the main agent and 1 for a subagent, and sets the indent.

use termimad::crossterm::style::Stylize;

use super::paint::for_stderr as paint;
use super::theme::{MUTED, PURPLE, RED, TEXT, YELLOW, tool_color};

/// A tool call is about to run.
pub fn tool_start(depth: usize, tool: &str, detail: &str) {
    let color = tool_color(tool);
    eprintln!(
        "{}{} {} {}",
        indent(depth),
        paint("⏺".with(color)),
        paint(tool.with(color).bold()),
        paint(one_line(detail).with(TEXT)),
    );
}

/// A tool call finished; `output` is what the model gets back.
pub fn tool_done(depth: usize, tool: &str, output: &str) {
    eprintln!(
        "{}  {} {} {}  {}",
        indent(depth),
        paint("⎿".with(MUTED)),
        paint("✓".with(tool_color(tool)).bold()),
        paint(tool.with(MUTED)),
        paint(one_line(output).with(MUTED)),
    );
}

/// A tool call failed; the error text is sent back to the model.
pub fn tool_failed(depth: usize, tool: &str, error: &str) {
    eprintln!(
        "{}  {} {} {}  {}",
        indent(depth),
        paint("⎿".with(MUTED)),
        paint("✗".with(RED).bold()),
        paint(tool.with(RED)),
        paint(one_line(error).with(RED)),
    );
}

/// A skill with `context: fork` is starting its own subagent.
pub fn fork_started(depth: usize) {
    eprintln!(
        "{}  {}",
        indent(depth),
        paint("↳ forked subagent".with(PURPLE).italic()),
    );
}

/// Text the model wrote next to its tool calls, like "Let me check the file first".
pub fn model_note(depth: usize, text: &str) {
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        eprintln!(
            "{}{} {}",
            indent(depth),
            paint("✦".with(YELLOW)),
            paint(line.trim().with(MUTED).italic()),
        );
    }
}

/// The user typed slash commands, and they were replaced by the skill bodies.
pub fn slash_expanded(count: usize) {
    let plural = if count == 1 { "" } else { "s" };
    eprintln!(
        "{} {}",
        paint("⚡".with(YELLOW)),
        paint(
            format!("slash command expanded inline into {count} skill prompt{plural}").with(YELLOW)
        ),
    );
}

fn indent(depth: usize) -> String {
    "    ".repeat(depth)
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
