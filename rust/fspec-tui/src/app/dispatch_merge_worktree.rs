//! App::dispatch routing for the /merge-worktree flow. Introduced: RPC-057.
//!
//! Factored into its own file to keep `app/dispatch.rs` under the
//! 300-LoC ceiling. Each helper here mirrors the established RPC-054 /
//! RPC-055 / RPC-056 patterns: spawn a tokio task that awaits the
//! backend round-trip, route the response back through the action bus.
//!
//! Flow:
//!   1. `handle_slash_merge_worktree` — entry point from the slash
//!      command palette. With no current session it's a silent no-op.
//!      Otherwise spawns `backend.inspect_session_changes(session_id)`
//!      and routes the response through `Action::InspectChangesLoaded`.
//!   2. `handle_inspect_changes_loaded` — folds the inspect response
//!      into either an `OpenMergeConfirmDialog` push or a
//!      `[merge] nothing to merge` notice when every count is zero.
//!   3. `handle_merge_confirmed` — pops the dialog (synchronously),
//!      spawns `backend.merge_session_worktree(session_id,
//!      MergeStrategy::FastForward)` and routes the
//!      `MergeOutcome.status` into the right scrollback notice (Success
//!      → `[merge] success ...`, NoChanges → `[merge] nothing to merge`,
//!      Conflict → `Action::SeedPendingInput` carrying the LLM
//!      conflict-context message, Err → `[error] /merge-worktree: ...`).
//!   4. `handle_discard_confirmed` — pops the dialog, spawns
//!      `backend.discard_session_worktree(session_id)`, emits a
//!      `[discard]` notice on Ok or `[error]` on failure.
//!   5. `handle_cancel_merge_dialog` — pops the dialog with no
//!      backend interaction.

use tokio::task::JoinHandle;

use codelet_rpc_types::{
    MergeOutcome, MergeStatus, MergeStrategy, SessionChangesSummary, SessionId,
};

use crate::components::Action;
use crate::views::agent::merge_confirm_dialog::{MergeConfirmDialog, MERGE_CONFIRM_DIALOG_ID};

use super::state::App;

impl App {
    /// RPC-057: /merge-worktree slash command entry point. With no
    /// current session this is a silent no-op (matches /clear, /compact,
    /// /debug). Otherwise spawn `backend.inspect_session_changes(...)`
    /// — the response lands as `Action::InspectChangesLoaded`.
    pub(crate) fn handle_slash_merge_worktree(&mut self) {
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
            match backend.inspect_session_changes(sid_for_task.clone()).await {
                Ok(summary) => {
                    let _ = action_tx.send(Action::InspectChangesLoaded {
                        session_id: sid_for_task,
                        summary,
                    });
                }
                Err(e) => {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        sid_for_task,
                        format!("[error] /merge-worktree: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-057: fold a `backend.inspect_session_changes()` response —
    /// zero-change result emits a `[merge] nothing to merge` notice;
    /// non-zero result re-dispatches `Action::OpenMergeConfirmDialog`
    /// so the standard compositor-push path runs.
    pub(crate) fn handle_inspect_changes_loaded(
        &mut self,
        session_id: SessionId,
        summary: SessionChangesSummary,
    ) {
        let total = summary.files_changed
            + summary.insertions
            + summary.deletions
            + summary.commits.len() as u32;
        if total == 0 {
            self.handle_emit_session_notice(&session_id, "[merge] nothing to merge".to_string());
            return;
        }
        let _ = self.action_tx.send(Action::OpenMergeConfirmDialog {
            session_id,
            summary,
        });
    }

    /// RPC-057: push the MergeConfirmDialog onto the compositor.
    /// Idempotent — pushing twice replaces the existing layer.
    pub(crate) fn handle_open_merge_confirm_dialog(
        &mut self,
        session_id: SessionId,
        summary: SessionChangesSummary,
    ) {
        if self.compositor.contains(MERGE_CONFIRM_DIALOG_ID) {
            let _ = self.compositor.remove(MERGE_CONFIRM_DIALOG_ID);
        }
        let dialog = MergeConfirmDialog::new(session_id, summary);
        self.compositor.push(Box::new(dialog));
    }

    /// RPC-057: route a `MergeConfirmed` action — pops the dialog
    /// before the backend round-trip completes (best-effort UX),
    /// spawns `backend.merge_session_worktree(...)`, and routes the
    /// response into the right scrollback notice (or SeedPendingInput
    /// on Conflict).
    pub(crate) fn handle_merge_confirmed(&mut self, session_id: SessionId) {
        let _ = self.compositor.remove(MERGE_CONFIRM_DIALOG_ID);
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            match backend
                .merge_session_worktree(sid_for_task.clone(), MergeStrategy::FastForward)
                .await
            {
                Ok(outcome) => route_merge_outcome(&action_tx, sid_for_task, outcome),
                Err(e) => {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        sid_for_task,
                        format!("[error] /merge-worktree: {e}"),
                    ));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-057: route a `DiscardConfirmed` action — pops the dialog
    /// before the round-trip completes, spawns
    /// `backend.discard_session_worktree(...)`, emits the matching
    /// scrollback notice.
    pub(crate) fn handle_discard_confirmed(&mut self, session_id: SessionId) {
        let _ = self.compositor.remove(MERGE_CONFIRM_DIALOG_ID);
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let sid_for_task = session_id;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            let text = match backend.discard_session_worktree(sid_for_task.clone()).await {
                Ok(()) => "[discard] worktree discarded".to_string(),
                Err(e) => format!("[error] /merge-worktree discard: {e}"),
            };
            let _ = action_tx.send(Action::EmitSessionNotice(sid_for_task, text));
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-057: pop the MergeConfirmDialog without firing any backend
    /// call.
    pub(crate) fn handle_cancel_merge_dialog(&mut self) {
        let _ = self.compositor.remove(MERGE_CONFIRM_DIALOG_ID);
    }

    /// WT-009: `MergeStatus::Success` teardown — mirrors the TS
    /// merge → summary → `cleanupCurrentSessionHandler()` +
    /// `destroySession()` + `onExit()` flow and the Rust
    /// `ExitChoice::CloseSession` teardown (dispatch_agent_exit.rs),
    /// driven by the EXPLICIT session id (not `current_session()` —
    /// the notice may have already flipped focus bookkeeping):
    ///
    /// 1. BoardStore detach of the session's work-unit binding.
    /// 2. AgentViewStore::open_sessions removal + mux window re-sync.
    /// 3. Current-work-unit pointer clear.
    /// 4. Spawned `backend.destroy_session(session_id)`.
    /// 5. `Action::BackToBoard` so the user lands on the board.
    pub(crate) fn handle_merge_success_teardown(&mut self, session_id: SessionId) {
        // Step 1: snapshot the work-unit binding BEFORE any mutation.
        let work_unit_id = self
            .agent_view_store
            .current_work_unit_id()
            .map(str::to_string);

        // Step 2: BoardStore detach so BoardView::selected_session
        // stops resolving this session for the bound work unit.
        if let Some(wu_id) = &work_unit_id {
            self.board_store.detach_session(wu_id);
        }

        // Step 3: open_sessions removal + mux window re-sync (the Rust
        // cycle source — without it Shift+Left/Right keep surfacing the
        // destroyed session).
        self.agent_view_store.remove_session_if_open(&session_id);
        self.mux_sync_window();

        // Step 4: current-work-unit pointer clear (stale chrome guard —
        // same rationale as the CloseSession path).
        self.agent_view_store.set_current_work_unit(None, None);

        // Step 5: backend destroy (spawned so BackToBoard stays
        // synchronous — open_sessions was already cleared above).
        if tokio::runtime::Handle::try_current().is_ok() {
            let backend = self.backend.clone();
            let handle = tokio::spawn(async move {
                let _ = backend.destroy_session(session_id).await;
            });
            self.pending_tasks.push(handle);
        }

        // Step 6: back to the board.
        let _ = self.action_tx.send(Action::BackToBoard);
    }

    /// Route the RPC-057 Action variants through their helpers. Called
    /// from the catch-all arm of `App::dispatch`'s match.
    pub(crate) fn try_dispatch_merge_worktree(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenMergeConfirmDialog {
                session_id,
                summary,
            } => {
                self.handle_open_merge_confirm_dialog(session_id.clone(), summary.clone());
            }
            Action::InspectChangesLoaded {
                session_id,
                summary,
            } => {
                self.handle_inspect_changes_loaded(session_id.clone(), summary.clone());
            }
            Action::MergeConfirmed { session_id } => {
                self.handle_merge_confirmed(session_id.clone());
            }
            Action::DiscardConfirmed { session_id } => {
                self.handle_discard_confirmed(session_id.clone());
            }
            Action::CancelMergeDialog => {
                self.handle_cancel_merge_dialog();
            }
            Action::MergeSuccess { session_id } => {
                // WT-009: success teardown (destroy + BackToBoard).
                self.handle_merge_success_teardown(session_id.clone());
            }
            _ => return false,
        }
        true
    }
}

/// Map a `MergeOutcome` to the matching action-bus event(s).
fn route_merge_outcome(
    action_tx: &tokio::sync::mpsc::UnboundedSender<Action>,
    session_id: SessionId,
    outcome: MergeOutcome,
) {
    match outcome.status {
        MergeStatus::Success => {
            let n = outcome.conflicts.len(); // typically 0
            let _ = n;
            let commit = outcome
                .merge_commit
                .as_deref()
                .map(|s| format!(" ({s})"))
                .unwrap_or_default();
            let _ = action_tx.send(Action::EmitSessionNotice(
                session_id.clone(),
                format!("[merge] success{commit}"),
            ));
            // WT-009: after the success notice lands, tear down the
            // session and return to the board (TS merge → summary →
            // destroySession → onExit flow). Queued AFTER the notice so
            // the notice is dispatched first (FIFO action bus).
            let _ = action_tx.send(Action::MergeSuccess { session_id });
        }
        MergeStatus::NoChanges => {
            let _ = action_tx.send(Action::EmitSessionNotice(
                session_id,
                "[merge] nothing to merge".to_string(),
            ));
        }
        MergeStatus::Conflict => {
            let body = build_conflict_context(&session_id, &outcome);
            let _ = action_tx.send(Action::SeedPendingInput {
                session_id,
                text: body,
            });
        }
    }
}

/// Build the LLM-facing conflict-context message that gets seeded into
/// the per-session input draft via `Action::SeedPendingInput`. Format
/// matches TS `buildConflictLlmContext`: header + bullet list of paths
/// + worktree footer.
///
/// WT-009: the footer prints the real worktree path
/// (`outcome.worktree_path`) instead of the session UUID so the LLM can
/// act on it.
fn build_conflict_context(session_id: &SessionId, outcome: &MergeOutcome) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Merge produced conflicts in the following files:");
    for path in &outcome.conflicts {
        let _ = writeln!(out, " - {path}");
    }
    match outcome.worktree_path.as_deref() {
        Some(wt) => {
            let _ = writeln!(out, "Effective worktree: {wt}");
        }
        None => {
            let _ = writeln!(out, "Effective worktree: {}", session_id.value);
        }
    }
    out
}
