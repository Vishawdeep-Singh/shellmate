//! The colour palette. Change a colour here and it changes everywhere,
//! in `-p` mode and in the full-screen UI.
//!
//! The colours are bright on purpose, so they read well on a dark terminal background.

use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub const PINK: Color = rgb(255, 121, 198);
pub const PURPLE: Color = rgb(189, 147, 249);
pub const CYAN: Color = rgb(139, 233, 253);
pub const GREEN: Color = rgb(80, 250, 123);
pub const ORANGE: Color = rgb(255, 184, 108);
pub const YELLOW: Color = rgb(241, 250, 140);
pub const RED: Color = rgb(255, 85, 85);

/// Plain body text.
pub const TEXT: Color = rgb(248, 248, 242);
/// Quiet text: hints, connectors, borders.
pub const MUTED: Color = rgb(108, 124, 174);
/// Background behind code and the selected dropdown row.
pub const SURFACE: Color = rgb(40, 42, 54);

/// The two ends of the title's colour gradient, as (red, green, blue).
const GRADIENT_START: (u8, u8, u8) = (255, 121, 198);
const GRADIENT_END: (u8, u8, u8) = (139, 233, 253);

/// Each tool gets its own colour, so you can spot it at a glance.
pub fn tool_color(tool: &str) -> Color {
    match tool {
        "Read" => CYAN,
        "Write" => ORANGE,
        "Bash" => GREEN,
        "Skill" => PINK,
        _ => PURPLE,
    }
}

/// `count` colours fading from GRADIENT_START to GRADIENT_END, one per character of a title.
pub fn gradient(count: usize) -> Vec<Color> {
    let last = count.saturating_sub(1).max(1) as f32;
    (0..count)
        .map(|i| {
            let t = i as f32 / last;
            rgb(
                blend(GRADIENT_START.0, GRADIENT_END.0, t),
                blend(GRADIENT_START.1, GRADIENT_END.1, t),
                blend(GRADIENT_START.2, GRADIENT_END.2, t),
            )
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

    #[test]
    fn gradient_starts_and_ends_on_the_two_colours() {
        let colors = gradient(5);

        assert_eq!(colors.len(), 5);
        assert_eq!(colors[0], PINK);
        assert_eq!(colors[4], CYAN);
    }
}
