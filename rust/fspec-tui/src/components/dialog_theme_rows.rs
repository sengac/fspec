//! RPC-027 — Helper row builders for the canonical dialog theme.
//!
//! Feature: spec/features/rpc027-dialog-theme.feature
//!
//! Extracted from `dialog_theme.rs` to keep both files under the
//! 300-LoC ceiling required by RPC-027 rule [11].

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::dialog_theme::{DialogRow, FspecDialog, MARKER_SELECTED, MARKER_UNSELECTED};
use crate::components::dialog_theme::Accent;
use crate::views::agent::text_wrap::wrap_to_width;

/// Display width of a single line (shared by the dialog theme's width
/// math and the footer painter).
pub fn line_width(line: &str) -> u16 {
    line.width() as u16
}

/// RPC-383: the resolved scroll geometry of the full-screen
/// `TurnContentModal` for a given `(area, body)`. The render path and
/// the App reducer MUST agree on these three numbers or the offset
/// clamp desyncs from what is painted, so both call this one helper.
pub struct TurnModalGeometry {
    /// Total wrapped visual rows of the body at `content_width`.
    pub total_rows: usize,
    /// Body viewport height (rows the dialog renderer actually paints).
    pub viewport_rows: usize,
    /// Body content width after the overflow scrollbar gutter is
    /// subtracted (1 col narrower than the full inner width when the
    /// body overflows the viewport).
    pub content_width: usize,
}

/// RPC-383: count the wrapped visual rows of `text` at `width`,
/// preserving hard breaks and counting empty paragraphs as one row.
/// Single source of truth shared by [`turn_modal_geometry`] (and hence
/// both `TurnContentModal::render` and the App scroll reducer).
pub fn wrap_row_count(text: &str, width: usize) -> usize {
    text.split('\n')
        .map(|hard| wrap_to_width(hard, width).len().max(1))
        .sum()
}

/// RPC-383: compute the `(total_rows, viewport_rows, content_width)`
/// scroll geometry of the full-screen `TurnContentModal` for `body`
/// inside `area`. This is the ONE place the fixed-rect → inner-width →
/// viewport-rows → overflow-narrowed-width pipeline lives; both the
/// render path and the reducer call it so they stay in lockstep.
pub fn turn_modal_geometry(area: Rect, body: &str) -> TurnModalGeometry {
    let rect = fixed_dialog_rect(area);
    let full_inner = rect.width.saturating_sub(4).max(1) as usize;
    let viewport_rows = body_content_rows(rect.height, 1, false).max(1);
    let probe = wrap_row_count(body, full_inner);
    let content_width = if probe > viewport_rows {
        full_inner.saturating_sub(1).max(1)
    } else {
        full_inner
    };
    let total_rows = if content_width == full_inner {
        probe
    } else {
        wrap_row_count(body, content_width)
    };
    TurnModalGeometry {
        total_rows,
        viewport_rows,
        content_width,
    }
}

/// RPC-383: build a [`FspecDialog`] without writing the raw struct
/// literal at any call site. The full-screen `TurnContentModal` paints
/// via `render_dialog_at` (not `render_dialog`), so it builds its
/// descriptor through this helper. Mutates a `Default` instance in place
/// so the forbidden raw `FspecDialog` struct literal never appears (the
/// `field_reassign_with_default` lint is intentionally allowed here).
#[allow(clippy::field_reassign_with_default)]
pub fn build_dialog<'a>(
    accent: Accent,
    title: &'a str,
    rows: Vec<DialogRow>,
    footer: &'a str,
    min_width: u16,
) -> FspecDialog<'a> {
    let mut d = FspecDialog::default();
    d.accent = accent;
    d.title = title;
    d.rows = rows;
    d.footer = footer;
    d.min_width = min_width;
    d
}

/// Convenience builder for a "marker + label - description" row used
/// by `ThinkingLevelDialog` and `ModelSelectorDialog`. The description
/// is dimmed when not selected (matches `dimColor={!isSelected}` in
/// `ThinkingLevelDialog.tsx`).
pub fn label_description_row(label: &str, description: &str, selected: bool) -> DialogRow {
    label_description_default_row(label, description, selected, false)
}

/// TUI-094: `label_description_row` plus an optional ` (default)` marker
/// appended onto the (dimmable) description span when `is_default` is
/// true. Mirrors `ThinkingLevelDialog.tsx` lines 140-144 where the
/// `(default)` text rides on the `dimColor={!isSelected}` description so
/// it dims with the row. `is_default = false` is byte-identical to the
/// pre-TUI-094 `label_description_row` output (the existing
/// `ModelSelectorDialog` path is unaffected).
pub fn label_description_default_row(
    label: &str,
    description: &str,
    selected: bool,
    is_default: bool,
) -> DialogRow {
    let marker = if selected {
        MARKER_SELECTED
    } else {
        MARKER_UNSELECTED
    };
    let mut spans: Vec<Span<'static>> = vec![Span::raw(marker.to_string())];
    spans.push(Span::raw(label.to_string()));
    if !description.is_empty() {
        spans.push(Span::raw(" - ".to_string()));
        let desc_style = if selected {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        let desc_text = if is_default {
            format!("{description} (default)")
        } else {
            description.to_string()
        };
        spans.push(Span::styled(desc_text, desc_style));
    }
    DialogRow {
        spans,
        selectable: true,
        selected,
    }
}

/// Paint a left-aligned sequence of spans into `rect`. Cells before
/// the spans are left as already painted (caller is responsible for
/// background). Cells after are painted with `tail_style` as spaces.
///
/// Extracted from `dialog_theme.rs` (RPC-383) so that file stays under
/// the 300-LoC ceiling after gaining the full-screen render path.
pub(super) fn paint_left_aligned(
    buf: &mut Buffer,
    rect: Rect,
    spans: &[Span<'_>],
    tail_style: Style,
) {
    let mut x = rect.x;
    for span in spans {
        for ch in span.content.chars() {
            if x >= rect.x + rect.width {
                return;
            }
            buf[(x, rect.y)].set_style(span.style);
            buf[(x, rect.y)].set_symbol(&ch.to_string());
            x += 1;
        }
    }
    while x < rect.x + rect.width {
        buf[(x, rect.y)].set_style(tail_style);
        buf[(x, rect.y)].set_symbol(" ");
        x += 1;
    }
}

/// Paint a plain text string at (x, y) with the given style, capped at
/// `max_width` cells. Extracted alongside [`paint_left_aligned`].
pub(super) fn paint_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_width: u16,
    text: &str,
    style: Style,
) {
    let end = x + max_width;
    for (cx, ch) in (x..end).zip(text.chars()) {
        buf[(cx, y)].set_style(style);
        buf[(cx, y)].set_symbol(&ch.to_string());
    }
}

/// Compute the FIXED full-screen dialog rect for RPC-383's
/// `TurnContentModal`: `area.width - 4` × `area.height - 6`, centered,
/// independent of content length. Kept separate from
/// `dialog_theme::dialog_rect` so every other dialog keeps its
/// shrink-to-content default untouched.
pub fn fixed_dialog_rect(area: Rect) -> Rect {
    let width = area.width.saturating_sub(4);
    let height = area.height.saturating_sub(6);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// RPC-383: number of content rows that fit inside a dialog of total
/// `rect_height` whose footer occupies `footer_h` lines, using the SAME
/// spacious/compact fallback as `dialog_theme::render_dialog_at`. The
/// full-screen `TurnContentModal` calls this to size its scroll viewport
/// so the reducer clamps offset against exactly what is painted.
/// BUG-159: `has_query_row` reserves one extra content row for the
/// pinned query row painted by `render_dialog_at` — returns one less
/// when true, in both the spacious and the compact fallback.
pub fn body_content_rows(rect_height: u16, footer_h: u16, has_query_row: bool) -> usize {
    // border(2) + padding(2) consumed before the body block.
    let body_h = rect_height.saturating_sub(4);
    if body_h == 0 {
        return 0;
    }
    let raw_footer_h = footer_h;
    // BUG-159: the pinned query row consumes one content row.
    let query_h = if has_query_row { 1 } else { 0 };
    let spacious_min = 3
        + query_h
        + if raw_footer_h > 0 {
            raw_footer_h + 1
        } else {
            0
        };
    let spacious = body_h >= spacious_min;
    let footer_h = if raw_footer_h == 0 {
        0
    } else if spacious || body_h >= 2 + raw_footer_h {
        raw_footer_h
    } else {
        0
    };
    let content_start = if spacious { 2 } else { 1 };
    let reserved = if footer_h > 0 {
        if spacious {
            footer_h + 1
        } else {
            footer_h
        }
    } else {
        0
    };
    (body_h.saturating_sub(reserved)).saturating_sub(content_start + query_h) as usize
}
