//! 4-row FSPEC ASCII art logo widget — Rust port of the TS sub-widget
//! `src/tui/components/Logo.tsx`.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015.
//!
//! Paints the 4-row block:
//!   row 0: `┏┓┏┓┏┓┏┓┏┓ `
//!   row 1: `┣ ┗┓┃┃┣ ┃ `
//!   row 2: `┻ ┗┛┣┛┗┛┗┛ `
//!   row 3: ` `
//!
//! Width: 12 cells (matches `<Box width={12}>` on the TS side).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Fixed width of the logo block, in terminal cells.
pub const LOGO_WIDTH: u16 = 12;

/// 4-row glyph block — sourced character-for-character from
/// `src/tui/components/Logo.tsx`.
pub const LOGO_ROWS: [&str; 4] = [
    "┏┓┏┓┏┓┏┓┏┓ ",
    "┣ ┗┓┃┃┣ ┃ ",
    "┻ ┗┛┣┛┗┛┗┛ ",
    " ",
];

/// Paint the 4-row FSPEC logo into the supplied area.
///
/// `area.height` must be at least 4. The renderer is a no-op when the
/// area is too narrow for the leftmost row (`area.width < 1`).
pub fn render(area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let max_rows = (area.height as usize).min(LOGO_ROWS.len());
    for (i, glyphs) in LOGO_ROWS.iter().take(max_rows).enumerate() {
        let row_area = Rect {
            x: area.x,
            y: area.y + i as u16,
            width: area.width.min(LOGO_WIDTH),
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(glyphs.to_string(), Style::default())))
            .render(row_area, buf);
    }
}
