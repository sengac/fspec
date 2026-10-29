macro_rules! compose_embedded_workspace_history {
    ($($collected:tt)*) => {
        compose_embedded_models_profiles! {
            $($collected)*
    async fn health(&self) -> Result<HealthInfo> {
        // RPC-011: embedded backend routes through the same tarpc
        // FspecService::health() method — both transports share the
        // single `FspecServiceImpl` implementation per RPC-005 rule.
        Ok(self.client.health(context::current()).await?)
    }

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        // RPC-015: route through the shared tarpc method so the
        // embedded transport produces the same result as the WS
        // transport against the same SharedFspecService.
        Ok(self.client.checkpoint_counts(context::current()).await?)
    }

    async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
        // RPC-355: one-line delegate to the shared tarpc method.
        Ok(self.client.changed_files(context::current()).await?)
    }

    async fn file_diff(&self, path: String) -> Result<Option<String>> {
        Ok(self.client.file_diff(context::current(), path).await?)
    }

    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        // RPC-362: one-line delegate to the shared tarpc method.
        Ok(self.client.list_checkpoints(context::current()).await?)
    }

    async fn checkpoint_diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>> {
        Ok(self
            .client
            .checkpoint_diff_files(context::current(), work_unit_id, name)
            .await?)
    }

    async fn checkpoint_file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        Ok(self
            .client
            .checkpoint_file_diff(context::current(), work_unit_id, name, path)
            .await?)
    }

    async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        self.client
            .restore_checkpoint_file(context::current(), work_unit_id, name, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_checkpoint_all(&self, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .restore_checkpoint_all(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        self.client
            .delete_checkpoint(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_all_checkpoints(&self) -> Result<()> {
        self.client
            .delete_all_checkpoints(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        // RPC-017: delegate to the shared tarpc method. The service
        // implementation maps any helper-level error to a `String` so
        // both transports surface identical diagnostics; we lift that
        // back into anyhow::Error here.
        self.client
            .move_work_unit_up(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_down(&self, id: String) -> Result<()> {
        self.client
            .move_work_unit_down(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo> {
        // RPC-018: one-line delegate to the shared tarpc method.
        Ok(self
            .client
            .get_model_info(context::current(), session_id)
            .await?)
    }

    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(self
            .client
            .get_thinking_level(context::current(), session_id)
            .await?)
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        Ok(self.client.get_workspace_info(context::current()).await?)
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: one-line delegate to the shared tarpc method.
        Ok(self
            .client
            .search_files(context::current(), prefix, limit)
            .await?)
    }

    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        // RPC-025: one-line delegate to the shared tarpc method.
        self.client
            .persistence_add_history(context::current(), session, text)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>> {
        self.client
            .persistence_get_history(context::current(), session, limit)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        self.client
            .persistence_search_history(context::current(), query)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        // RPC-026: one-line delegate to the shared tarpc method.
        self.client
            .persistence_delete_session(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
        }
    };
}
