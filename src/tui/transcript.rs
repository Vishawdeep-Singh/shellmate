//! Turns the chat entries into styled lines for the chat panel.

use ansi_to_tui::IntoText;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};

use super::app::Entry;
use super::color;
use crate::ui::markdown;
use crate::ui::theme::{PINK, PURPLE, RED, TEXT};
use crate::ui::trace::{self, Piece};

/// All entries as one block of text. `width` is needed to lay out Markdown tables and code.
pub fn render(entries: &[Entry], width: u16) -> Text<'static> {
    let mut lines = Vec::new();
    for entry in entries {
        match entry {
            Entry::User(prompt) => {
                lines.push(Line::default());
                lines.push(user_line(prompt));
            }
            Entry::Event(event) => lines.push(pieces_line(trace::pieces(event))),
            Entry::Answer(answer) => {
                lines.push(Line::default());
                lines.extend(markdown_lines(answer, width));
            }
            Entry::Error(message) => lines.push(error_line(message)),
            Entry::Notice(message) => lines.push(notice_line(message)),
        }
    }
    Text::from(lines)
}

fn user_line(prompt: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "❯ ",
            Style::new().fg(color(PINK)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            prompt.to_string(),
            Style::new().fg(color(TEXT)).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn error_line(message: &str) -> Line<'static> {
    let style = Style::new().fg(color(RED));
    Line::from(vec![
        Span::styled("✗ error ", style.add_modifier(Modifier::BOLD)),
        Span::styled(message.to_string(), style),
    ])
}

fn notice_line(message: &str) -> Line<'static> {
    Line::styled(
        format!("⚙ {message}"),
        Style::new()
            .fg(color(PURPLE))
            .add_modifier(Modifier::ITALIC),
    )
}

/// The same pieces `-p` mode prints, painted as ratatui spans.
fn pieces_line(pieces: Vec<Piece>) -> Line<'static> {
    let spans: Vec<Span> = pieces
        .into_iter()
        .map(|piece| {
            let mut style = Style::new().fg(color(piece.color));
            if piece.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if piece.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            Span::styled(piece.text, style)
        })
        .collect();
    Line::from(spans)
}

/// Renders the Markdown with the same skin as `-p` mode, then converts its colour codes
/// into ratatui lines. Falls back to the raw text if the conversion fails.
fn markdown_lines(answer: &str, width: u16) -> Vec<Line<'static>> {
    match markdown::to_ansi(answer, width).into_text() {
        Ok(text) => text.lines,
        Err(_) => answer
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect(),
    }
}
