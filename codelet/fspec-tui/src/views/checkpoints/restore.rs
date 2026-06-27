//! RPC-365 — `CheckpointsView` restore key handling + dialog transitions.
//!
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Split out of `mod.rs` (which hosts the three-pane state machine) so
//! every file stays under the 300-line ceiling. Holds the `r`/`t` key
//! handlers, the confirm/cancel dialog transitions, and the
//! result-folding `on_restore_result` that the App dispatcher calls when
//! a `RestoreCheckpointResult` action arrives.

use crossterm::event::{KeyCode, KeyEvent};

use crate::components::Action;

use super::dialog::{DialogPhase, RestoreDialog, RestoreTarget};
use super::{CheckpointsEvent, CheckpointsView, Pane};

impl CheckpointsView {
    /// `r`/`R`: open a single-file restore confirmation — but only when
    /// the Files pane is focused AND files exist; otherwise a no-op.
    pub(super) fn open_restore_single(&mut self) -> CheckpointsEvent {
        if self.focused_pane() != Pane::Files {
            return CheckpointsEvent::Consumed;
        }
        let (Some(cp), Some(path)) = (self.selected_checkpoint_info(), self.selected_file_path())
        else {
            return CheckpointsEvent::Consumed;
        };
        self.restore_dialog = Some(RestoreDialog::confirm(RestoreTarget::Single {
            work_unit_id: cp.work_unit_id.clone(),
            name: cp.name.clone(),
            path,
        }));
        CheckpointsEvent::Consumed
    }

    /// `t`/`T`: open a restore-all confirmation for the selected
    /// checkpoint — a no-op when no checkpoints exist.
    pub(super) fn open_restore_all(&mut self) -> CheckpointsEvent {
        let Some(cp) = self.selected_checkpoint_info() else {
            return CheckpointsEvent::Consumed;
        };
        self.restore_dialog = Some(RestoreDialog::confirm(RestoreTarget::All {
            work_unit_id: cp.work_unit_id.clone(),
            name: cp.name.clone(),
            file_count: self.file_count(),
        }));
        CheckpointsEvent::Consumed
    }

    /// Route a key while the restore dialog is active. On `Confirm`,
    /// `y` dispatches the matching restore Action (and flips to
    /// `Restoring`); `n`/Esc cancels. On a terminal phase, any key
    /// dismisses the dialog.
    pub(super) fn handle_dialog_key(&mut self, key: KeyEvent) -> CheckpointsEvent {
        let phase = match self.restore_dialog.as_ref() {
            Some(d) => d.phase.clone(),
            None => return CheckpointsEvent::Consumed,
        };
        match phase {
            DialogPhase::Confirm => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_restore(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.restore_dialog = None;
                    CheckpointsEvent::Consumed
                }
                _ => CheckpointsEvent::Consumed,
            },
            DialogPhase::Restoring => CheckpointsEvent::Consumed,
            DialogPhase::Complete | DialogPhase::Error(_) => {
                self.restore_dialog = None;
                CheckpointsEvent::Consumed
            }
        }
    }

    /// Confirm: emit the restore Action and move the dialog to
    /// `Restoring` so the modal shows progress until the result arrives.
    fn confirm_restore(&mut self) -> CheckpointsEvent {
        let target = match self.restore_dialog.as_ref() {
            Some(d) => d.target.clone(),
            None => return CheckpointsEvent::Consumed,
        };
        let action = match &target {
            RestoreTarget::Single {
                work_unit_id,
                name,
                path,
            } => Action::RestoreCheckpointFile {
                work_unit_id: work_unit_id.clone(),
                name: name.clone(),
                path: path.clone(),
            },
            RestoreTarget::All {
                work_unit_id, name, ..
            } => Action::RestoreCheckpointAll {
                work_unit_id: work_unit_id.clone(),
                name: name.clone(),
            },
        };
        if let Some(dialog) = self.restore_dialog.as_mut() {
            dialog.phase = DialogPhase::Restoring;
        }
        CheckpointsEvent::Emit(action)
    }

    /// Fold a `RestoreCheckpointResult` into the dialog: drive it to
    /// `Complete` (success) or `Error` (failure), and return the
    /// follow-up actions — a single-file success reloads that file's diff
    /// and any success refreshes the board counts.
    pub fn on_restore_result(
        &mut self,
        work_unit_id: &str,
        name: &str,
        path: Option<&str>,
        error: Option<&str>,
    ) -> Vec<Action> {
        match error {
            Some(message) => {
                if let Some(dialog) = self.restore_dialog.as_mut() {
                    dialog.phase = DialogPhase::Error(message.to_string());
                }
                Vec::new()
            }
            None => {
                if let Some(dialog) = self.restore_dialog.as_mut() {
                    dialog.phase = DialogPhase::Complete;
                }
                let mut follow_ups = Vec::new();
                if let Some(path) = path {
                    follow_ups.push(Action::LoadCheckpointFileDiff {
                        work_unit_id: work_unit_id.to_string(),
                        name: name.to_string(),
                        path: path.to_string(),
                    });
                }
                follow_ups.push(Action::RefreshCheckpointCounts);
                follow_ups
            }
        }
    }
}
