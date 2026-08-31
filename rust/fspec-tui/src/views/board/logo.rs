//! 4-row FSPEC ASCII art logo widget — Rust port of the TS sub-widget
//! `src/tui/components/Logo.tsx`, extended by BOARD-021 with the
//! compile-time build version on the 4th row.
//!
//! Feature: spec/features/display-fspec-version-under-the-board-logo.feature
//! Card: BOARD-021 (version row); original logo: RPC-015.
//!
//! Paints the 4-row block:
//!   row 0: `┏┓┏┓┏┓┏┓┏┓ `
//!   row 1: `┣ ┗┓┃┃┣ ┃ `
//!   row 2: `┻ ┗┛┣┛┗┛┗┛ `
//!   row 3: `v{CARGO_PKG_VERSION}` (centered + dim-styled, BOARD-021)
//!
//! Width: 12 cells (matches `<Box width={12}>` on the TS side).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::Theme;

/// Fixed width of the logo block, in terminal cells.
pub const LOGO_WIDTH: u16 = 12;

/// 3-row glyph block — sourced character-for-character from
/// `src/tui/components/Logo.tsx`.
pub const LOGO_GLYPH_ROWS: [&str; 3] = ["┏┓┏┓┏┓┏┓┏┓ ", "┣ ┗┓┃┃┣ ┃ ", "┻ ┗┛┣┛┗┛┗┛ "];

/// The build version painted on the 4th logo row (BOARD-021): the
/// compile-time workspace version prefixed with `v` (e.g. `v0.10.5`).
pub const VERSION_LINE: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Paint the 4-row FSPEC logo into the supplied area.
///
/// Rows 0-2 carry the glyph block (default style); row 3 carries the
/// build version in the theme's dim color (BOARD-021). `area.height`
/// must be at least 4. The renderer is a no-op when the area is too
/// narrow (`area.width < 1`).
pub fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let max_rows = (area.height as usize).min(LOGO_GLYPH_ROWS.len() + 1);
    for (i, text) in LOGO_GLYPH_ROWS.iter().take(max_rows).enumerate() {
        paint_row(area, buf, i as u16, text, Style::default());
    }
    // BOARD-021: the 4th row carries the build version, centered within
    // the 12-cell logo block (mirroring the centered glyph rows) and
    // dim-styled so it reads as quiet branding, not a UI control.
    if max_rows > LOGO_GLYPH_ROWS.len() {
        paint_row(
            area,
            buf,
            LOGO_GLYPH_ROWS.len() as u16,
            &center_in_block(VERSION_LINE, LOGO_WIDTH),
            Style::default().fg(theme.dim),
        );
    }
}

/// Paint one 1-cell-tall row of the logo block at `row_index`.
fn paint_row(area: Rect, buf: &mut Buffer, row_index: u16, text: &str, style: Style) {
    let row_area = Rect {
        x: area.x,
        y: area.y + row_index,
        width: area.width.min(LOGO_WIDTH),
        height: 1,
    };
    Paragraph::new(Line::from(Span::styled(text.to_string(), style))).render(row_area, buf);
}

/// Center `text` in a `width`-cell block (ASCII; the version string is
/// pure ASCII so `len()` equals the cell count). Returns `text`
/// unchanged when it is wider than the block — the Paragraph render
/// clips at the block edge, so overflow never reaches the right-hand
/// header column.
fn center_in_block(text: &str, width: u16) -> String {
    let text_len = text.len() as u16;
    if text_len >= width {
        return text.to_string();
    }
    let pad = (width - text_len) / 2;
    format!("{}{}", " ".repeat(pad as usize), text)
}
