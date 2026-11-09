macro_rules! compose_backend_worktrees_jobs {
    ($($collected:tt)*) => {
        compose_backend_subscriptions! {
            $($collected)*
    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the `/blocklist` view.
    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Err(BackendError::Unimplemented("blocklist_list").into())
    }

    /// RPC-057: merge a session's worktree changes back to base.
    async fn merge_session_worktree(
        &self,
        _session_id: SessionId,
        _strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        Err(BackendError::Unimplemented("merge_session_worktree").into())
    }

    /// RPC-057: discard a session's worktree changes.
    async fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        Err(BackendError::Unimplemented("discard_session_worktree").into())
    }

    /// RPC-057: prune orphaned session worktrees.
    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("prune_orphaned_worktrees").into())
    }

    /// RPC-057: list every known session worktree.
    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        Err(BackendError::Unimplemented("list_session_worktrees").into())
    }

    /// RPC-057: inspect a session's pending change summary.
    async fn inspect_session_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        Err(BackendError::Unimplemented("inspect_session_changes").into())
    }

    /// WT-009: detach a session from its isolation worktree (the
    /// `/isolation` toggle-off path). The session keeps running; the
    /// backend clears its worktree state, marks the worktree prunable,
    /// and emits `IsolationStateChange(false, None)`.
    async fn detach_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        Err(anyhow::anyhow!(
            "detach_session_worktree is not supported by this transport"
        ))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-058: persist a new scheduled job.
    async fn schedule_add(&self, _job: ScheduledJob) -> Result<ScheduledJob> {
        Err(BackendError::Unimplemented("schedule_add").into())
    }

    /// RPC-058: list every persisted scheduled job.
    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        Err(BackendError::Unimplemented("schedule_list").into())
    }

    /// RPC-058: flip a job's status to `paused`.
    async fn schedule_pause(&self, _name: String) -> Result<ScheduledJob> {
        Err(BackendError::Unimplemented("schedule_pause").into())
    }

    /// RPC-058: flip a job's status to `active`.
    async fn schedule_resume(&self, _name: String) -> Result<ScheduledJob> {
        Err(BackendError::Unimplemented("schedule_resume").into())
    }

    /// RPC-058: remove a job by name.
    async fn schedule_remove(&self, _name: String) -> Result<()> {
        Err(BackendError::Unimplemented("schedule_remove").into())
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop trait defaults.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-059: register a new session-scoped recurring prompt.
    async fn loop_add(
        &self,
        _session_id: SessionId,
        _interval_seconds: u32,
        _prompt: String,
    ) -> Result<RegisteredLoop> {
        Err(BackendError::Unimplemented("loop_add").into())
    }

    /// RPC-059: cancel a registered loop by id.
    async fn loop_cancel(&self, _id: String) -> Result<bool> {
        Err(BackendError::Unimplemented("loop_cancel").into())
    }

    /// RPC-059: list every loop registered against a session.
    async fn loop_list(&self, _session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        Err(BackendError::Unimplemented("loop_list").into())
    }
    // RPC-453: checkpoint calls for a named workspace; a backend without
    // the RPCs refuses.
    async fn checkpoint_counts_in_workspace(&self, _workspace: String) -> Result<codelet_rpc_types::CheckpointCounts> {
        Err(BackendError::Unimplemented("checkpoint_counts_in_workspace").into())
    }
    async fn list_checkpoints_in_workspace(&self, _workspace: String) -> Result<Vec<codelet_rpc_types::CheckpointInfo>> {
        Err(BackendError::Unimplemented("list_checkpoints_in_workspace").into())
    }
    async fn checkpoint_diff_files_in_workspace(&self, _workspace: String, _work_unit_id: String, _name: String) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        Err(BackendError::Unimplemented("checkpoint_diff_files_in_workspace").into())
    }
    async fn checkpoint_file_diff_in_workspace(&self, _workspace: String, _work_unit_id: String, _name: String, _path: String) -> Result<Option<String>> {
        Err(BackendError::Unimplemented("checkpoint_file_diff_in_workspace").into())
    }
    async fn restore_checkpoint_file_in_workspace(&self, _workspace: String, _work_unit_id: String, _name: String, _path: String) -> Result<()> {
        Err(BackendError::Unimplemented("restore_checkpoint_file_in_workspace").into())
    }
    async fn restore_checkpoint_all_in_workspace(&self, _workspace: String, _work_unit_id: String, _name: String) -> Result<()> {
        Err(BackendError::Unimplemented("restore_checkpoint_all_in_workspace").into())
    }
    async fn delete_checkpoint_in_workspace(&self, _workspace: String, _work_unit_id: String, _name: String) -> Result<()> {
        Err(BackendError::Unimplemented("delete_checkpoint_in_workspace").into())
    }
    async fn delete_all_checkpoints_in_workspace(&self, _workspace: String) -> Result<()> {
        Err(BackendError::Unimplemented("delete_all_checkpoints_in_workspace").into())
    }
    // RPC-454: a named workspace's footer info and git state; a backend
    // without the RPCs refuses.
    async fn get_workspace_info_in_workspace(&self, _workspace: String) -> Result<codelet_rpc_types::WorkspaceInfo> {
        Err(BackendError::Unimplemented("get_workspace_info_in_workspace").into())
    }
    async fn git_state_in_workspace(&self, _workspace: String) -> Result<codelet_rpc_types::GitState> {
        Err(BackendError::Unimplemented("git_state_in_workspace").into())
    }
    // RPC-455: file search and input history of a named workspace; a
    // backend without the RPCs refuses.
    async fn search_files_in_workspace(&self, _workspace: String, _prefix: String, _limit: u32) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("search_files_in_workspace").into())
    }
    async fn persistence_add_history_in_workspace(&self, _workspace: String, _session: SessionId, _text: String) -> Result<()> {
        Err(BackendError::Unimplemented("persistence_add_history_in_workspace").into())
    }
    async fn persistence_get_history_in_workspace(&self, _workspace: String, _session: SessionId, _limit: u32) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("persistence_get_history_in_workspace").into())
    }
    async fn persistence_search_history_in_workspace(&self, _workspace: String, _query: String) -> Result<Vec<HistoryMatch>> {
        Err(BackendError::Unimplemented("persistence_search_history_in_workspace").into())
    }
    // TUI-122 / RPC-457: add or remove a registered workspace; a backend
    // without the RPCs refuses.
    async fn add_workspace(&self, _path: String) -> Result<codelet_rpc_types::WorkspaceEntry> {
        Err(BackendError::Unimplemented("add_workspace").into())
    }
    async fn remove_workspace(&self, _path: String) -> Result<()> {
        Err(BackendError::Unimplemented("remove_workspace").into())
    }
        }
    };
}
