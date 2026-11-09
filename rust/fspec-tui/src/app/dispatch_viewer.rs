//! RPC-373 — App::dispatch routing for the board `D` key (open FOUNDATION.md).
//!
//! Feature: spec/features/rust-board-open-foundation.feature
//!
//! Factored into its own file (like `dispatch_changed_files.rs`) to keep
//! `app/dispatch.rs` under the 300-LoC ceiling. The board's `D`/`d` handler
//! only emits `Action::OpenFoundation`; TUI-120 opens FOUNDATION.md of the
//! selected workspace, read from the daemon
//! (spec/features/foundation-viewer-and-fspec-runner-follow-the-selected-workspace.feature).

use crate::components::attachment_picker_dialog::{
    AttachmentPickerDialog, ATTACHMENT_PICKER_DIALOG_ID,
};
use crate::components::Action;

use super::state::App;

/// TUI-120: the foundation document, relative to a workspace.
const FOUNDATION_PATH: &str = "spec/FOUNDATION.md";

impl App {
    /// Build the attachment viewer URL for a given server port + relative
    /// path. Each `/`-separated segment is percent-encoded (so spaces and
    /// unicode are URL-safe — matching the TS `encodeURI(attachment)`) then
    /// rejoined with `/`. Pure + unit-testable; carries no side effects.
    pub fn attachment_url(port: u16, path: &str) -> String {
        let encoded = path
            .split('/')
            .map(|seg| urlencoding::encode(seg).into_owned())
            .collect::<Vec<_>>()
            .join("/");
        format!("http://127.0.0.1:{port}/view/{encoded}")
    }

    /// Handle `Action::OpenFoundation`: TUI-120 — read spec/FOUNDATION.md
    /// of the board's selected workspace from the daemon and open it in the
    /// viewer. Before the workspace list loads, or on any failure, the reason
    /// is shown.
    pub(crate) fn handle_open_foundation(&mut self) {
        let Some(workspace) = self.selected_workspace_path() else {
            self.dispatch(Action::SessionOperationFailed(
                "FOUNDATION.md cannot be opened: the workspace list has not loaded, so no workspace is selected"
                    .to_owned(),
            ));
            return;
        };
        self.spawn_workspace_file_open(
            std::path::PathBuf::from(workspace),
            FOUNDATION_PATH,
            |_, message| {
                Action::SessionOperationFailed(format!(
                    "FOUNDATION.md could not be opened: {message}"
                ))
            },
        );
    }

    /// Handle `Action::OpenAttachmentPicker`: push an [`AttachmentPickerDialog`]
    /// built from the selected work unit's attachments onto the compositor at
    /// Priority::Foreground. Idempotent on dialog id. TUI-129: with no
    /// selected work unit, or one without attachments, a notice says so.
    pub(crate) fn handle_open_attachment_picker(&mut self) {
        if self.compositor.contains(ATTACHMENT_PICKER_DIALOG_ID) {
            return;
        }
        let Some((id, attachments)) = self
            .board_store
            .selected_work_unit()
            .map(|u| (u.id.clone(), u.attachments.clone()))
        else {
            self.dispatch(Action::SessionOperationFailed(
                "No card is selected, so there are no attachments to open".to_owned(),
            ));
            return;
        };
        if attachments.is_empty() {
            self.dispatch(Action::SessionOperationFailed(format!(
                "No attachments on {id}"
            )));
            return;
        }
        let dialog =
            AttachmentPickerDialog::new(attachments).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// Handle `Action::OpenAttachment(path)`: BUG-469 — read the attachment
    /// from the daemon for the board's selected workspace and open it in the
    /// viewer (the TUI-120 path). Before the workspace list loads, or on any
    /// failure, the reason is shown.
    pub(crate) fn handle_open_attachment(&mut self, path: String) {
        let Some(workspace) = self.selected_workspace_path() else {
            self.dispatch(Action::SessionOperationFailed(format!(
                "The attachment {path} cannot be opened: the workspace list has not loaded, so no workspace is selected"
            )));
            return;
        };
        self.spawn_workspace_file_open(
            std::path::PathBuf::from(workspace),
            &path,
            |path, message| {
                Action::SessionOperationFailed(format!(
                    "The attachment {path} could not be opened: {message}"
                ))
            },
        );
    }

    /// Route the RPC-373/RPC-374 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_viewer(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenFoundation => {
                self.handle_open_foundation();
            }
            Action::OpenAttachmentPicker => {
                self.handle_open_attachment_picker();
            }
            Action::OpenAttachment(path) => {
                self.handle_open_attachment(path.clone());
            }
            _ => return false,
        }
        true
    }
}
