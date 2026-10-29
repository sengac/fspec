macro_rules! compose_embedded_worktrees_scheduling {
    ($($collected:tt)*) => {
        compose_embedded_subscriptions! {
            $($collected)*
    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        Ok(self.client.blocklist_list(context::current()).await?)
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
            .await?)
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
        Ok(self.client.schedule_list(context::current()).await?)
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
            .await?)
    }
        }
    };
}
