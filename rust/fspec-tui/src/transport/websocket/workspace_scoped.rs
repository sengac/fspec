// RPC-455: file search and input history of a named workspace.
macro_rules! compose_websocket_workspace_scoped {
    ($($collected:tt)*) => {
        compose_websocket_history_models! {
            $($collected)*
    async fn search_files_in_workspace(&self, workspace: String, prefix: String, limit: u32) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .search_files_in_workspace(context::current(), workspace, prefix, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_add_history_in_workspace(&self, workspace: String, session: SessionId, text: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_add_history_in_workspace(context::current(), workspace, session, text)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_get_history_in_workspace(&self, workspace: String, session: SessionId, limit: u32) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_get_history_in_workspace(context::current(), workspace, session, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn persistence_search_history_in_workspace(&self, workspace: String, query: String) -> Result<Vec<HistoryMatch>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .persistence_search_history_in_workspace(context::current(), workspace, query)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    // TUI-122 / RPC-457: add or remove a registered workspace.
    async fn add_workspace(&self, path: String) -> Result<codelet_rpc_types::WorkspaceEntry> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .add_workspace(context::current(), path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    async fn remove_workspace(&self, path: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .remove_workspace(context::current(), path)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
        }
    };
}
