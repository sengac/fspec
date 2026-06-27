//! RPC-365 — Checkpoint restore confirmation + status dialog sub-state.
//!
//! Feature: spec/features/checkpoint-restore.feature
//!
//! A modal sub-state the `CheckpointsView` renders over its three panes
//! and that captures input while active. Modeled on
//! `components/disconnect_dialog.rs`: it owns no transport, only the
//! pending intent + a render via the shared `dialog_theme` renderer.
//!
//! Lifecycle: a restore key (`r`/`t`) opens a `Confirm` dialog naming the
//! target. `y` transitions to `Restoring` (and the view emits the
//! matching `Action::RestoreCheckpoint*`); `n`/Esc cancels with no call.
//! A `RestoreCheckpointResult` action then drives `Restoring → Complete`
//! or `Restoring → Error(message)`.

/// Which restore the dialog is confirming / running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreTarget {
    /// Restore a single file `path` of `(work_unit_id, name)`.
    Single {
        work_unit_id: String,
        name: String,
        path: String,
    },
    /// Restore all `file_count` files of `(work_unit_id, name)`.
    All {
        work_unit_id: String,
        name: String,
        file_count: usize,
    },
}

/// The modal phase. `Confirm` awaits y/n; `Restoring` is shown while the
/// transport call is in flight; `Complete`/`Error` are terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogPhase {
    Confirm,
    Restoring,
    Complete,
    Error(String),
}

/// The active restore dialog. Absent (`None` on the view) means no modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDialog {
    pub target: RestoreTarget,
    pub phase: DialogPhase,
}

impl RestoreDialog {
    /// Open a fresh confirmation dialog for `target`.
    pub fn confirm(target: RestoreTarget) -> Self {
        Self {
            target,
            phase: DialogPhase::Confirm,
        }
    }

    /// The dialog title for the current phase.
    pub fn title(&self) -> &'static str {
        match self.phase {
            DialogPhase::Confirm => "Restore Checkpoint",
            DialogPhase::Restoring => "Restoring…",
            DialogPhase::Complete => "Restore Complete",
            DialogPhase::Error(_) => "Restore Failed",
        }
    }

    /// The body lines rendered inside the dialog.
    pub fn body_lines(&self) -> Vec<String> {
        match &self.phase {
            DialogPhase::Confirm => {
                let warning = match &self.target {
                    RestoreTarget::Single { path, .. } => {
                        format!("Restore {path}? This overwrites the working copy.")
                    }
                    RestoreTarget::All { file_count, .. } => {
                        format!("Restore ALL {file_count} files? This overwrites the working copy.")
                    }
                };
                vec![warning, String::new(), "y: confirm   n: cancel".to_string()]
            }
            DialogPhase::Restoring => vec!["Restoring…".to_string()],
            DialogPhase::Complete => vec![
                "Restore complete.".to_string(),
                String::new(),
                "any key: close".to_string(),
            ],
            DialogPhase::Error(message) => vec![
                format!("Error: {message}"),
                String::new(),
                "any key: close".to_string(),
            ],
        }
    }
}
