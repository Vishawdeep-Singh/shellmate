//! Draws the whole screen from the `App` state:
//!
//! ```text
//! ╭ ✦ shellmate ✦ ─────────────────── model ╮
//! │ chat panel (scrolls)                    │
//! ╰─────────────────────────────────────────╯
//!   ╭ skills ───────────────╮   ← dropdown, only while typing /name
//!   │ ▸ /loc  Counts lines… │
//!   ╰───────────────────────╯
//! ╭─────────────────────────────────────────╮
//! │ ❯ input                                 │
//! ╰─────────────────────────────────────────╯
//!  ⠋ thinking… 3s          Enter send · Tab …
//! ```

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::app::{App, Status};
use super::dropdown::Pick;
use super::{color, transcript};
use crate::ui::theme::{GREEN, MUTED, PINK, SURFACE, TEXT, YELLOW, gradient};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const MAX_DROPDOWN_ROWS: usize = 8;
/// Model ids can be long. Past this, the label is padded no further so the detail still shows.
const MAX_LABEL_WIDTH: usize = 44;

pub fn draw(frame: &mut Frame, app: &App) {
    let [chat, input, status] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_chat(frame, app, chat);
    draw_input(frame, app, input);
    draw_status(frame, app, status);
    // Last, so it floats on top of the chat panel.
    draw_dropdown(frame, app, input);
}

fn rounded_block(border: Style) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
}

fn draw_chat(frame: &mut Frame, app: &App, area: Rect) {
    let block = rounded_block(Style::new().fg(color(MUTED)))
        .title(gradient_title(" ✦ shellmate ✦ "))
        .title(
            Line::styled(format!(" {} ", app.model), Style::new().fg(color(MUTED))).right_aligned(),
        );

    if app.entries.is_empty() {
        frame.render_widget(welcome().block(block), area);
        return;
    }

    let inner = block.inner(area);
    let chat =
        Paragraph::new(transcript::render(&app.entries, inner.width)).wrap(Wrap { trim: false });

    // Show the bottom of the chat, minus however far the user scrolled up.
    let total_lines = chat.line_count(inner.width) as u16;
    let bottom = total_lines.saturating_sub(inner.height);
    let top = bottom.saturating_sub(app.scroll_up);

    frame.render_widget(chat.block(block).scroll((top, 0)), area);
}

fn welcome() -> Paragraph<'static> {
    let hint = |text: &'static str| Line::styled(text, Style::new().fg(color(MUTED)));
    Paragraph::new(vec![
        Line::default(),
        gradient_title("✦ shellmate ✦").centered(),
        Line::default(),
        hint("Ask anything about this project.").centered(),
        hint("Type / to pick a skill.").centered(),
    ])
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let border = if app.is_busy() { MUTED } else { PINK };
    let prompt = Span::styled(
        "❯ ",
        Style::new().fg(color(PINK)).add_modifier(Modifier::BOLD),
    );
    let typed = Span::styled(app.input.as_str(), Style::new().fg(color(TEXT)));
    let cursor_x = area.x + 1 + prompt.width() as u16 + typed.width() as u16;

    let line = Line::from(vec![prompt, typed]);
    frame.render_widget(
        Paragraph::new(line).block(rounded_block(Style::new().fg(color(border)))),
        area,
    );
    frame.set_cursor_position((cursor_x.min(area.right().saturating_sub(2)), area.y + 1));
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let state = match &app.status {
        Status::Ready => Line::styled(" ● ready", Style::new().fg(color(GREEN))),
        Status::Thinking { since } => Line::styled(
            format!(
                " {} thinking… {}s",
                SPINNER[app.frame % SPINNER.len()],
                since.elapsed().as_secs()
            ),
            Style::new().fg(color(YELLOW)),
        ),
    };
    let keys = Line::styled(
        "Enter send · Tab complete · ↑↓ scroll · Ctrl-C quit ",
        Style::new().fg(color(MUTED)),
    )
    .alignment(Alignment::Right);

    frame.render_widget(Paragraph::new(state), area);
    frame.render_widget(Paragraph::new(keys), area);
}

/// The `/` dropdown (commands, skills or models), drawn just above the input box.
fn draw_dropdown(frame: &mut Frame, app: &App, input: Rect) {
    let Some(dropdown) = app.dropdown() else {
        return;
    };

    let label_width = dropdown
        .rows
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0)
        .min(MAX_LABEL_WIDTH);
    let items: Vec<ListItem> = dropdown
        .rows
        .iter()
        .map(|row| {
            let label_style = match row.pick {
                Pick::Nothing => Style::new().fg(color(MUTED)).add_modifier(Modifier::ITALIC),
                _ => Style::new().fg(color(PINK)).add_modifier(Modifier::BOLD),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<label_width$}  ", row.label), label_style),
                Span::styled(row.detail.clone(), Style::new().fg(color(MUTED))),
            ]))
        })
        .collect();

    let height = dropdown.rows.len().min(MAX_DROPDOWN_ROWS) as u16 + 2;
    let area = Rect {
        x: input.x + 2,
        y: input.y.saturating_sub(height),
        width: input.width.saturating_sub(4).min(110),
        height,
    };

    let list = List::new(items)
        .block(
            rounded_block(Style::new().fg(color(PINK)))
                .title(Line::styled(dropdown.title, Style::new().fg(color(PINK)))),
        )
        .highlight_style(Style::new().bg(color(SURFACE)))
        .highlight_symbol("▸ ");
    let selected = app.selected.min(dropdown.rows.len() - 1);
    let mut state = ListState::default().with_selected(Some(selected));

    frame.render_widget(Clear, area);
    frame.render_stateful_widget(list, area, &mut state);
}

/// Each character gets its own colour, fading from pink to cyan.
fn gradient_title(text: &str) -> Line<'static> {
    let colors = gradient(text.chars().count());
    let spans: Vec<Span> = text
        .chars()
        .zip(colors)
        .map(|(c, tint)| {
            Span::styled(
                c.to_string(),
                Style::new().fg(color(tint)).add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Line::from(spans)
}
