//! BoardView footer — single centred hint line.
//!
//! Feature: spec/features/rpc013-board-footer.feature
//! Card: RPC-013 (view-aware footer), extracted to its own module in
//!       RPC-016 so `views/board.rs` stays under the 300 LoC ceiling.
//!
//! Literal port of `src/tui/components/UnifiedBoardLayout.tsx:504-511`:
//! the TS source renders a single plain `<Text>` (no bold, no dim)
//! centred between the side borders.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::theme::Theme;

/// Paint the 1-row Board footer string inside the supplied inner
/// rectangle (already stripped of the side borders).
pub(crate) fn render(area: Rect, buf: &mut Buffer, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let style = Style::default().fg(theme.fg);
    let text =
        "← → Columns ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Work Agent ◆ ESC Back";
    let text_len = text.chars().count() as u16;
    let inner = if text_len < area.width {
        let pad = (area.width - text_len) / 2;
        Rect {
            x: area.x + pad,
            y: area.y,
            width: area.width - pad,
            height: 1,
        }
    } else {
        area
    };
    Paragraph::new(Line::from(Span::styled(text.to_string(), style))).render(inner, buf);
}
