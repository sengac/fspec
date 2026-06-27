//! RPC-364/365 — `CheckpointsView` key + mouse event routing.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Split out of `mod.rs` (RPC-365) so every file stays under the
//! 300-line ceiling. Holds `handle_key` (pane nav + restore keys, with
//! the restore modal capturing input while active) and `handle_mouse`
//! (wheel routed to the pane under the cursor).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::components::scroll_viewport::WheelDirection;

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

    pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> CheckpointsEvent {
        // RPC-365/366: the restore/delete modal swallows wheel events
        // while active.
        if self.dialog().is_some() || self.delete_dialog().is_some() {
            return CheckpointsEvent::Consumed;
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
}
