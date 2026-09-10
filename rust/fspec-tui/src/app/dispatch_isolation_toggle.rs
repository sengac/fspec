//! App::dispatch routing for the `/isolation` state toggle (WT-009).
//!
//! Feature:
//! spec/features/isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui.feature
//!
//! `/isolation` is a real state toggle on the current session:
//!
//! 1. No active session → silent no-op (no dialog, no backend call).
//! 2. Isolated session (store shows `is_isolated = true` with a worktree
//!    path) → spawn `backend.detach_session_worktree(session_id)`; on Ok
//!    emit a `[isolation] detached from worktree` notice, on Err emit
//!    `[error] /isolation: ...`. The store's isolation state flips off
//!    via the `IsolationStateChange(false, None)` chunk the backend
//!    emits; the session stays open.
//! 3. Non-isolated session with a live worktree row the store does not
//!    track (e.g. a session resumed after a restart) → emit the
//!    `session still isolated — use /merge-worktree` notice WITHOUT a
//!    backend detach call (the session's store state says non-isolated;
//!    only the worktree listing disagrees).
//! 4. Everything else (non-isolated, no tracked worktree) → the
//!    pre-existing RPC-060 path: open the CreateSessionDialog
//!    preselecting `Yes - Isolated`.
//!
//! The list probe is best-effort: a `list_session_worktrees` error is
//! swallowed (the toggle still degrades to the RPC-060 dialog path —
//! never an error notice for a command that is still usable).

use tokio::task::JoinHandle;

use crate::components::create_session_dialog::CreateSessionOption;
use crate::components::Action;

use super::state::App;

impl App {
    /// WT-009: `/isolation` toggle entry point (called from
    /// `handle_slash_command`'s `SlashCommandAction::Isolation` arm).
    /// With no current session it is a silent no-op. Otherwise the
    /// backend's worktree listing decides the route per the file
    /// header: tracked-isolated → detach; untracked live worktree →
    /// still-isolated notice; otherwise → the RPC-060 dialog path.
    pub(crate) fn handle_slash_isolation_toggle(&mut self) {
        // @no-active-session: silent no-op (no dialog, no backend call).
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }

        // Read the store's isolation state for the current session.
        let store_isolated = self
            .agent_view_store
            .isolation_state_for(&session_id)
            .is_some_and(|st| st.is_isolated);

        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            // Best-effort probe: a listing failure degrades to the
            // RPC-060 dialog path (the toggle is still usable).
            let tracked_rows = match backend.list_session_worktrees().await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::debug!("list_session_worktrees probe failed: {e}");
                    Vec::new()
                }
            };
            let has_live_worktree = tracked_rows
                .iter()
                .any(|w| w.session_id == sid);

            if store_isolated {
                // Path 2: tracked isolated session → detach.
                match backend.detach_session_worktree(sid.clone()).await {
                    Ok(()) => {
                        let _ = action_tx.send(Action::EmitSessionNotice(
                            sid.clone(),
                            "[isolation] detached from worktree".to_string(),
                        ));
                    }
                    Err(e) => {
                        let _ = action_tx.send(Action::EmitSessionNotice(
                            sid,
                            format!("[error] /isolation: {e}"),
                        ));
                    }
                }
            } else if has_live_worktree {
                // Path 3: the worktree listing shows a live worktree for
                // this session but the store does not track it as
                // isolated — do NOT detach (the backend would error on
                // a session it doesn't consider isolated); point the
                // user at /merge-worktree instead.
                let _ = action_tx.send(Action::EmitSessionNotice(
                    sid,
                    "session still isolated — use /merge-worktree".to_string(),
                ));
            } else {
                // Path 4: non-isolated, no live worktree → the RPC-060
                // create-session dialog preselecting Isolated.
                let _ = action_tx.send(Action::OpenCreateSessionDialog {
                    preselect: Some(CreateSessionOption::Isolated),
                });
            }
        });
        self.pending_tasks.push(handle);
    }
}
