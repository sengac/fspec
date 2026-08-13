//! RPC-366 — `CheckpointsView` delete key handling + dialog transitions.
//!
//! Feature: spec/features/checkpoint-delete.feature
//!
//! Split out of `mod.rs` (which hosts the three-pane state machine) so
//! every file stays under the 300-line ceiling. Holds the `d`/`a` key
//! handlers, the single yes/no + delete-all typed-confirmation dialog
//! transitions, and the result-folding `on_delete_result` that the App
//! dispatcher calls when a `DeleteCheckpointResult` action arrives.

use crossterm::event::{KeyCode, KeyEvent};

use crate::components::Action;

use super::checkpoint_row::checkpoint_label;
use super::delete_dialog::{DeleteDialog, DeletePhase, DeleteTarget};
use super::{CheckpointsEvent, CheckpointsView};

impl CheckpointsView {
    /// `d`/`D`: open a single-checkpoint delete confirmation for the
    /// selected checkpoint — a no-op when no checkpoints exist.
    pub(super) fn open_delete_single(&mut self) -> CheckpointsEvent {
        let Some(cp) = self.selected_checkpoint_info() else {
            return CheckpointsEvent::Consumed;
        };
        let label = checkpoint_label(cp);
        let (work_unit_id, name) = (cp.work_unit_id.clone(), cp.name.clone());
        self.delete_dialog = Some(DeleteDialog::confirm_single(work_unit_id, name, label));
        CheckpointsEvent::Consumed
    }

    /// `a`/`A`: open the delete-all typed confirmation — a no-op when no
    /// checkpoints exist.
    pub(super) fn open_delete_all(&mut self) -> CheckpointsEvent {
        if self.checkpoints.is_empty() {
            return CheckpointsEvent::Consumed;
        }
        self.delete_dialog = Some(DeleteDialog::confirm_all());
        CheckpointsEvent::Consumed
    }

    /// Route a key while the delete dialog is active. `ConfirmSingle`
    /// awaits y/n; `ConfirmAll` captures the typed phrase and only
    /// dispatches on Enter once it equals `DELETE ALL`; a terminal Error
    /// phase dismisses on any key.
    pub(super) fn handle_delete_dialog_key(&mut self, key: KeyEvent) -> CheckpointsEvent {
        let phase = match self.delete_dialog.as_ref() {
            Some(d) => d.phase.clone(),
            None => return CheckpointsEvent::Consumed,
        };
        match phase {
            DeletePhase::ConfirmSingle => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete_single(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.delete_dialog = None;
                    CheckpointsEvent::Consumed
                }
                _ => CheckpointsEvent::Consumed,
            },
            DeletePhase::ConfirmAll { .. } => match key.code {
                KeyCode::Esc => {
                    self.delete_dialog = None;
                    CheckpointsEvent::Consumed
                }
                KeyCode::Enter => self.confirm_delete_all_if_ready(),
                KeyCode::Backspace => {
                    self.delete_all_input_edit(|input| {
                        input.pop();
                    });
                    CheckpointsEvent::Consumed
                }
                KeyCode::Char(c) => {
                    self.delete_all_input_edit(|input| input.push(c));
                    CheckpointsEvent::Consumed
                }
                _ => CheckpointsEvent::Consumed,
            },
            DeletePhase::Deleting => CheckpointsEvent::Consumed,
            DeletePhase::Error(_) => {
                self.delete_dialog = None;
                CheckpointsEvent::Consumed
            }
        }
    }

    /// Mutate the delete-all typed-input buffer in place (no-op unless the
    /// dialog is in `ConfirmAll`).
    fn delete_all_input_edit<F: FnOnce(&mut String)>(&mut self, edit: F) {
        if let Some(dialog) = self.delete_dialog.as_mut() {
            if let DeletePhase::ConfirmAll { input } = &mut dialog.phase {
                edit(input);
            }
        }
    }

    /// Confirm the single delete: emit `DeleteCheckpoint` and show the
    /// `Deleting` status until the result arrives.
    fn confirm_delete_single(&mut self) -> CheckpointsEvent {
        let target = match self.delete_dialog.as_ref() {
            Some(d) => d.target.clone(),
            None => return CheckpointsEvent::Consumed,
        };
        let DeleteTarget::Single {
            work_unit_id, name, ..
        } = target
        else {
            return CheckpointsEvent::Consumed;
        };
        if let Some(dialog) = self.delete_dialog.as_mut() {
            dialog.phase = DeletePhase::Deleting;
        }
        CheckpointsEvent::Emit(Action::DeleteCheckpoint { work_unit_id, name })
    }

    /// Confirm delete-all only when the typed phrase matches exactly;
    /// otherwise a no-op (the dialog stays open, disabled).
    fn confirm_delete_all_if_ready(&mut self) -> CheckpointsEvent {
        let ready = self
            .delete_dialog
            .as_ref()
            .map(DeleteDialog::all_confirm_ready)
            .unwrap_or(false);
        if !ready {
            return CheckpointsEvent::Consumed;
        }
        if let Some(dialog) = self.delete_dialog.as_mut() {
            dialog.phase = DeletePhase::Deleting;
        }
        CheckpointsEvent::Emit(Action::DeleteAllCheckpoints)
    }

    /// Fold a `DeleteCheckpointResult` into the view: on error show the
    /// message; on success remove the row (or clear all), clamp the
    /// selection, and return the follow-up actions (reload the new
    /// selection or close the view when empty, plus a counts refresh).
    pub fn on_delete_result(
        &mut self,
        work_unit_id: &str,
        name: &str,
        all: bool,
        error: Option<&str>,
    ) -> Vec<Action> {
        if let Some(message) = error {
            if let Some(dialog) = self.delete_dialog.as_mut() {
                dialog.phase = DeletePhase::Error(message.to_string());
            }
            return Vec::new();
        }
        self.delete_dialog = None;
        let mut follow_ups = Vec::new();
        if all {
            self.clear_all_checkpoints();
            follow_ups.push(Action::CloseCheckpointsView);
        } else {
            self.remove_checkpoint(work_unit_id, name);
            if self.checkpoints.is_empty() {
                follow_ups.push(Action::CloseCheckpointsView);
            } else if let Some(cp) = self.selected_checkpoint_info() {
                follow_ups.push(Action::LoadCheckpointFiles {
                    work_unit_id: cp.work_unit_id.clone(),
                    name: cp.name.clone(),
                });
            }
        }
        follow_ups.push(Action::RefreshCheckpointCounts);
        follow_ups
    }

    fn clear_all_checkpoints(&mut self) {
        self.checkpoints.clear();
        self.selected_checkpoint = 0;
        self.checkpoint_scroll = 0;
        self.clear_files();
    }

    /// Remove the checkpoint matching `(work_unit_id, name)`, clamp the
    /// selection, and clear the dependent files/diff so the new selection
    /// reloads cleanly.
    fn remove_checkpoint(&mut self, work_unit_id: &str, name: &str) {
        if let Some(idx) = self
            .checkpoints
            .iter()
            .position(|c| c.work_unit_id == work_unit_id && c.name == name)
        {
            self.checkpoints.remove(idx);
        }
        let max = self.checkpoints.len().saturating_sub(1);
        if self.selected_checkpoint > max {
            self.selected_checkpoint = max;
        }
        self.checkpoint_scroll = self.checkpoint_scroll.min(self.selected_checkpoint);
        self.clear_files();
    }

    /// Number of checkpoints currently loaded (used by delete tests).
    #[cfg(test)]
    pub(super) fn checkpoint_count_total(&self) -> usize {
        self.checkpoints.len()
    }

    /// RPC-366: borrow the active delete dialog, if any. Used by the
    /// renderer (to paint the modal) and by tests.
    pub fn delete_dialog(&self) -> Option<&DeleteDialog> {
        self.delete_dialog.as_ref()
    }
}
