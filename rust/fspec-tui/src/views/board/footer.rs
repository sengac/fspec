//! BoardView footer — centred hint rows.
//!
//! Feature: spec/features/rpc013-board-footer.feature
//! Card: RPC-013 (view-aware footer), extracted to its own module in
//!       RPC-016 so `views/board.rs` stays under the 300 LoC ceiling.
//!
//! Literal port of `src/tui/components/UnifiedBoardLayout.tsx:504-511`:
//! the TS source renders a single plain `<Text>` (no bold, no dim)
//! centred between the side borders.
//!
//! TUI-131 (spec/features/long-text-wraps-instead-of-being-cut-off.feature):
//! a hint wider than the board wraps onto more rows, each centred.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;
use crate::views::agent::text_wrap::wrap_to_width;

// MENU-002 R11: the ring is now columns → menu items → chips, so the hint
// describes the full walk (the old '← → Columns' token survives: 'Cycle
// Columns').
// TUI-129: Enter opens the chooser and Esc the Exit confirmation.
const COMPACT: &str = "← → Cycle Columns → menu items → chips ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Choose ◆ ESC Exit";
// BOARD-024: Enter now opens the card chooser (agent or full details); the
// wider hint names it whenever it fits on one row, else the compact hint.
const CHOOSER: &str = "← → Cycle Columns → menu items → chips ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Agent / Details (choose) ◆ ESC Exit";

/// TUI-131: the footer's rows for an inner `width`: the chooser hint when
/// it fits on one row, else the compact hint wrapped
/// (`text_wrap::wrap_to_width`) onto as many rows as it needs.
fn rows(width: u16) -> Vec<String> {
    let text = if (CHOOSER.width() as u16) < width {
        CHOOSER
    } else {
        COMPACT
    };
    wrap_to_width(text, usize::from(width))
}

/// TUI-131: the rows the footer takes inside an inner `width` (the board
/// sizes its footer band with this and gives the content the rest).
pub(crate) fn height(width: u16) -> u16 {
    u16::try_from(rows(width).len()).unwrap_or(u16::MAX)
}

/// Paint the Board footer inside the supplied inner rectangle (already
/// stripped of the side borders), each row centred.
pub(crate) fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let style = Style::default().fg(theme.fg);
    for (offset, text) in (0..area.height).zip(rows(area.width)) {
        let pad = area.width.saturating_sub(text.width() as u16) / 2;
        let row = Rect {
            x: area.x + pad,
            y: area.y + offset,
            width: area.width - pad,
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(text, style))).render(row, buf);
    }
}
