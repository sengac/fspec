//! RPC-373 — App::dispatch routing for the board `D` key (open FOUNDATION.md).
//!
//! Feature: spec/features/rust-board-open-foundation.feature
//!
//! Factored into its own file (like `dispatch_changed_files.rs`) to keep
//! `app/dispatch.rs` under the 300-LoC ceiling. The board's `D`/`d` handler
//! only emits `Action::OpenFoundation`; this helper reads the viewer port set
//! at bootstrap, builds the FOUNDATION.md viewer URL, and launches the default
//! browser. The pure `foundation_url` / `foundation_target` seam is unit-tested
//! so no real browser launches in tests — the `open::that` call sits only in
//! the `Some(target)` branch.

use crate::components::attachment_picker_dialog::{
    AttachmentPickerDialog, ATTACHMENT_PICKER_DIALOG_ID,
};
use crate::components::Action;

use super::state::App;

impl App {
    /// Build the FOUNDATION.md viewer URL for a given server port. Pure +
    /// unit-testable; carries no side effects.
    pub fn foundation_url(port: u16) -> String {
        format!("http://127.0.0.1:{port}/view/spec/FOUNDATION.md")
    }

    /// Resolve the FOUNDATION.md launch target: `Some(url)` when the viewer
    /// server is running (a port is known), else `None` (safe no-op).
    pub fn foundation_target(&self) -> Option<String> {
        self.viewer_port.map(Self::foundation_url)
    }

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

    /// Resolve the attachment launch target: `Some(url)` when the viewer
    /// server is running (a port is known), else `None` (safe no-op).
    pub fn attachment_target(&self, path: &str) -> Option<String> {
        self.viewer_port.map(|p| Self::attachment_url(p, path))
    }

    /// Handle `Action::OpenFoundation`: launch the FOUNDATION.md viewer URL in
    /// the user's default browser when the viewer server is available; a
    /// `tracing::warn!` no-op otherwise. The `open::that` call is spawned so it
    /// never blocks the dispatch loop, and never panics.
    pub(crate) fn handle_open_foundation(&mut self) {
        let Some(url) = self.foundation_target() else {
            tracing::warn!("OpenFoundation: viewer server unavailable; ignoring D key");
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let handle = tokio::task::spawn_blocking(move || {
            if let Err(err) = open::that(&url) {
                tracing::warn!(error = %err, "OpenFoundation: failed to launch browser");
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Handle `Action::OpenAttachmentPicker`: push an [`AttachmentPickerDialog`]
    /// built from the selected work unit's attachments onto the compositor at
    /// Priority::Foreground. Idempotent on dialog id, and a no-op when there is
    /// no selected work unit or it has no attachments.
    pub(crate) fn handle_open_attachment_picker(&mut self) {
        if self.compositor.contains(ATTACHMENT_PICKER_DIALOG_ID) {
            return;
        }
        let Some(attachments) = self
            .board_store
            .selected_work_unit()
            .map(|u| u.attachments.clone())
        else {
            return;
        };
        if attachments.is_empty() {
            return;
        }
        let dialog =
            AttachmentPickerDialog::new(attachments).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// Handle `Action::OpenAttachment(path)`: launch the attachment's viewer
    /// URL in the user's default browser when the viewer server is available;
    /// a `tracing::warn!` no-op otherwise. The `open::that` call is spawned so
    /// it never blocks the dispatch loop, and never panics.
    pub(crate) fn handle_open_attachment(&mut self, path: String) {
        let Some(url) = self.attachment_target(&path) else {
            tracing::warn!("OpenAttachment: viewer server unavailable; ignoring selection");
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let handle = tokio::task::spawn_blocking(move || {
            if let Err(err) = open::that(&url) {
                tracing::warn!(error = %err, "OpenAttachment: failed to launch browser");
            }
        });
        self.pending_tasks.push(handle);
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

    /// RPC-373 test-only seam: set the viewer port directly so dispatch/URL
    /// tests can exercise `foundation_target` without starting a real server.
    pub fn set_viewer_port_for_test(&mut self, port: Option<u16>) {
        self.viewer_port = port;
    }
}
