//! COPY-004 — reconstruct selected text from scrollback, excluding the
//! scrollbar gutter.
//!
//! Feature: spec/features/scrollback-selected-text-reconstruction.feature
//!
//! Extracted from `scrollback.rs` to keep that file under the 300-LoC
//! source-shape ceiling (like `scrollback_select.rs`). Declared as a
//! child module of `scrollback` via `#[path] mod copy;`, so it can
//! `impl super::ScrollbackList` and read the private `chunks` /
//! `scroll_state` fields.
//!
//! Takes the [`RowSpan`]s produced by COPY-002's `Selection::spans`
//! (already clamped to content width by the COPY-006 caller) and
//! additionally clamps each row's end to the row's real char length —
//! double-guarding the scrollbar-glyph exclusion. Pure: no I/O.

use crate::mouse::selection::RowSpan;

use super::ScrollbackList;

impl ScrollbackList {
    /// Reconstruct the plain text covered by `region`.
    ///
    /// `region` RowSpans are in VIEWPORT-row space; each maps to a
    /// scrollback visual row via `scroll_state.offset + span.row`. Each
    /// row is flattened by concatenating its `Line` spans' content, then
    /// char-sliced `[start_col .. min(end_col, char_len)]` (unicode-safe,
    /// no padding past real content). Rows are joined with `\n`. An empty
    /// region yields an empty string.
    pub fn selected_text(&self, region: &[RowSpan]) -> String {
        if region.is_empty() {
            return String::new();
        }
        let offset = self.scroll_state.offset;
        let rows: Vec<String> = region
            .iter()
            .map(|span| {
                let visual_index = offset + span.row as usize;
                let flat = self.flatten_visual_row(visual_index);
                slice_chars(&flat, span.start_col, span.end_col)
            })
            .collect();
        rows.join("\n")
    }

    /// Flatten the visual row at `visual_index` into a plain String by
    /// walking chunks in order (same iteration as `total_visual_rows`)
    /// and concatenating the located `Line`'s spans' content. Out-of-range
    /// indices produce an empty string.
    fn flatten_visual_row(&self, visual_index: usize) -> String {
        let mut row_idx = 0usize;
        for chunk in self.chunks.iter() {
            let chunk_rows = chunk.lines.len();
            if visual_index < row_idx + chunk_rows {
                let line = &chunk.lines[visual_index - row_idx];
                return line.spans.iter().map(|s| s.content.as_ref()).collect();
            }
            row_idx += chunk_rows;
        }
        String::new()
    }

    /// Test-only: set the raw scroll offset directly so viewport→visual
    /// row mapping can be exercised without driving the full scroll API.
    #[cfg(test)]
    fn set_offset_for_test(&mut self, offset: usize) {
        self.scroll_state.offset = offset;
    }

    /// COPY-005/006: replace the viewport-space selection spans painted as
    /// the REVERSED highlight overlay. Empty clears the highlight. The
    /// caller (COPY-006) populates this from the live Selection.
    #[allow(dead_code)] // Wired into render now; populated by COPY-006.
    pub(crate) fn set_selection_highlight_spans(&mut self, spans: Vec<RowSpan>) {
        self.selection_highlight_spans = spans;
    }

    /// COPY-006/010: begin a live selection anchored PRECISELY at the
    /// pressed cell so a drag copies from the press column (COPY-010).
    pub(crate) fn selection_begin(&mut self, cell: crate::mouse::selection::Cell) {
        use crate::mouse::selection::Selection;
        self.selection = Some(Selection::collapsed(cell));
        self.refresh_selection_highlight();
    }

    /// COPY-010: begin a whole-line selection on the press row (long-press):
    /// anchor line start → gutter-free content width (feature example 5).
    pub(crate) fn selection_begin_line(&mut self, cell: crate::mouse::selection::Cell) {
        use crate::mouse::selection::Selection;
        self.selection = Some(Selection::whole_line(cell.row, self.content_width));
        self.refresh_selection_highlight();
    }

    /// COPY-006: move the live selection's cursor to `cell` (drag).
    /// No-op when no selection is active.
    pub(crate) fn selection_extend(&mut self, cell: crate::mouse::selection::Cell) {
        if let Some(sel) = self.selection.as_mut() {
            sel.cursor = cell;
        }
        self.refresh_selection_highlight();
    }

    /// COPY-006: drop the live selection AND its highlight overlay.
    pub(crate) fn selection_clear(&mut self) {
        self.selection = None;
        self.selection_highlight_spans.clear();
    }

    /// COPY-006 test-observability seam: true while a live text
    /// selection is anchored on this scrollback. `pub` so integration
    /// tests can assert the highlight persisted / cleared.
    pub fn text_selection_active(&self) -> bool {
        self.selection.is_some()
    }

    /// COPY-006 test-observability seam: number of viewport-space
    /// REVERSED highlight spans painted for the live selection (0 when
    /// there is none). Lets tests assert the highlight is present/gone.
    pub fn selection_highlight_span_count(&self) -> usize {
        self.selection_highlight_spans.len()
    }

    /// COPY-006: normalized viewport-space RowSpans for the live
    /// selection, clamped to the cached content width. Empty when there
    /// is no selection or the selection is collapsed.
    pub(crate) fn selection_spans_for_content_width(&self) -> Vec<RowSpan> {
        match self.selection {
            Some(sel) => sel.spans(self.content_width),
            None => Vec::new(),
        }
    }

    /// COPY-006: recompute + store the REVERSED highlight spans from the
    /// live selection so the next render paints it.
    pub(crate) fn refresh_selection_highlight(&mut self) {
        self.selection_highlight_spans = self.selection_spans_for_content_width();
    }
}

/// Char-slice `text[start_col..min(end_col, char_len)]` on unicode scalar
/// boundaries, with no padding past real content.
fn slice_chars(text: &str, start_col: u16, end_col: u16) -> String {
    let char_len = text.chars().count();
    let end = (end_col as usize).min(char_len);
    let start = (start_col as usize).min(end);
    text.chars().skip(start).take(end - start).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! Feature: spec/features/scrollback-selected-text-reconstruction.feature
    use super::*;
    use crate::views::agent::RenderedChunk;
    use ratatui::text::{Line, Span};

    fn chunk(seq: u64, body: &str) -> RenderedChunk {
        RenderedChunk {
            seq,
            lines: vec![Line::from(Span::raw(body.to_string()))],
            source: None,
        }
    }

    #[test]
    fn reconstruct_a_single_row_partial_selection() {
        // @step Given a scrollback whose visual row reads "Hello world"
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "Hello world"));

        // @step When I reconstruct the selection of that row from column 0 to column 5
        let text = list.selected_text(&[RowSpan {
            row: 0,
            start_col: 0,
            end_col: 5,
        }]);

        // @step Then the reconstructed text is "Hello"
        assert_eq!(text, "Hello");
    }

    #[test]
    fn reconstruct_a_multi_row_selection_joined_by_newline() {
        // @step Given a scrollback with consecutive visual rows "foo" and "bar"
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "foo"));
        list.push(chunk(1, "bar"));

        // @step When I reconstruct the full-width selection spanning both rows
        let text = list.selected_text(&[
            RowSpan {
                row: 0,
                start_col: 0,
                end_col: 3,
            },
            RowSpan {
                row: 1,
                start_col: 0,
                end_col: 3,
            },
        ]);

        // @step Then the reconstructed text is "foo\nbar"
        assert_eq!(text, "foo\nbar");
    }

    #[test]
    fn the_scrollbar_gutter_glyph_is_excluded_from_copied_text() {
        // @step Given a scrollback row rendered as "answer text  │" where the │ sits in the reserved scrollbar gutter
        // The visual row carries the gutter glyph after the content; the
        // content width is 11 chars ("answer text"), the │ sits beyond it.
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "answer text│"));

        // @step When I reconstruct the full-width selection of that row clamped to the content width
        let text = list.selected_text(&[RowSpan {
            row: 0,
            start_col: 0,
            end_col: 11,
        }]);

        // @step Then the reconstructed text is "answer text"
        assert_eq!(text, "answer text");
        assert!(!text.contains('│'));
    }

    #[test]
    fn scroll_offset_maps_viewport_rows_to_visual_rows() {
        // @step Given a scrollback with scroll offset 10
        // Build 13 single-row chunks (visual rows 0..=12); row 12 has
        // known text. Set offset to 10 so viewport row 2 -> visual row 12.
        let mut list = ScrollbackList::new();
        for i in 0..13u64 {
            list.push(chunk(i, &format!("visual{i}")));
        }
        list.set_offset_for_test(10);

        // @step When I reconstruct the selection of viewport row 2
        let text = list.selected_text(&[RowSpan {
            row: 2,
            start_col: 0,
            end_col: 20,
        }]);

        // @step Then the text comes from scrollback visual row 12
        assert_eq!(text, "visual12");
    }

    #[test]
    fn end_column_beyond_content_length_is_clamped_without_padding() {
        // @step Given a scrollback whose visual row reads "hi"
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "hi"));

        // @step When I reconstruct the selection of that row from column 0 to column 20
        let text = list.selected_text(&[RowSpan {
            row: 0,
            start_col: 0,
            end_col: 20,
        }]);

        // @step Then the reconstructed text is "hi" with no trailing spaces
        assert_eq!(text, "hi");
    }

    #[test]
    fn multi_byte_glyphs_are_sliced_on_character_boundaries() {
        // @step Given a scrollback whose visual row reads "a😀b"
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "a😀b"));

        // @step When I reconstruct the selection of that row from column 0 to column 2
        let text = list.selected_text(&[RowSpan {
            row: 0,
            start_col: 0,
            end_col: 2,
        }]);

        // @step Then the reconstructed text is "a😀" and the emoji bytes are not split
        assert_eq!(text, "a😀");
    }

    #[test]
    fn an_empty_selection_reconstructs_an_empty_string() {
        // @step Given a scrollback with a collapsed (empty) selection region
        let mut list = ScrollbackList::new();
        list.push(chunk(0, "Hello world"));
        let region: &[RowSpan] = &[];

        // @step When I reconstruct the selected text
        let text = list.selected_text(region);

        // @step Then the reconstructed text is the empty string
        assert_eq!(text, "");
    }
}
