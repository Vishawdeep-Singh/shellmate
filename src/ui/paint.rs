//! Colours only make sense in a terminal. When output goes to a file or a pipe
//! (`shellmate -p "..." > out.txt`), colour codes would show up as junk like `\x1b[1m`.
//! These helpers keep the colours for a terminal and drop them everywhere else.
//! Setting the `NO_COLOR` environment variable turns colours off too.

use std::fmt::Display;
use std::io::{self, IsTerminal};
use std::sync::LazyLock;

use crossterm::style::StyledContent;

static STDOUT_HAS_COLOR: LazyLock<bool> = LazyLock::new(|| wants_color(io::stdout().is_terminal()));
static STDERR_HAS_COLOR: LazyLock<bool> = LazyLock::new(|| wants_color(io::stderr().is_terminal()));

fn wants_color(is_terminal: bool) -> bool {
    is_terminal && std::env::var_os("NO_COLOR").is_none()
}

pub fn stdout_has_color() -> bool {
    *STDOUT_HAS_COLOR
}

/// Styled text for stderr: coloured in a terminal, plain otherwise.
pub fn for_stderr<D: Display>(styled: StyledContent<D>) -> String {
    render(styled, *STDERR_HAS_COLOR)
}

fn render<D: Display>(styled: StyledContent<D>, with_color: bool) -> String {
    if with_color {
        styled.to_string()
    } else {
        styled.content().to_string()
    }
}
