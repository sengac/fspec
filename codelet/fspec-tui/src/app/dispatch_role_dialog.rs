//! App::dispatch routing for the `/role` slash command's
//! dialog-open path. Introduced: RPC-063.
//!
//! Feature: spec/features/role-slash-command-end-to-end-ui-dialog.feature
//! Feature: spec/features/role-dialog-component.feature
//!
//! Factored into its own file so `app/dispatch_slash_commands.rs`
//! AND `app/dispatch.rs` both stay under the 300-LoC ceiling.
//!
//! One responsibility:
//!
//! - `handle_open_role_dialog` — read the current role from
//!   `AgentViewStore::role_for(current_session)` (no backend
//!   round-trip — the store is already kept in sync by RPC-022's
//!   `spawn_get_session_role` + `Action::SessionRoleLoaded` path) and
//!   push a fresh `RoleDialog` at `Priority::Foreground`. Idempotent
//!   on dialog-id collision (matches existing dialog open helpers).

use crate::components::role_dialog::{RoleDialog, ROLE_DIALOG_ID};

use super::state::App;

impl App {
    /// RPC-063: push a fresh `RoleDialog` onto the Compositor seeded
    /// with the current session's role text. Silent no-op when there
    /// is no active session OR when a `RoleDialog` is already mounted
    /// (idempotency: matches `handle_open_model_dialog` /
    /// `handle_open_thinking_dialog` semantics).
    pub(crate) fn handle_open_role_dialog(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if self.compositor.contains(ROLE_DIALOG_ID) {
            return;
        }
        let seed = self
            .agent_view_store
            .role_for(&session_id)
            .map(str::to_string);
        let dialog = RoleDialog::new(session_id, seed).with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }
}
