//! RPC-368 — left-click selection for `ChangedFilesView`.
//!
//! Feature: spec/features/changed-files-view-click-to-select.feature
//!
//! A `MouseEventKind::Down(_)` over the Files pane selects the clicked
//! file row (reusing `move_selection` so the clamp / `ensure_visible` /
//! `Emit(LoadFileDiff)` path is shared); a click over the Diff pane only
//! focuses it. Clicks on empty space below the last file or outside both
//! pane rects change nothing.

use super::{ChangedFilesEvent, ChangedFilesView, Pane};
use crate::components::scroll_viewport::WheelDirection;
use crossterm::event::{MouseEvent, MouseEventKind};

impl ChangedFilesView {
    /// Route a mouse event. `Down` left-clicks select a row (delegated to
    /// `handle_click`); scroll-wheel events scroll the pane under the
    /// cursor (or the focused pane when outside both rects). Called from
    /// `handle_event` in the parent module.
    pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent {
        if let MouseEventKind::Down(_) = ev.kind {
            return self.handle_click(ev.column, ev.row);
        }
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return ChangedFilesEvent::Ignored,
        };
        // Hit-test which pane the wheel landed over; default to the
        // currently-focused pane when the cursor is outside both rects.
        let target = self.pane_at(ev.column, ev.row).unwrap_or(self.focused_pane);
        let step = self.wheel.step(dir);
        match target {
            Pane::Diff => {
                self.apply_diff_scroll(step);
                ChangedFilesEvent::Consumed
            }
            // Mirror `handle_key`: propagate the Emit(LoadFileDiff) so a
            // wheel-driven selection change reloads the diff pane.
            Pane::Files => self.move_selection(step),
        }
    }

    /// Handle a left-click hit-test. Called from `handle_mouse` for
    /// `MouseEventKind::Down(_)` events.
    pub(super) fn handle_click(&mut self, col: u16, row: u16) -> ChangedFilesEvent {
        match self.pane_at(col, row) {
            Some(Pane::Diff) => {
                self.focused_pane = Pane::Diff;
                ChangedFilesEvent::Consumed
            }
            Some(Pane::Files) => {
                self.focused_pane = Pane::Files;
                let rect = match self.last_files_rect {
                    Some(r) => r,
                    None => return ChangedFilesEvent::Consumed,
                };
                let offset = row.saturating_sub(rect.y) as usize;
                // Click landed on empty space below the last visible file row.
                if offset >= self.files.len().saturating_sub(self.file_scroll) {
                    return ChangedFilesEvent::Consumed;
                }
                let target = self.file_scroll + offset; // < files.len()
                self.move_selection(target as i32 - self.selected_index as i32)
            }
            None => ChangedFilesEvent::Ignored,
        }
    }
}
