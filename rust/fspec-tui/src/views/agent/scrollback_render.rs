//! ScrollbackList — the BUG-190 render pass (row reflow cache paint).
//!
//! Feature: spec/features/scrollback-row-reflow-cache.feature
//!
//! Extracted from `scrollback.rs` (300-LoC ceiling) AND re-wired: the
//! chunk-row paint now goes through the per-(chunk content, width)
//! `RowReflowCache`, so the ~60fps busy-state repaint blits cached
//! reflected rows instead of rebuilding a ratatui `Paragraph` (re-wrap
//! + grapheme scan + width measurement) per visible row per frame.
//!
//! Scrollbar / arrow-bar / selection-highlight painting stay per-frame
//! and unchanged (R5).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::ScrollbackList;

/// Render the visible window into `area`; returns chunks visited.
/// RPC-078 fills from the TOP; RPC-094 reserves a 2-col gutter +
/// scrollbar on overflow. **BUG-190**: chunk rows paint through the
/// `RowReflowCache` — a cache hit blits the stored cells, a miss
/// reflects the row once through the SAME ratatui path the old
/// uncached painter used (byte-identical, R2).
pub(crate) fn render_count_visited(
    list: &mut ScrollbackList,
    area: Rect,
    buf: &mut Buffer,
) -> usize {
    // Pass 1: wrap at full width to detect overflow.
    list.set_viewport_width(area.width);
    list.set_viewport_height(area.height);
    list.last_rect = Some(area);
    if area.width == 0 || area.height == 0 || list.chunks.is_empty() {
        return 0;
    }
    let vh = area.height as usize;
    // Pass 2: on overflow with width >= 4, reserve a 2-col gutter, rewrap.
    let reserve_gutter = list.total_visual_rows() > vh && area.width >= 4;
    let content_width = if reserve_gutter {
        area.width - 2
    } else {
        area.width
    };
    if reserve_gutter {
        list.set_viewport_width(content_width);
    }
    // COPY-006: cache the gutter-free width so highlight + copy clamp alike.
    list.content_width = content_width;
    let total_rows = list.total_visual_rows();
    let skip_rows = if list.scroll_state.stick_to_bottom {
        total_rows.saturating_sub(vh)
    } else {
        list.scroll_state.offset
    };
    let visited = crate::views::agent::scrollback_paint::paint_chunk_rows(
        area,
        buf,
        &list.chunks,
        content_width,
        skip_rows,
        &mut list.reflow_cache,
    );
    // RPC-381: in Item mode, frame the selected turn with ▼/▲ bars.
    list.paint_selection_overlay(area, buf, content_width, skip_rows);
    // COPY-005: overlay the live text-selection region (REVERSED).
    crate::views::agent::scrollback_paint::paint_selection_highlight(
        area,
        buf,
        &list.selection_highlight_spans,
        content_width,
    );
    if reserve_gutter && total_rows > vh {
        crate::views::agent::scrollback_paint::paint_scrollbar(
            area,
            buf,
            vh,
            total_rows,
            list.scroll_state,
        );
    }
    visited
}
