//! MockBackend scripting and recording helpers (part 6).

use super::*;

impl MockBackend {
    /// RPC-057: force the next `merge_session_worktree` call to fail.
    #[allow(dead_code)]
    pub fn set_merge_session_worktree_error(&self, message: String) {
        *self.controls.merge_session_worktree_error.lock() = Some(message);
    }
    /// RPC-057: per-call counter for `merge_session_worktree`.
    #[allow(dead_code)]
    pub fn merge_session_worktree_calls(&self) -> usize {
        self.controls
            .merge_session_worktree_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-057: force the next `discard_session_worktree` call to fail.
    #[allow(dead_code)]
    pub fn set_discard_session_worktree_error(&self, message: String) {
        *self.controls.discard_session_worktree_error.lock() = Some(message);
    }
    /// RPC-057: per-call counter for `discard_session_worktree`.
    #[allow(dead_code)]
    pub fn discard_session_worktree_calls(&self) -> usize {
        self.controls
            .discard_session_worktree_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-057: seed the pruned session ids list.
    #[allow(dead_code)]
    pub fn seed_pruned_sessions(&self, ids: Vec<String>) {
        *self.orchestration.pruned_sessions.lock() = ids;
    }
    /// WT-005: force the next `prune_orphaned_worktrees` call to fail.
    #[allow(dead_code)]
    pub fn set_prune_orphaned_worktrees_error(&self, message: String) {
        *self.orchestration.prune_orphaned_worktrees_error.lock() = Some(message);
    }
    /// RPC-057: per-call counter for `prune_orphaned_worktrees`.
    #[allow(dead_code)]
    pub fn prune_orphaned_worktrees_calls(&self) -> usize {
        self.controls
            .prune_orphaned_worktrees_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-057: seed the `SessionWorktreeInfo` rows.
    #[allow(dead_code)]
    pub fn seed_session_worktrees(&self, rows: Vec<codelet_rpc_types::SessionWorktreeInfo>) {
        *self.orchestration.session_worktrees.lock() = rows;
    }
    /// RPC-057: per-call counter for `list_session_worktrees`.
    #[allow(dead_code)]
    pub fn list_session_worktrees_calls(&self) -> usize {
        self.orchestration
            .list_session_worktrees_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-057: seed the `SessionChangesSummary`.
    #[allow(dead_code)]
    pub fn seed_session_changes_summary(&self, summary: codelet_rpc_types::SessionChangesSummary) {
        *self.orchestration.session_changes_summary.lock() = summary;
    }
    /// RPC-057: per-call counter for `inspect_session_changes`.
    #[allow(dead_code)]
    pub fn inspect_session_changes_calls(&self) -> usize {
        self.orchestration
            .inspect_session_changes_calls
            .load(Ordering::SeqCst)
    }
    // ─────────────────────────────────────────────────────────────────
    // WT-009 — detach_session_worktree seeds + counter.
    // ─────────────────────────────────────────────────────────────────

    /// WT-009: force the next `detach_session_worktree` call to fail.
    #[allow(dead_code)]
    pub fn set_detach_session_worktree_error(&self, message: String) {
        *self.orchestration.detach_session_worktree_error.lock() = Some(message);
    }
    /// WT-009: per-call counter for `detach_session_worktree`.
    #[allow(dead_code)]
    pub fn detach_session_worktree_calls(&self) -> usize {
        self.orchestration
            .detach_session_worktree_calls
            .load(Ordering::SeqCst)
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule seeds + counters.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_schedule_add_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self.orchestration.schedule_add_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_schedule_list_result(
        &self,
        result: std::result::Result<Vec<codelet_rpc_types::ScheduledJob>, String>,
    ) {
        *self.orchestration.schedule_list_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_schedule_pause_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self.orchestration.schedule_pause_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_schedule_resume_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self.orchestration.schedule_resume_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_schedule_remove_result(&self, result: std::result::Result<(), String>) {
        *self.orchestration.schedule_remove_result.lock() = result;
    }
    /// WT-018: answer `session_card_changes` with `answer`.
    #[allow(dead_code)]
    pub fn set_session_card_changes(
        &self,
        answer: impl Fn(&SessionId) -> std::result::Result<Vec<codelet_rpc_types::SessionCardChange>, String>
            + Send
            + Sync
            + 'static,
    ) {
        *self.orchestration.session_card_changes.lock() = Some(Arc::new(answer));
    }
    /// WT-018: per-call counter for `session_card_changes`.
    #[allow(dead_code)]
    pub fn session_card_changes_calls(&self) -> usize {
        self.orchestration
            .session_card_changes_calls
            .load(Ordering::SeqCst)
    }
    /// WT-018: answer `move_session_cards_back` with `answer`.
    #[allow(dead_code)]
    pub fn set_move_session_cards_back(
        &self,
        answer: impl Fn(&SessionId) -> std::result::Result<Vec<codelet_rpc_types::CardMoveBack>, String>
            + Send
            + Sync
            + 'static,
    ) {
        *self.orchestration.move_session_cards_back.lock() = Some(Arc::new(answer));
    }
    /// WT-018: per-call counter for `move_session_cards_back`.
    #[allow(dead_code)]
    pub fn move_session_cards_back_calls(&self) -> usize {
        self.orchestration
            .move_session_cards_back_calls
            .load(Ordering::SeqCst)
    }
}
