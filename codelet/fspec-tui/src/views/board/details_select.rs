//! COPY-009 — text selection + copy inside the BoardView work-unit
//! details strip.
//!
//! Feature: spec/features/board-details-strip-text-selection-copy.feature
//!
//! Sibling of `mouse.rs` / `render.rs` (kept in its own module so
//! `board.rs`, `details_strip.rs` and `render.rs` all stay under the
//! 300-LoC source-shape ceiling). Holds the BoardView methods that:
//!   * feed a strip-local left press/drag/release to the strip's
//!     [`SelectionRecognizer`] (COPY-003) and apply the resulting
//!     gestures (`feed_details_selection`);
//!   * on Commit, reconstruct the exact border-free on-screen rows of the
//!     details strip (`truncate_to` / `wrap_to_two_lines`, fixed 5 rows)
//!     and copy them via the OSC 52 writer (COPY-001), retaining the
//!     highlight;
//!   * clear an active selection when the selected work unit changes or
//!     on Esc (`clear_details_selection`); and
//!   * paint the REVERSED highlight overlay (COPY-005) over the selected
//!     strip rows, never over the `│` side-border columns.
//!
//! Reuses COPY-001/002/003/005 unchanged.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use codelet_rpc_types::WorkUnitInfo;

use crate::mouse::gesture::SelectionGesture;
use crate::mouse::selection::{Cell, RowSpan, Selection};

use super::details_strip::visible_strip_rows;
use super::BoardView;

/// COPY-009: true for the navigation keys that change the selected work
/// unit (and thus the strip content), so an active strip selection clears.
pub(super) fn is_selection_nav_key(code: crossterm::event::KeyCode) -> bool {
    use crossterm::event::KeyCode;
    matches!(
        code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Char('h')
            | KeyCode::Char('j')
            | KeyCode::Char('k')
            | KeyCode::Char('l')
    )
}


impl BoardView {
    /// COPY-009: feed a left press/drag/release to the strip selection
    /// recognizer using STRIP-local coordinates (subtract the cached
    /// details-strip origin), then apply the resulting gestures. Returns
    /// `true` when the event landed inside the cached strip rect (so the
    /// mouse branch stops before the existing wheel/click handling).
    pub(super) fn feed_details_selection(
        &self,
        ev: crossterm::event::MouseEvent,
        rect: Rect,
        selected: Option<&WorkUnitInfo>,
    ) {
        let local = crossterm::event::MouseEvent {
            column: ev.column.saturating_sub(rect.x),
            row: ev.row.saturating_sub(rect.y),
            ..ev
        };
        let gestures = self
            .recognizer
            .borrow_mut()
            .on_mouse(local, std::time::Instant::now());
        self.apply_details_gestures(&gestures, rect, selected);
    }

    /// COPY-009: translate recognizer gestures into strip-selection state.
    /// Begin anchors the row start → the border-free content width so a
    /// bare Begin+Commit selects the whole row; Extend overrides the
    /// cursor to the drag cell; Commit reconstructs + copies.
    fn apply_details_gestures(
        &self,
        gestures: &[SelectionGesture],
        rect: Rect,
        selected: Option<&WorkUnitInfo>,
    ) {
        let cw = rect.width;
        for gesture in gestures {
            match gesture {
                SelectionGesture::Begin(cell) => {
                    *self.details_selection.borrow_mut() = Some(Selection {
                        anchor: Cell {
                            row: cell.row,
                            col: 0,
                        },
                        cursor: Cell {
                            row: cell.row,
                            col: cw,
                        },
                    });
                    *self.selection_unit_id.borrow_mut() =
                        selected.map(|u| u.id.clone());
                }
                SelectionGesture::Extend(cell) => {
                    if let Some(sel) = self.details_selection.borrow_mut().as_mut() {
                        sel.cursor = *cell;
                    }
                }
                SelectionGesture::Commit => self.commit_details_selection(rect, selected),
                SelectionGesture::Cancel => self.clear_details_selection(),
            }
        }
    }

    /// COPY-009: reconstruct the border-free selected text and write it via
    /// the OSC 52 writer. The selection is NOT cleared so the highlight
    /// persists (rule [5]); mouse capture is untouched.
    fn commit_details_selection(&self, rect: Rect, selected: Option<&WorkUnitInfo>) {
        let Some(sel) = *self.details_selection.borrow() else {
            return;
        };
        let spans = sel.spans(rect.width);
        let text = reconstruct_strip_text(&spans, rect.width, selected);
        if !text.is_empty() {
            if let Err(e) = self.clipboard.borrow_mut().copy(&text) {
                tracing::warn!("OSC 52 clipboard copy failed: {e}");
            }
        }
    }

    /// COPY-009: clear any active strip selection and reset the recognizer
    /// (rule [7]). Idempotent — a no-op when nothing is selected.
    pub(super) fn clear_details_selection(&self) {
        *self.details_selection.borrow_mut() = None;
        *self.selection_unit_id.borrow_mut() = None;
        self.details_press_active.set(false);
        *self.recognizer.borrow_mut() = crate::mouse::gesture::SelectionRecognizer::new();
    }

    /// COPY-009: clear the strip selection when the currently selected work
    /// unit differs from the one it was anchored on. Called from the render
    /// path so a keyboard/mouse selection change (which mutates the store,
    /// not the view) still clears the strip highlight (rule [7]).
    pub(super) fn sync_details_selection(&self, selected: Option<&WorkUnitInfo>) {
        let anchored = self.selection_unit_id.borrow().clone();
        let Some(anchored) = anchored else {
            return;
        };
        let current = selected.map(|u| u.id.clone());
        if current.as_deref() != Some(anchored.as_str()) {
            self.clear_details_selection();
        }
    }

    /// COPY-009: paint the REVERSED highlight overlay over the selected
    /// strip rows, clamped to the border-free content width so the `│`
    /// side-border columns are never highlighted (rule [4]).
    pub(super) fn paint_details_highlight(&self, rect: Rect, buf: &mut Buffer) {
        let Some(sel) = *self.details_selection.borrow() else {
            return;
        };
        let spans = sel.spans(rect.width);
        crate::views::agent::scrollback_paint::paint_selection_highlight(
            rect,
            buf,
            &spans,
            rect.width,
        );
    }
}

/// Reconstruct border-free text from strip-local `spans`. Each span row is
/// mapped to the matching on-screen strip row via [`visible_strip_rows`]
/// (the SAME truncation/wrap the strip render uses) and char-sliced to the
/// span columns, so the copied text mirrors the screen exactly.
fn reconstruct_strip_text(
    spans: &[RowSpan],
    width: u16,
    selected: Option<&WorkUnitInfo>,
) -> String {
    if spans.is_empty() {
        return String::new();
    }
    let rows = visible_strip_rows(selected, width);
    let lines: Vec<String> = spans
        .iter()
        .map(|span| {
            let flat = rows.get(span.row as usize).cloned().unwrap_or_default();
            slice_chars(&flat, span.start_col, span.end_col)
        })
        .collect();
    lines.join("\n")
}

/// Char-slice `text[start_col..min(end_col, char_len)]` on unicode scalar
/// boundaries, with no padding past real content (COPY-004 semantics).
fn slice_chars(text: &str, start_col: u16, end_col: u16) -> String {
    let char_len = text.chars().count();
    let end = (end_col as usize).min(char_len);
    let start = (start_col as usize).min(end);
    text.chars().skip(start).take(end - start).collect()
}
