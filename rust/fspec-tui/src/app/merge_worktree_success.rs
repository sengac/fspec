//! WT-009: merge-success session teardown, split out of
//! `dispatch_merge_worktree.rs` to keep that file under the 300-LoC
//! ceiling.
//!
//! Drives the TS merge → summary → `cleanupCurrentSessionHandler()` +
//! `destroySession()` + `onExit()` flow and the Rust
//! `ExitChoice::CloseSession` teardown (dispatch_agent_exit.rs) from the
//! explicit `MergeSuccess` action id (not `current_session()` — the
//! success notice may have already flipped focus bookkeeping).

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::state::App;

impl App {
    /// WT-009: `MergeStatus::Success` teardown:
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
}
