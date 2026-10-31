//! The colour palette. Change a colour here and it changes everywhere.
//!
//! The colours are bright on purpose, so they read well on a dark terminal background.

use termimad::crossterm::style::Color;
use termimad::rgb;

pub const PINK: Color = rgb(255, 121, 198);
pub const PURPLE: Color = rgb(189, 147, 249);
pub const CYAN: Color = rgb(139, 233, 253);
pub const GREEN: Color = rgb(80, 250, 123);
pub const ORANGE: Color = rgb(255, 184, 108);
pub const YELLOW: Color = rgb(241, 250, 140);
pub const RED: Color = rgb(255, 85, 85);

/// Plain body text.
pub const TEXT: Color = rgb(248, 248, 242);
/// Quiet text: hints, connectors, table borders.
pub const MUTED: Color = rgb(108, 124, 174);
/// Background behind code.
pub const SURFACE: Color = rgb(40, 42, 54);

/// The two ends of the banner's colour gradient, as (red, green, blue).
pub const GRADIENT_START: (u8, u8, u8) = (255, 121, 198);
pub const GRADIENT_END: (u8, u8, u8) = (139, 233, 253);

/// Each tool gets its own colour, so you can spot it in the trace at a glance.
pub fn tool_color(tool: &str) -> Color {
    match tool {
        "Read" => CYAN,
        "Write" => ORANGE,
        "Bash" => GREEN,
        "Skill" => PINK,
        _ => PURPLE,
    }
}
