macro_rules! compose_backend_worktrees_jobs {
    ($($collected:tt)*) => {
        compose_backend_subscriptions! {
            $($collected)*
    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the `/blocklist` view.
    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Ok(Vec::new())
    }

    /// RPC-057: merge a session's worktree changes back to base.
    async fn merge_session_worktree(
        &self,
        _session_id: SessionId,
        _strategy: MergeStrategy,
    ) -> Result<MergeOutcome> {
        Ok(MergeOutcome::default())
    }

    /// RPC-057: discard a session's worktree changes.
    async fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-057: prune orphaned session worktrees.
    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// RPC-057: list every known session worktree.
    async fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>> {
        Ok(Vec::new())
    }

    /// RPC-057: inspect a session's pending change summary.
    async fn inspect_session_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<SessionChangesSummary> {
        Ok(SessionChangesSummary::default())
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
        Ok(ScheduledJob::default())
    }

    /// RPC-058: list every persisted scheduled job.
    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        Ok(Vec::new())
    }

    /// RPC-058: flip a job's status to `paused`.
    async fn schedule_pause(&self, _name: String) -> Result<ScheduledJob> {
        Ok(ScheduledJob::default())
    }

    /// RPC-058: flip a job's status to `active`.
    async fn schedule_resume(&self, _name: String) -> Result<ScheduledJob> {
        Ok(ScheduledJob::default())
    }

    /// RPC-058: remove a job by name.
    async fn schedule_remove(&self, _name: String) -> Result<()> {
        Ok(())
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
        Ok(RegisteredLoop::default())
    }

    /// RPC-059: cancel a registered loop by id.
    async fn loop_cancel(&self, _id: String) -> Result<bool> {
        Ok(false)
    }

    /// RPC-059: list every loop registered against a session.
    async fn loop_list(&self, _session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        Ok(Vec::new())
    }
        }
    };
}
