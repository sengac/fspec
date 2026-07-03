//! COPY-005 — selection highlight overlay for the AgentView scrollback.
//!
//! Feature: spec/features/scrollback-selection-highlight.feature
//!
//! Paints the live text-selection region with a REVERSED style overlay,
//! called from `ScrollbackList::render_count_visited` AFTER the chunk
//! text and the RPC-381 DIM arrow bars, so it visually overlays existing
//! content without erasing it (only fg/bg swap via `Cell::set_style`).
//!
//! Input is viewport-space [`RowSpan`]s (already offset-mapped and
//! content-width-clamped) that COPY-006 derives from the live Selection —
//! COPY-005 itself only paints. The gutter column (`>= content_width`) is
//! never touched thanks to the `end_col.min(content_width)` clamp,
//! guaranteeing the scrollbar glyph is never highlighted.
//!
//! Extracted into its own sibling module so both `scrollback_paint.rs`
//! and `scrollback.rs` stay under the 300-LoC source-shape ceiling.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::mouse::selection::RowSpan;

/// Overlay the REVERSED style onto every cell covered by
/// `spans_in_viewport`, clamped to `content_width` (so the reserved
/// scrollbar gutter is never highlighted) and to the `area` bounds (so
/// rows/cols scrolled out of view are skipped). An empty span slice
/// paints nothing (no active selection).
pub(crate) fn paint_selection_highlight(
    area: Rect,
    buf: &mut Buffer,
    spans_in_viewport: &[RowSpan],
    content_width: u16,
) {
    let reversed = Style::default().add_modifier(Modifier::REVERSED);
    let y_end = area.y.saturating_add(area.height);
    let x_end = area.x.saturating_add(area.width);
    for span in spans_in_viewport {
        let y = area.y.saturating_add(span.row);
        if y >= y_end {
            continue;
        }
        let end = span.end_col.min(content_width);
        for col in span.start_col..end {
            let x = area.x.saturating_add(col);
            if x >= x_end {
                break;
            }
            buf[(x, y)].set_style(reversed);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/scrollback-selection-highlight.feature
    use super::*;

    fn is_reversed(buf: &Buffer, x: u16, y: u16) -> bool {
        buf[(x, y)].modifier.contains(Modifier::REVERSED)
    }

    #[test]
    fn selected_cells_are_painted_with_the_reversed_style() {
        // @step Given a rendered scrollback with an active single-row selection at viewport row 3 columns 0 to 5
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let spans = [RowSpan {
            row: 3,
            start_col: 0,
            end_col: 5,
        }];

        // @step When the selection highlight is painted
        paint_selection_highlight(area, &mut buf, &spans, 20);

        // @step Then the buffer cells at row 3 columns 0 to 5 carry the reversed style
        for col in 0..5 {
            assert!(is_reversed(&buf, col, 3), "cell (col {col}, row 3) reversed");
        }
        assert!(!is_reversed(&buf, 5, 3), "col 5 (past end) not reversed");
    }

    #[test]
    fn the_scrollbar_gutter_column_is_never_highlighted() {
        // @step Given a rendered scrollback with a full-width selection on a row that has a scrollbar gutter
        let area = Rect::new(0, 0, 20, 10);
        let content_width = 18; // area.width 20 minus a 2-col gutter.
        let mut buf = Buffer::empty(area);
        let spans = [RowSpan {
            row: 2,
            start_col: 0,
            end_col: 20,
        }];

        // @step When the selection highlight is painted
        paint_selection_highlight(area, &mut buf, &spans, content_width);

        // @step Then the reserved gutter column is left un-highlighted
        assert!(is_reversed(&buf, content_width - 1, 2), "last content col reversed");
        assert!(!is_reversed(&buf, content_width, 2), "gutter col not reversed");
        assert!(!is_reversed(&buf, area.width - 1, 2), "rightmost gutter col not reversed");
    }

    #[test]
    fn no_selection_paints_no_highlight() {
        // @step Given a rendered scrollback with no active selection
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        let spans: &[RowSpan] = &[];

        // @step When the selection highlight is painted
        paint_selection_highlight(area, &mut buf, spans, 20);

        // @step Then every cell keeps its original style and no cell is reversed
        for y in 0..area.height {
            for x in 0..area.width {
                assert!(!is_reversed(&buf, x, y), "cell ({x},{y}) must not be reversed");
            }
        }
    }

    #[test]
    fn a_multi_row_selection_highlights_first_middle_and_last_rows_correctly() {
        // @step Given a rendered scrollback with an active selection spanning three viewport rows
        let area = Rect::new(0, 0, 20, 10);
        let content_width = 18;
        let mut buf = Buffer::empty(area);
        let spans = [
            RowSpan {
                row: 0,
                start_col: 4,
                end_col: content_width,
            },
            RowSpan {
                row: 1,
                start_col: 0,
                end_col: content_width,
            },
            RowSpan {
                row: 2,
                start_col: 0,
                end_col: 6,
            },
        ];

        // @step When the selection highlight is painted
        paint_selection_highlight(area, &mut buf, &spans, content_width);

        // @step Then the first row is highlighted from its start column to the content width
        assert!(!is_reversed(&buf, 3, 0), "first row before start not reversed");
        for col in 4..content_width {
            assert!(is_reversed(&buf, col, 0), "first row col {col} reversed");
        }

        // @step And the middle row is highlighted fully within the content width
        for col in 0..content_width {
            assert!(is_reversed(&buf, col, 1), "middle row col {col} reversed");
        }
        assert!(!is_reversed(&buf, content_width, 1), "middle row gutter not reversed");

        // @step And the last row is highlighted up to its end column
        for col in 0..6 {
            assert!(is_reversed(&buf, col, 2), "last row col {col} reversed");
        }
        assert!(!is_reversed(&buf, 6, 2), "last row past end not reversed");
    }

    #[test]
    fn a_selection_scrolled_partly_off_screen_highlights_only_the_visible_portion() {
        // @step Given a rendered scrollback whose selection top row is above the viewport
        // The caller passes only viewport-space spans; a span with row >=
        // area.height models the off-screen remainder and must be skipped.
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        let spans = [
            RowSpan {
                row: 0,
                start_col: 0,
                end_col: 5,
            },
            RowSpan {
                row: 1,
                start_col: 0,
                end_col: 5,
            },
            RowSpan {
                row: 4, // beyond area.height (0..3 visible) -> off-screen.
                start_col: 0,
                end_col: 5,
            },
        ];

        // @step When the selection highlight is painted
        paint_selection_highlight(area, &mut buf, &spans, 20);

        // @step Then only the visible rows of the selection carry the reversed style
        for col in 0..5 {
            assert!(is_reversed(&buf, col, 0), "visible row 0 col {col} reversed");
            assert!(is_reversed(&buf, col, 1), "visible row 1 col {col} reversed");
        }
        for y in 2..area.height {
            for x in 0..area.width {
                assert!(!is_reversed(&buf, x, y), "off-region cell ({x},{y}) not reversed");
            }
        }
    }
}
