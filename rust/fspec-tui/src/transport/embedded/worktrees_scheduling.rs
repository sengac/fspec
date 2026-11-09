macro_rules! compose_embedded_worktrees_scheduling {
    ($($collected:tt)*) => {
        compose_embedded_subscriptions! {
            $($collected)*
    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Ok(self.client.blocklist_list(context::current()).await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }

    async fn merge_session_worktree(
        &self,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        self.client
            .merge_session_worktree(context::current(), session_id, strategy)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn discard_session_worktree(&self, session_id: SessionId) -> Result<()> {
        self.client
            .discard_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        self.client
            .prune_orphaned_worktrees(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        Ok(self
            .client
            .list_session_worktrees(context::current())
            .await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }

    async fn inspect_session_changes(
        &self,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        self.client
            .inspect_session_changes(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn detach_session_worktree(&self, session_id: SessionId) -> Result<()> {
        self.client
            .detach_session_worktree(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        self.client
            .schedule_add(context::current(), job)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        Ok(self.client.schedule_list(context::current()).await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }

    async fn schedule_pause(&self, name: String) -> Result<ScheduledJob> {
        self.client
            .schedule_pause(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_resume(&self, name: String) -> Result<ScheduledJob> {
        self.client
            .schedule_resume(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_remove(&self, name: String) -> Result<()> {
        self.client
            .schedule_remove(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop> {
        self.client
            .loop_add(context::current(), session_id, interval_seconds, prompt)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_cancel(&self, id: String) -> Result<bool> {
        self.client
            .loop_cancel(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_list(&self, session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        Ok(self
            .client
            .loop_list(context::current(), session_id)
            .await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }
    // RPC-453: checkpoint calls for a named workspace.
    async fn checkpoint_counts_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::CheckpointCounts> {
        self.client
            .checkpoint_counts_in_workspace(context::current(), workspace)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn list_checkpoints_in_workspace(&self, workspace: String) -> Result<Vec<codelet_rpc_types::CheckpointInfo>> {
        self.client
            .list_checkpoints_in_workspace(context::current(), workspace)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn checkpoint_diff_files_in_workspace(&self, workspace: String, work_unit_id: String, name: String) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        self.client
            .checkpoint_diff_files_in_workspace(context::current(), workspace, work_unit_id, name)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn checkpoint_file_diff_in_workspace(&self, workspace: String, work_unit_id: String, name: String, path: String) -> Result<Option<String>> {
        self.client
            .checkpoint_file_diff_in_workspace(context::current(), workspace, work_unit_id, name, path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn restore_checkpoint_file_in_workspace(&self, workspace: String, work_unit_id: String, name: String, path: String) -> Result<()> {
        self.client
            .restore_checkpoint_file_in_workspace(context::current(), workspace, work_unit_id, name, path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn restore_checkpoint_all_in_workspace(&self, workspace: String, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .restore_checkpoint_all_in_workspace(context::current(), workspace, work_unit_id, name)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn delete_checkpoint_in_workspace(&self, workspace: String, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .delete_checkpoint_in_workspace(context::current(), workspace, work_unit_id, name)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn delete_all_checkpoints_in_workspace(&self, workspace: String) -> Result<()> {
        self.client
            .delete_all_checkpoints_in_workspace(context::current(), workspace)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    // RPC-454: a named workspace's footer info and git state.
    async fn get_workspace_info_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::WorkspaceInfo> {
        self.client
            .get_workspace_info_in_workspace(context::current(), workspace)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn git_state_in_workspace(&self, workspace: String) -> Result<codelet_rpc_types::GitState> {
        self.client
            .git_state_in_workspace(context::current(), workspace)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    // RPC-455: file search and input history of a named workspace.
    async fn search_files_in_workspace(&self, workspace: String, prefix: String, limit: u32) -> Result<Vec<String>> {
        self.client
            .search_files_in_workspace(context::current(), workspace, prefix, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_add_history_in_workspace(&self, workspace: String, session: SessionId, text: String) -> Result<()> {
        self.client
            .persistence_add_history_in_workspace(context::current(), workspace, session, text)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_get_history_in_workspace(&self, workspace: String, session: SessionId, limit: u32) -> Result<Vec<String>> {
        self.client
            .persistence_get_history_in_workspace(context::current(), workspace, session, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_search_history_in_workspace(&self, workspace: String, query: String) -> Result<Vec<HistoryMatch>> {
        self.client
            .persistence_search_history_in_workspace(context::current(), workspace, query)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    // TUI-122 / RPC-457: add or remove a registered workspace.
    async fn add_workspace(&self, path: String) -> Result<codelet_rpc_types::WorkspaceEntry> {
        self.client
            .add_workspace(context::current(), path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn remove_workspace(&self, path: String) -> Result<()> {
        self.client
            .remove_workspace(context::current(), path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
        }
    };
}
