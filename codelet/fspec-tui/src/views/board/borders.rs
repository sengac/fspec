//! RPC-014/015/016 — box-border painters for the unified Kanban grid,
//! split out of `board.rs` so that file stays under the 300-LoC
//! source-shape ceiling while keeping canonical rustfmt formatting.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub(super) fn paint_border_string(area: Rect, buf: &mut Buffer, body: &str, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    Paragraph::new(Line::from(Span::styled(body.to_string(), style))).render(area, buf);
}

pub(super) fn paint_side_borders(area: Rect, buf: &mut Buffer, style: Style) {
    if area.width < 2 || area.height == 0 {
        return;
    }
    for y in 0..area.height {
        buf.set_string(area.x, area.y + y, "│", style);
        buf.set_string(area.x + area.width - 1, area.y + y, "│", style);
    }
}

pub(super) fn inner_rect(area: Rect) -> Rect {
    if area.width < 2 {
        return area;
    }
    Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width - 2,
        height: area.height,
    }
}
