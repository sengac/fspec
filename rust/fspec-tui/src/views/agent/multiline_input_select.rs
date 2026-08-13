//! COPY-007 — mouse text-selection + copy for the AgentView composer.
//!
//! Feature: spec/features/agentview-input-composer-text-selection-copy.feature
//!
//! Adds a `handle_mouse` seam on [`MultiLineInput`] that feeds the
//! COPY-003 [`SelectionRecognizer`] and turns the resulting gestures
//! into a live COPY-002 [`Selection`] over the composer's WRAPPED visual
//! rows. On Commit the selected text — with the `> ` prompt and side
//! padding already excluded — is reconstructed from
//! [`multiline_wrap::wrap_lines`] and returned so the caller can copy it
//! via the OSC 52 writer (COPY-001). The highlight is RETAINED after a
//! commit (rule [4]); editing keystrokes and a scroll change clear it.
//!
//! Split out of `multiline_input.rs` (which is already at the 300-LoC
//! ceiling) following the `multiline_input_enter.rs` / `_paste.rs` /
//! `_render.rs` sibling-module convention.

use std::time::Instant;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::multiline_input::MultiLineInput;
use super::multiline_input_render::{input_body_width, INPUT_PAD_X, PROMPT_WIDTH};
use super::multiline_wrap::wrap_lines;
use crate::mouse::gesture::SelectionGesture;
use crate::mouse::selection::{Cell, RowSpan, Selection};

impl MultiLineInput {
    /// COPY-007: true while a live composer text selection exists.
    pub fn text_selection_active(&self) -> bool {
        self.selection.is_some()
    }

    /// COPY-007: drop the live selection (Esc / edit / scroll).
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// COPY-007: viewport-space REVERSED highlight spans for the live
    /// selection, clamped to the body width. Empty when there is no
    /// selection. Painted by the renderer at the body origin so the
    /// `> ` prompt columns are never highlighted (rule [3]).
    pub(super) fn selection_highlight_spans(&self, body_width: u16) -> Vec<RowSpan> {
        match self.selection {
            Some(sel) => {
                let top = self.scroll_top() as u16;
                sel.spans(body_width)
                    .into_iter()
                    .filter(|s| s.row >= top)
                    .map(|s| RowSpan {
                        row: s.row - top,
                        start_col: s.start_col,
                        end_col: s.end_col,
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    /// COPY-007: route one mouse event through the selection recognizer.
    ///
    /// Converts the terminal (col,row) to a composer BODY-relative
    /// visual (row,col) — subtracting the body origin and adding
    /// `scroll_top` so the row is the logical WRAPPED-row index — then
    /// feeds the recognizer. On Begin the anchor + cursor are set; on
    /// Extend the cursor moves; on Commit the prompt-free text is
    /// reconstructed and returned (the selection is KEPT, rule [4]).
    /// Returns `None` for wheel / non-left events and for a quick click
    /// that produced no gesture.
    pub fn handle_mouse(&mut self, ev: MouseEvent, area: Rect) -> Option<String> {
        if !matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            return None;
        }
        let cell = self.mouse_to_body_cell(ev, area);
        let local = MouseEvent {
            column: cell.col,
            row: cell.row,
            ..ev
        };
        let gestures = self.recognizer.on_mouse(local, Instant::now());
        self.apply_gestures(&gestures, area)
    }

    /// COPY-007: poll the recognizer's long-press timer. Fires a Begin
    /// gesture once a stationary press has been held past the threshold.
    /// Returns `None` (a bare Begin never commits text).
    pub fn poll_selection_tick(&mut self, area: Rect) -> Option<String> {
        let gestures = self.recognizer.tick(Instant::now());
        self.apply_gestures(&gestures, area)
    }

    /// Translate the mouse (col,row) into a composer BODY-relative
    /// visual cell: subtract the body col origin (left pad + prompt) and
    /// the body row origin (`area.y`), then add `scroll_top` for the
    /// logical wrapped-row index.
    fn mouse_to_body_cell(&self, ev: MouseEvent, area: Rect) -> Cell {
        let col_origin = area
            .x
            .saturating_add(INPUT_PAD_X)
            .saturating_add(PROMPT_WIDTH);
        let col = ev.column.saturating_sub(col_origin);
        let rel_row = ev.row.saturating_sub(area.y);
        let row = rel_row.saturating_add(self.scroll_top() as u16);
        Cell { row, col }
    }

    /// Apply recognizer gestures to the live selection. Returns
    /// `Some(text)` on Commit, `None` otherwise.
    fn apply_gestures(&mut self, gestures: &[SelectionGesture], area: Rect) -> Option<String> {
        let mut committed: Option<String> = None;
        for gesture in gestures {
            match gesture {
                SelectionGesture::Begin(cell) => {
                    // COPY-010: anchor precisely at the pressed cell (real
                    // row AND column) so a drag selects from the press
                    // column, not the line start. Extend then moves the
                    // cursor to the pointer.
                    self.selection = Some(Selection::collapsed(*cell));
                }
                SelectionGesture::BeginLine(cell) => {
                    // COPY-010: a long-press selects the WHOLE row under
                    // the press — anchor row start → body width.
                    let body_width = input_body_width(area.width);
                    self.selection = Some(Selection::whole_line(cell.row, body_width));
                }
                SelectionGesture::Extend(cell) => {
                    if let Some(sel) = self.selection.as_mut() {
                        sel.cursor = *cell;
                    }
                }
                SelectionGesture::Commit => {
                    committed = Some(self.reconstruct_selected_text(area));
                }
                SelectionGesture::Cancel => {
                    self.selection = None;
                }
            }
        }
        committed
    }

    /// Reconstruct the selected text from the wrapped visual rows,
    /// excluding the prompt/pad columns (already subtracted, so spans
    /// are body-relative) and clamped to each row's real char length.
    fn reconstruct_selected_text(&self, area: Rect) -> String {
        let body_width = input_body_width(area.width);
        let selection = match self.selection {
            Some(sel) => sel,
            None => return String::new(),
        };
        let spans = selection.spans(body_width);
        if spans.is_empty() {
            return String::new();
        }
        let rows = wrap_lines(self.lines(), body_width);
        let out: Vec<String> = spans
            .iter()
            .map(|span| {
                let text = rows
                    .get(span.row as usize)
                    .map(|r| r.text.as_str())
                    .unwrap_or("");
                slice_chars(text, span.start_col, span.end_col)
            })
            .collect();
        out.join("\n")
    }
}

/// Char-slice `text[start_col..min(end_col, char_len)]` on unicode
/// scalar boundaries, with no padding past real content.
fn slice_chars(text: &str, start_col: u16, end_col: u16) -> String {
    let char_len = text.chars().count();
    let end = (end_col as usize).min(char_len);
    let start = (start_col as usize).min(end);
    text.chars().skip(start).take(end - start).collect()
}

#[cfg(test)]
#[path = "multiline_input_select_tests.rs"]
mod tests;
