//! RPC-023: BoardView Event::Mouse branch extracted to keep
//! `views/board.rs` under the 300 LoC ceiling.
//!
//! Feature files:
//!   - spec/features/boardview-mouse-handling.feature
//!   - spec/features/app-mouse-dispatch.feature
//!
//! The render path (in `views/board.rs`) caches three pieces of
//! geometry in `Cell<Option<...>>` fields on [`BoardView`]:
//!   * `last_content_area`           — the full content `Rect` (split[7]).
//!   * `last_column_header_areas`    — per-column header rects (split[5] sliced).
//!   * `last_column_content_areas`   — per-column content rects (split[7] sliced).
//!
//! This module reads those cached rects and translates wheel + click
//! events into Actions. Wheel events delegate to RPC-016's existing
//! `Action::SelectPrev` / `SelectNext` / `FocusPrev/NextColumn` so the
//! viewport math is reused for free.
//!
//! Decision (Q9) explicitly defers TUI-078 native text selection to
//! RPC-019; this module never touches `MouseTrackingToggle`.

use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};

use crate::components::{Action, EventResult};
use crate::mouse::rect_contains;
use crate::store::BoardStore;

use super::BoardView;

/// Handle an [`Event::Mouse`] against the BoardView's cached layout.
/// Returns [`EventResult::Consumed`] when the event landed inside a
/// known hit-test region; [`EventResult::Ignored`] otherwise.
///
/// Variants handled:
///   * `ScrollUp` / `ScrollDown` inside `last_content_area`        → SelectPrev / Next
///   * `ScrollLeft` / `ScrollRight` inside `last_content_area`     → FocusPrev / NextColumn
///   * `Down(Left)` inside a `last_column_header_areas[idx]`        → SetFocusedColumn(idx)
///   * `Down(Left)` inside a `last_column_content_areas[idx]` cell  → SetFocusedColumn(idx)
///    + SelectIndexInFocused(row + scroll_offset)
pub(super) fn handle_mouse(view: &BoardView, event: &Event, store: &BoardStore) -> EventResult {
    let MouseEvent {
        kind, column, row, ..
    } = match event {
        Event::Mouse(m) => *m,
        _ => return EventResult::ignored(),
    };

    let in_content = view
        .last_content_area
        .get()
        .map(|r| rect_contains(r, column, row))
        .unwrap_or(false);

    match kind {
        MouseEventKind::ScrollUp if in_content => {
            view.emit(Action::SelectPrev);
            EventResult::consumed()
        }
        MouseEventKind::ScrollDown if in_content => {
            view.emit(Action::SelectNext);
            EventResult::consumed()
        }
        MouseEventKind::ScrollLeft if in_content => {
            view.emit(Action::FocusPrevColumn);
            EventResult::consumed()
        }
        MouseEventKind::ScrollRight if in_content => {
            view.emit(Action::FocusNextColumn);
            EventResult::consumed()
        }
        MouseEventKind::Down(MouseButton::Left) => handle_left_click(view, column, row, store),
        _ => EventResult::ignored(),
    }
}

fn handle_left_click(
    view: &BoardView,
    column: u16,
    row: u16,
    store: &BoardStore,
) -> EventResult {
    // Header click → focus that column.
    if let Some(headers) = view.last_column_header_areas.get() {
        for (idx, rect) in headers.iter().enumerate() {
            if rect_contains(*rect, column, row) {
                view.emit(Action::SetFocusedColumn(idx));
                return EventResult::consumed();
            }
        }
    }
    // Content-row click → focus column AND select row under cursor,
    // accounting for the column's scroll_offset.
    if let Some(contents) = view.last_column_content_areas.get() {
        for (idx, rect) in contents.iter().enumerate() {
            if rect_contains(*rect, column, row) {
                let row_in_col = row.saturating_sub(rect.y) as usize;
                let column_name = crate::store::COLUMN_ORDER[idx];
                let target = store.scroll_offset_for(column_name) + row_in_col;
                view.emit(Action::SetFocusedColumn(idx));
                view.emit(Action::SelectIndexInFocused(target));
                return EventResult::consumed();
            }
        }
    }
    EventResult::ignored()
}
