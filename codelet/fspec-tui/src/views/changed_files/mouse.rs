//! RPC-368 — left-click selection for `ChangedFilesView`.
//!
//! Feature: spec/features/changed-files-view-click-to-select.feature
//!
//! A `MouseEventKind::Down(_)` over the Files pane selects the clicked
//! file row (reusing `move_selection` so the clamp / `ensure_visible` /
//! `Emit(LoadFileDiff)` path is shared); a click over the Diff pane only
//! focuses it. Clicks on empty space below the last file or outside both
//! pane rects change nothing.
//!
//! TUI-101: scrollbar click-and-drag navigation for both panes.

use super::{ChangedFilesEvent, ChangedFilesView, Pane};
use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

impl ChangedFilesView {
    /// Route a mouse event. `Down` left-clicks select a row (delegated to
    /// `handle_click`); scroll-wheel events scroll the pane under the
    /// cursor (or the focused pane when outside both rects). Called from
    /// `handle_event` in the parent module.
    ///
    /// TUI-101: scrollbar click-and-drag events are handled when the cursor
    /// lands on a scrollbar gutter.
    pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent {
        // TUI-101: handle scrollbar click-and-drag first.
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            // Hit-test files scrollbar
            if let Some(sb_rect) = self.last_files_sb_rect {
                if rect_contains(sb_rect, ev.column, ev.row) {
                    let visible = self.last_files_rect.map(|r| r.height as usize).unwrap_or(0);
                    let total = self.files.len();
                    if total > visible {
                        let geom = ScrollbarGeometry {
                            area_height: visible,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.file_scroll,
                        };
                        if let Some(offset) = self.files_scrollbar_drag.on_mouse(ev, geom) {
                            self.file_scroll = offset;
                        }
                        return ChangedFilesEvent::Consumed;
                    }
                }
            }
            // Hit-test diff scrollbar
            if let Some(sb_rect) = self.last_diff_sb_rect {
                if rect_contains(sb_rect, ev.column, ev.row) {
                    let visible = self.last_diff_rect.map(|r| r.height as usize).unwrap_or(0);
                    let total = self.diff_lines.len();
                    if total > visible {
                        let geom = ScrollbarGeometry {
                            area_height: visible,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.diff_scroll,
                        };
                        if let Some(offset) = self.diff_scrollbar_drag.on_mouse(ev, geom) {
                            self.diff_scroll = offset;
                        }
                        return ChangedFilesEvent::Consumed;
                    }
                }
            }
            // Click outside scrollbar: reset drag states on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.files_scrollbar_drag.reset();
                self.diff_scrollbar_drag.reset();
            }
            // Fall through to click handling
        }

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
