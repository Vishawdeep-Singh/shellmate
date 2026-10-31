//! Renders the model's final answer, which is Markdown, with colours and styles.

use termimad::crossterm::style::Attribute;
use termimad::{MadSkin, ROUNDED_TABLE_BORDER_CHARS, StyledChar};

use super::paint;
use super::theme::{CYAN, GREEN, MUTED, ORANGE, PINK, PURPLE, SURFACE, TEXT, YELLOW};

/// Prints the answer as styled Markdown in a terminal, or as plain Markdown otherwise.
pub fn print_answer(answer: &str) {
    if paint::stdout_has_color() {
        println!();
        skin().print_text(answer);
    } else {
        println!("{answer}");
    }
}

/// How each piece of Markdown looks. Every line here styles one kind of element.
fn skin() -> MadSkin {
    let mut skin = MadSkin::default_dark();

    // # Heading 1, ## Heading 2, ### Heading 3
    skin.headers[0].set_fg(PINK);
    skin.headers[0].add_attr(Attribute::Bold);
    skin.headers[1].set_fg(PURPLE);
    skin.headers[1].add_attr(Attribute::Bold);
    skin.headers[2].set_fg(CYAN);

    // **bold**, *italic*, ~~strikeout~~
    skin.bold.set_fg(ORANGE);
    skin.italic.set_fg(YELLOW);
    skin.strikeout.set_fg(MUTED);

    // `inline code` and ``` code blocks ```
    skin.inline_code.set_fgbg(GREEN, SURFACE);
    skin.code_block.set_fgbg(TEXT, SURFACE);

    // - list items, > quotes, --- rules
    skin.bullet = StyledChar::from_fg_char(PINK, '•');
    skin.quote_mark = StyledChar::from_fg_char(PURPLE, '▌');
    skin.horizontal_rule = StyledChar::from_fg_char(MUTED, '─');

    // | tables |
    skin.table.set_fg(MUTED);
    skin.table_border_chars = ROUNDED_TABLE_BORDER_CHARS;

    skin
}
