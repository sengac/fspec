//! RPC-364 — `CheckpointsView` pane focus + selection navigation.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! Split out of `mod.rs` (RPC-365) so every file stays under the
//! 300-line ceiling. Holds the pure scroll/selection math: `cycle_pane`,
//! `scroll_focused`, `move_checkpoint_selection`, `move_file_selection`,
//! `apply_diff_scroll`, and the page-step helpers. Reuses the shared
//! `scroll_viewport::ensure_visible`.

use crate::components::scroll_viewport::ensure_visible;

use super::{CheckpointsEvent, CheckpointsView, Pane};

impl CheckpointsView {
    /// Cycle focus across the three panes. `forward` advances
    /// Checkpoints→Files→Diff→Checkpoints; otherwise the reverse.
    pub(super) fn cycle_pane(&mut self, forward: bool) {
        self.focused_pane = match (self.focused_pane, forward) {
            (Pane::Checkpoints, true) => Pane::Files,
            (Pane::Files, true) => Pane::Diff,
            (Pane::Diff, true) => Pane::Checkpoints,
            (Pane::Checkpoints, false) => Pane::Diff,
            (Pane::Files, false) => Pane::Checkpoints,
            (Pane::Diff, false) => Pane::Files,
        };
    }

    pub(super) fn scroll_focused(&mut self, delta: i32) -> CheckpointsEvent {
        match self.focused_pane {
            Pane::Diff => {
                self.apply_diff_scroll(delta);
                CheckpointsEvent::Consumed
            }
            Pane::Files => self.move_file_selection(delta),
            Pane::Checkpoints => self.move_checkpoint_selection(delta),
        }
    }

    /// Move the checkpoint selection, clamped, and request a files reload
    /// for the new selection.
    pub(super) fn move_checkpoint_selection(&mut self, delta: i32) -> CheckpointsEvent {
        if self.checkpoints.is_empty() {
            return CheckpointsEvent::Consumed;
        }
        let max = self.checkpoints.len().saturating_sub(1);
        let proposed = (self.selected_checkpoint as i64).saturating_add(delta as i64);
        let clamped = proposed.clamp(0, max as i64) as usize;
        if clamped == self.selected_checkpoint {
            return CheckpointsEvent::Consumed;
        }
        self.selected_checkpoint = clamped;
        let visible = self
            .last_checkpoints_rect
            .map(|r| r.height as usize)
            .unwrap_or(0);
        ensure_visible(
            &mut self.checkpoint_scroll,
            self.selected_checkpoint,
            visible,
            self.checkpoints.len(),
        );
        self.clear_files();
        match self.selected_checkpoint_info() {
            Some(c) => CheckpointsEvent::Emit(crate::components::Action::LoadCheckpointFiles {
                work_unit_id: c.work_unit_id.clone(),
                name: c.name.clone(),
            }),
            None => CheckpointsEvent::Consumed,
        }
    }

    /// Move the file selection, clamped, and request a diff reload.
    pub(super) fn move_file_selection(&mut self, delta: i32) -> CheckpointsEvent {
        if self.files.is_empty() {
            return CheckpointsEvent::Consumed;
        }
        let max = self.files.len().saturating_sub(1);
        let proposed = (self.selected_file as i64).saturating_add(delta as i64);
        let clamped = proposed.clamp(0, max as i64) as usize;
        if clamped == self.selected_file {
            return CheckpointsEvent::Consumed;
        }
        self.selected_file = clamped;
        self.diff_scroll = 0;
        let visible = self.last_files_rect.map(|r| r.height as usize).unwrap_or(0);
        ensure_visible(
            &mut self.file_scroll,
            self.selected_file,
            visible,
            self.files.len(),
        );
        match (self.selected_checkpoint_info(), self.selected_file_path()) {
            (Some(c), Some(path)) => {
                CheckpointsEvent::Emit(crate::components::Action::LoadCheckpointFileDiff {
                    work_unit_id: c.work_unit_id.clone(),
                    name: c.name.clone(),
                    path,
                })
            }
            _ => CheckpointsEvent::Consumed,
        }
    }

    pub(super) fn apply_diff_scroll(&mut self, delta: i32) {
        let max = self.max_diff_scroll();
        let proposed = (self.diff_scroll as i64).saturating_add(delta as i64);
        self.diff_scroll = proposed.clamp(0, max as i64) as usize;
    }

    fn max_diff_scroll(&self) -> usize {
        let viewport = self.last_diff_rect.map(|r| r.height as usize).unwrap_or(0);
        let len = self.diff_lines.len();
        if viewport == 0 {
            return len.saturating_sub(1);
        }
        len.saturating_sub(viewport)
    }

    pub(super) fn page_step(&self) -> i32 {
        let h = match self.focused_pane {
            Pane::Diff => self.last_diff_rect.map(|r| r.height).unwrap_or(1),
            Pane::Files => self.last_files_rect.map(|r| r.height).unwrap_or(1),
            Pane::Checkpoints => self.last_checkpoints_rect.map(|r| r.height).unwrap_or(1),
        };
        (h.max(1)) as i32
    }
}
