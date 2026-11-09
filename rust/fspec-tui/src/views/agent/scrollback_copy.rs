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
use crate::views::agent::text_wrap::RowEnd;

use super::ScrollbackList;

impl ScrollbackList {
    /// Reconstruct the plain text covered by `region`.
    ///
    /// `region` RowSpans are in VIEWPORT-row space; each maps to a
    /// scrollback visual row via `scroll_state.offset + span.row`. Each
    /// row is flattened by concatenating its `Line` spans' content, then
    /// char-sliced `[start_col .. min(end_col, char_len)]` (unicode-safe,
    /// no padding past real content). Rows are joined with `\n`, or with what
    /// the wrap removed where it split a line (TUI-130). An empty
    /// region yields an empty string.
    pub fn selected_text(&self, region: &[RowSpan]) -> String {
        if region.is_empty() {
            return String::new();
        }
        let offset = self.scroll_state.offset;
        let mut text = String::new();
        for (i, span) in region.iter().enumerate() {
            let (flat, end) = self.flatten_visual_row(offset + span.row as usize);
            text.push_str(&slice_chars(&flat, span.start_col, span.end_col));
            if i + 1 < region.len() {
                // TUI-130: a row the wrap split joins the next one with
                // what the wrap removed, not a line break.
                text.push_str(end.joiner());
            }
        }
        text
    }

    /// Flatten the visual row at `visual_index` into a plain String by
    /// walking chunks in order (same iteration as `total_visual_rows`)
    /// and concatenating the located `Line`'s spans' content, with how the
    /// row ends. Out-of-range indices produce an empty line.
    fn flatten_visual_row(&self, visual_index: usize) -> (String, RowEnd) {
        let mut row_idx = 0usize;
        for chunk in self.chunks.iter() {
            let chunk_rows = chunk.lines.len();
            if visual_index < row_idx + chunk_rows {
                let line = &chunk.lines[visual_index - row_idx];
                let flat = line.spans.iter().map(|s| s.content.as_ref()).collect();
                return (flat, chunk.row_end(visual_index - row_idx));
            }
            row_idx += chunk_rows;
        }
        (String::new(), RowEnd::Break)
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
#[path = "scrollback_copy_tests.rs"]
mod tests;
