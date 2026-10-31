//! The parts of the REPL around the conversation: the banner, the prompt and errors.

use std::io::{self, Write};

use termimad::crossterm::style::Stylize;
use termimad::rgb;

use super::paint::{for_stderr, for_stdout};
use super::theme::{GRADIENT_END, GRADIENT_START, MUTED, PINK, RED};

/// The welcome banner shown when the REPL starts.
pub fn banner() {
    println!();
    println!("  {}", gradient("✦ shellmate ✦"));
    println!(
        "  {}",
        for_stdout("type a prompt, /skill to run a skill, exit or Ctrl-D to quit".with(MUTED))
    );
    println!();
}

/// The input prompt. It flushes, so it shows up before the user starts typing.
pub fn prompt() -> io::Result<()> {
    print!("{} ", for_stdout("❯".with(PINK).bold()));
    io::stdout().flush()
}

/// An error that ended a turn. The REPL keeps running after it.
pub fn error(err: &anyhow::Error) {
    eprintln!(
        "{} {}",
        for_stderr("✗ error".with(RED).bold()),
        for_stderr(format!("{err:#}").with(RED)),
    );
}

/// Colours each character a little further from GRADIENT_START towards GRADIENT_END.
fn gradient(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let last = chars.len().saturating_sub(1).max(1) as f32;

    chars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let t = i as f32 / last;
            let color = rgb(
                blend(GRADIENT_START.0, GRADIENT_END.0, t),
                blend(GRADIENT_START.1, GRADIENT_END.1, t),
                blend(GRADIENT_START.2, GRADIENT_END.2, t),
            );
            for_stdout(c.with(color).bold())
        })
        .collect()
}

/// The value `t` of the way from `from` to `to`, where `t` goes from 0.0 to 1.0.
fn blend(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_hits_both_ends_and_the_middle() {
        assert_eq!(blend(0, 200, 0.0), 0);
        assert_eq!(blend(0, 200, 1.0), 200);
        assert_eq!(blend(0, 200, 0.5), 100);
        assert_eq!(blend(200, 0, 0.5), 100);
    }
}
