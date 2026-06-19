//! App::dispatch routing for `Action::DismissDialog(id)`
//! and ErrorDialog promotion on LLM provider Error chunks. Introduced: RPC-079.
//!
//! Feature: spec/features/rust-error-notification-status-dialog-wrappers.feature
//!
//! Factored into its own file so `app/dispatch.rs` stays under the
//! 300-LoC ceiling.
//!
//! Two responsibilities:
//!
//! - `try_dispatch_dialog_dismiss` — matches `Action::DismissDialog(id)` and
//!   calls `self.compositor.remove(&id)`. Returns `true` when the
//!   action was consumed; `false` otherwise so the caller can fall
//!   through to the next dispatch helper in `app/dispatch.rs`.
//! - `maybe_push_error_dialog_for_chunk` — when an inbound
//!   `StreamChunk::Error` arrives via `Action::ChunkReceived`, push a
//!   Priority::Critical [`ErrorDialog`] onto the compositor so the
//!   user gets a centred modal alert (the same text also stays in
//!   scrollback per RPC-078 rule 5). Idempotent: if an ErrorDialog is
//!   already on the compositor, the new error is ignored to avoid
//!   stacking modals.
//!
//! This is the back half of the auto-dismiss timer pattern introduced
//! by `NotificationDialog` and `StatusDialog`: those dialogs spawn a
//! `tokio::time::sleep` task that fires `Action::DismissDialog(id)`
//! when the countdown elapses; this dispatch helper turns that
//! action into the corresponding `compositor.remove` call.

use codelet_rpc_types::StreamChunk;

use crate::components::error_dialog::{ErrorDialog, ERROR_DIALOG_ID};
use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-079: handle [`Action::DismissDialog`] by popping the dialog
    /// identified by its stable id off the compositor. Returns `true`
    /// when the action matched; `false` otherwise (so the main
    /// `dispatch.rs` `_` arm can chain into the next try_dispatch_*).
    pub(crate) fn try_dispatch_dialog_dismiss(&mut self, action: &Action) -> bool {
        if let Action::DismissDialog(id) = action {
            let _ = self.compositor.remove(id);
            true
        } else {
            false
        }
    }

    /// RPC-079: when an LLM provider returns a `StreamChunk::Error`,
    /// surface it as a modal `ErrorDialog` so the user is alerted
    /// with the same prominence as a disconnect. The scrollback
    /// `API Error: ...` line still appears per RPC-078 rule 5; this
    /// is purely additive.
    pub(crate) fn maybe_push_error_dialog_for_chunk(&mut self, chunk: &StreamChunk) {
        if let StreamChunk::Error { error } = chunk {
            if !self.compositor.contains(ERROR_DIALOG_ID) {
                let dialog = ErrorDialog::new(format!("API Error: {error}"));
                self.compositor.push(Box::new(dialog));
            }
        }
    }
}
