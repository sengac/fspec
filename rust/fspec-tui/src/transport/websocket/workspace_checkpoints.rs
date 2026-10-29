macro_rules! compose_websocket_workspace_checkpoints {
    ($($collected:tt)*) => {
        compose_websocket_history_models! {
            $($collected)*
    async fn health(&self) -> Result<HealthInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().health(context::current()).await?)
    }

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_counts(context::current())
            .await?)
    }

    async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
        // RPC-355: guarded delegate following the standard Disconnected pattern.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().changed_files(context::current()).await?)
    }

    async fn file_diff(&self, path: String) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().file_diff(context::current(), path).await?)
    }

    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().list_checkpoints(context::current()).await?)
    }

    async fn checkpoint_diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_diff_files(context::current(), work_unit_id, name)
            .await?)
    }

    async fn checkpoint_file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .checkpoint_file_diff(context::current(), work_unit_id, name, path)
            .await?)
    }

    async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_checkpoint_file(context::current(), work_unit_id, name, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn restore_checkpoint_all(&self, work_unit_id: String, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .restore_checkpoint_all(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_checkpoint(context::current(), work_unit_id, name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn delete_all_checkpoints(&self) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .delete_all_checkpoints(context::current())
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .move_work_unit_up(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_work_unit_down(&self, id: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .move_work_unit_down(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_model_info(context::current(), session_id)
            .await?)
    }

    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_thinking_level(context::current(), session_id)
            .await?)
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_workspace_info(context::current())
            .await?)
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: route through the shared tarpc method, returning
        // Disconnected when the supervisor has dropped the client slot.
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .search_files(context::current(), prefix, limit)
            .await?)
    }
        }
    };
}
