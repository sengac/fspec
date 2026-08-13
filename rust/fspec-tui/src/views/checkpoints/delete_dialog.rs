//! RPC-366 — Checkpoint delete confirmation dialog sub-state.
//!
//! Feature: spec/features/checkpoint-delete.feature
//!
//! A modal sub-state the `CheckpointsView` renders over its three panes
//! and that captures input while active. Modeled on the RPC-365
//! `RestoreDialog`: it owns no transport, only the pending intent + the
//! render via the shared modal renderer.
//!
//! Two flavours: a single-checkpoint delete behind a yes/no `ConfirmSingle`
//! prompt, and a delete-all behind a `ConfirmAll` typed-confirmation that
//! only enables the action once the user types the exact phrase
//! `DELETE ALL`.

/// The exact phrase the user must type to enable the delete-all action.
pub const DELETE_ALL_PHRASE: &str = "DELETE ALL";

/// Which delete the dialog is confirming / running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteTarget {
    /// Delete a single checkpoint `(work_unit_id, name)`; `label` is the
    /// human-facing row text shown in the confirmation copy.
    Single {
        work_unit_id: String,
        name: String,
        label: String,
    },
    /// Delete every checkpoint across all work units.
    All,
}

/// The modal phase. `ConfirmSingle` awaits y/n; `ConfirmAll` captures the
/// typed phrase; `Deleting` is shown while the transport call is in
/// flight; `Error` is terminal until dismissed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePhase {
    ConfirmSingle,
    ConfirmAll { input: String },
    Deleting,
    Error(String),
}

/// The active delete dialog. Absent (`None` on the view) means no modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDialog {
    pub target: DeleteTarget,
    pub phase: DeletePhase,
}

impl DeleteDialog {
    /// Open a single-checkpoint delete confirmation for `(work_unit_id,
    /// name)` labelled `label`.
    pub fn confirm_single(work_unit_id: String, name: String, label: String) -> Self {
        Self {
            target: DeleteTarget::Single {
                work_unit_id,
                name,
                label,
            },
            phase: DeletePhase::ConfirmSingle,
        }
    }

    /// Open a delete-all typed confirmation with an empty input buffer.
    pub fn confirm_all() -> Self {
        Self {
            target: DeleteTarget::All,
            phase: DeletePhase::ConfirmAll {
                input: String::new(),
            },
        }
    }

    /// Whether the delete-all action is currently enabled — true only
    /// when the typed input matches `DELETE ALL` exactly.
    pub fn all_confirm_ready(&self) -> bool {
        matches!(&self.phase, DeletePhase::ConfirmAll { input } if input == DELETE_ALL_PHRASE)
    }

    /// The dialog title for the current phase.
    pub fn title(&self) -> &'static str {
        match self.phase {
            DeletePhase::ConfirmSingle => "Delete Checkpoint",
            DeletePhase::ConfirmAll { .. } => "Delete ALL Checkpoints",
            DeletePhase::Deleting => "Deleting…",
            DeletePhase::Error(_) => "Delete Failed",
        }
    }

    /// The body lines rendered inside the dialog.
    pub fn body_lines(&self) -> Vec<String> {
        match &self.phase {
            DeletePhase::ConfirmSingle => {
                let label = match &self.target {
                    DeleteTarget::Single { label, .. } => label.as_str(),
                    DeleteTarget::All => "",
                };
                vec![
                    format!("Delete checkpoint '{label}'? This cannot be undone."),
                    String::new(),
                    "y: confirm   n: cancel".to_string(),
                ]
            }
            DeletePhase::ConfirmAll { input } => {
                let enabled = input == DELETE_ALL_PHRASE;
                let hint = if enabled {
                    "enter: confirm   esc: cancel".to_string()
                } else {
                    "enter: (disabled)   esc: cancel".to_string()
                };
                vec![
                    "Delete ALL checkpoints? This cannot be undone.".to_string(),
                    format!("Type {DELETE_ALL_PHRASE} to confirm:"),
                    format!("> {input}"),
                    String::new(),
                    hint,
                ]
            }
            DeletePhase::Deleting => vec!["Deleting…".to_string()],
            DeletePhase::Error(message) => vec![
                format!("Error: {message}"),
                String::new(),
                "any key: close".to_string(),
            ],
        }
    }
}
