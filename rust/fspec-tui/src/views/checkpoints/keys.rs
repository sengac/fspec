//! RPC-364/365 — `CheckpointsView` key + mouse event routing.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Split out of `mod.rs` (RPC-365) so every file stays under the
//! 300-line ceiling. Holds `handle_key` (pane nav + restore keys, with
//! the restore modal capturing input while active) and `handle_mouse`
//! (wheel routed to the pane under the cursor).
//!
//! TUI-101: scrollbar click-and-drag navigation for all three panes.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::components::scroll_viewport::WheelDirection;
use crate::mouse::rect_contains;
use crate::mouse::scrollbar_drag::ScrollbarGeometry;

use super::{CheckpointsEvent, CheckpointsView, Pane};

impl CheckpointsView {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> CheckpointsEvent {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return CheckpointsEvent::Ignored;
        }
        // RPC-365: while a restore dialog is active it captures all input.
        if self.dialog().is_some() {
            return self.handle_dialog_key(key);
        }
        // RPC-366: while a delete dialog is active it captures all input.
        if self.delete_dialog().is_some() {
            return self.handle_delete_dialog_key(key);
        }
        // TUI-107: while the loading dialog is active it captures input —
        // ESC is Ignored (the view stays open; a reflex ESC mid-load must
        // not force a re-open + re-load), every other key is swallowed.
        // Guard order: restore/delete dialogs keep precedence (invariant).
        if self.is_loading() {
            return self.handle_loading_key(key);
        }
        match key.code {
            KeyCode::Esc => CheckpointsEvent::Close,
            KeyCode::Tab | KeyCode::Right => {
                self.cycle_pane(true);
                CheckpointsEvent::Consumed
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.cycle_pane(false);
                CheckpointsEvent::Consumed
            }
            KeyCode::Down => self.scroll_focused(1),
            KeyCode::Up => self.scroll_focused(-1),
            KeyCode::PageDown => self.scroll_focused(self.page_step()),
            KeyCode::PageUp => self.scroll_focused(-self.page_step()),
            // RPC-365: restore single file (r/R) / all files (t/T).
            KeyCode::Char('r') | KeyCode::Char('R') => self.open_restore_single(),
            KeyCode::Char('t') | KeyCode::Char('T') => self.open_restore_all(),
            // RPC-366: delete single checkpoint (d/D) / all checkpoints (a/A).
            KeyCode::Char('d') | KeyCode::Char('D') => self.open_delete_single(),
            KeyCode::Char('a') | KeyCode::Char('A') => self.open_delete_all(),
            _ => CheckpointsEvent::Consumed,
        }
    }

    /// TUI-107: key routing while the loading dialog is active. ESC →
    /// `Ignored` (the view stays open — mirrors the StatusDialog rule
    /// that a reflex ESC during a short load must not close-with-no-data);
    /// every other key is swallowed so no selection/scroll state mutates
    /// mid-load.
    fn handle_loading_key(&mut self, key: KeyEvent) -> CheckpointsEvent {
        if key.code == KeyCode::Esc {
            CheckpointsEvent::Ignored
        } else {
            CheckpointsEvent::Consumed
        }
    }

    pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> CheckpointsEvent {
        // RPC-365/366: the restore/delete modal swallows all mouse input
        // while active — this guard MUST run before click + wheel handling.
        if self.dialog().is_some() || self.delete_dialog().is_some() {
            return CheckpointsEvent::Consumed;
        }
        // TUI-107: the loading dialog swallows all mouse input while a
        // cascade stage is in flight (nothing is selectable mid-load).
        if self.is_loading() {
            return CheckpointsEvent::Consumed;
        }

        // TUI-101: handle scrollbar click-and-drag first.
        if matches!(
            ev.kind,
            MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                | MouseEventKind::Up(MouseButton::Left)
        ) {
            // Hit-test checkpoints scrollbar
            if let Some(sb_rect) = self.last_cp_sb_rect {
                if rect_contains(sb_rect, ev.column, ev.row) {
                    let visible = self.last_checkpoints_rect.map(|r| r.height as usize).unwrap_or(0);
                    let total = self.checkpoints.len();
                    if total > visible {
                        let geom = ScrollbarGeometry {
                            area_height: visible,
                            total_items: total,
                            visible_items: visible,
                            current_offset: self.checkpoint_scroll,
                        };
                        if let Some(offset) = self.cp_scrollbar_drag.on_mouse(ev, geom) {
                            self.checkpoint_scroll = offset;
                        }
                        return CheckpointsEvent::Consumed;
                    }
                }
            }
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
                        return CheckpointsEvent::Consumed;
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
                        return CheckpointsEvent::Consumed;
                    }
                }
            }
            // Click outside scrollbar: reset drag states on Up
            if matches!(ev.kind, MouseEventKind::Up(MouseButton::Left)) {
                self.cp_scrollbar_drag.reset();
                self.files_scrollbar_drag.reset();
                self.diff_scrollbar_drag.reset();
            }
            // Fall through to click handling
        }

        // RPC-369: a left click selects the row under the cursor.
        if let MouseEventKind::Down(_) = ev.kind {
            return self.handle_click(ev.column, ev.row);
        }
        let dir = match ev.kind {
            MouseEventKind::ScrollUp => WheelDirection::Up,
            MouseEventKind::ScrollDown => WheelDirection::Down,
            _ => return CheckpointsEvent::Ignored,
        };
        let target = self
            .pane_at(ev.column, ev.row)
            .unwrap_or_else(|| self.focused_pane());
        let step = self.wheel_step(dir);
        match target {
            Pane::Diff => {
                self.apply_diff_scroll(step);
                CheckpointsEvent::Consumed
            }
            Pane::Files => self.move_file_selection(step),
            Pane::Checkpoints => self.move_checkpoint_selection(step),
        }
    }

    /// RPC-369: hit-test a left click. Focuses the pane under the cursor;
    /// a Checkpoints/Files click selects the row (reusing the navigation
    /// setters so the clamp / `ensure_visible` / `Emit` path is shared); a
    /// Diff click only focuses. Clicks past the last populated row or
    /// outside all rects change nothing.
    fn handle_click(&mut self, col: u16, row: u16) -> CheckpointsEvent {
        let pane = match self.pane_at(col, row) {
            Some(p) => p,
            None => return CheckpointsEvent::Ignored,
        };
        self.set_focused_pane(pane);
        match pane {
            Pane::Diff => CheckpointsEvent::Consumed,
            Pane::Checkpoints => match self.row_target(row, true) {
                Some(target) => self
                    .move_checkpoint_selection(target as i32 - self.selected_checkpoint() as i32),
                None => CheckpointsEvent::Consumed,
            },
            Pane::Files => match self.row_target(row, false) {
                Some(target) => {
                    self.move_file_selection(target as i32 - self.selected_file() as i32)
                }
                None => CheckpointsEvent::Consumed,
            },
        }
    }
}
