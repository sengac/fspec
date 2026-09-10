//! App::dispatch routing for the WT-005 `/worktrees` flow:
//! SessionWorktreesDialog open + Prune / Cancel actions.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper mirrors the established RPC-057 /
//! RPC-054 patterns: spawn a tokio task that awaits the backend
//! round-trip, route the response back through the action bus.
//!
//! Flow:
//!   1. `handle_open_session_worktrees_dialog` — entry point from the
//!      slash command palette. Spawns `backend.list_session_worktrees()`
//!      and routes the response through `Action::SessionWorktreesLoaded`.
//!   2. `handle_session_worktrees_loaded` — pushes the
//!      SessionWorktreesDialog onto the compositor seeded with the rows.
//!   3. `handle_prune_leaked_worktrees` — spawns
//!      `backend.prune_orphaned_worktrees()` and routes the outcome into
//!      the matching scrollback notice.
//!   4. `handle_cancel_worktrees_dialog` — pops the dialog with no
//!      backend interaction.

use tokio::task::JoinHandle;

use crate::components::Action;
use crate::views::agent::session_worktrees_dialog::{
    SessionWorktreesDialog, SESSION_WORKTREES_DIALOG_ID,
};

use super::state::App;

impl App {
    /// WT-005: /worktrees slash command entry point. Spawns
    /// `backend.list_session_worktrees()` — the response lands as
    /// `Action::SessionWorktreesLoaded`. The originator session is
    /// captured up front so a list failure lands on the originating
    /// session even after a focus switch.
    pub(crate) fn handle_open_session_worktrees_dialog(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend.list_session_worktrees().await {
                Ok(rows) => {
                    let _ = action_tx.send(Action::SessionWorktreesLoaded(rows));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session_id,
                        format!("[error] /worktrees: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// WT-005: fold a `backend.list_session_worktrees()` response into a
    /// fresh SessionWorktreesDialog on the compositor. Idempotent —
    /// pushing twice replaces the existing layer.
    pub(crate) fn handle_session_worktrees_loaded(
        &mut self,
        rows: Vec<codelet_rpc_types::SessionWorktreeInfo>,
    ) {
        if self.compositor.contains(SESSION_WORKTREES_DIALOG_ID) {
            let _ = self.compositor.remove(SESSION_WORKTREES_DIALOG_ID);
        }
        let dialog = SessionWorktreesDialog::new(rows)
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// WT-005: route a `PruneLeakedWorktrees` action — spawns
    /// `backend.prune_orphaned_worktrees()` and routes the outcome into
    /// the matching scrollback notice:
    ///   - Ok with N>0  → `[worktrees] pruned N leaked worktree(s)`
    ///   - Ok with N==0 → `[worktrees] no leaked worktrees to prune`
    ///   - Err          → `[error] /worktrees prune: {e}`
    pub(crate) fn handle_prune_leaked_worktrees(&mut self) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let text = match backend.prune_orphaned_worktrees().await {
                Ok(ids) if ids.is_empty() => {
                    "[worktrees] no leaked worktrees to prune".to_string()
                }
                Ok(ids) => format!("[worktrees] pruned {} leaked worktree(s)", ids.len()),
                Err(e) => format!("[error] /worktrees prune: {e}"),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    /// WT-005: pop the SessionWorktreesDialog without firing any backend
    /// call.
    pub(crate) fn handle_cancel_worktrees_dialog(&mut self) {
        let _ = self.compositor.remove(SESSION_WORKTREES_DIALOG_ID);
    }

    /// Route the WT-005 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_worktrees(&mut self, action: &Action) -> bool {
        match action {
            Action::SessionWorktreesLoaded(rows) => {
                self.handle_session_worktrees_loaded(rows.clone());
            }
            Action::PruneLeakedWorktrees => {
                self.handle_prune_leaked_worktrees();
            }
            Action::CancelWorktreesDialog => {
                self.handle_cancel_worktrees_dialog();
            }
            _ => return false,
        }
        true
    }
}
