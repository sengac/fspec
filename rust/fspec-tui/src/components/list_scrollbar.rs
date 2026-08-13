//! RPC-352 — Shared proportional list-scrollbar painter.
//!
//! Feature: spec/features/provider-settings-scrollbar.feature
//!
//! Lifts the `/model` `render_scrollbar` math (model_selector/rows_render.rs)
//! into a single reusable helper so both the full-screen ModelSelector and
//! the ProviderSettings List view paint a byte-identical proportional
//! `■` thumb over a `│` track, both `Modifier::DIM`. The math is preserved
//! exactly:
//!   * `thumb_h  = ((visible * h) / total).max(1)`
//!   * `thumb_pos = (scroll_offset * h) / total`

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};

/// Draw a proportional scrollbar thumb `■` over the track `│` inside the
/// 1-cell-wide `area`. `visible` is the number of rows on screen, `total`
/// the total item count, and `scroll_offset` the index of the first visible
/// item. A zero-height area or empty list paints nothing.
pub fn render_list_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    visible: usize,
    total: usize,
) {
    let h = area.height as usize;
    if h == 0 || total == 0 {
        return;
    }
    let thumb_h = ((visible * h) / total).max(1);
    let thumb_pos = (scroll_offset * h) / total;
    for i in 0..h {
        let is_thumb = i >= thumb_pos && i < thumb_pos + thumb_h;
        let sym = if is_thumb { "■" } else { "│" };
        let row = Rect {
            x: area.x,
            y: area.y + i as u16,
            width: 1,
            height: 1,
        };
        Paragraph::new(Span::styled(
            sym,
            Style::default().add_modifier(Modifier::DIM),
        ))
        .render(row, buf);
    }
}
