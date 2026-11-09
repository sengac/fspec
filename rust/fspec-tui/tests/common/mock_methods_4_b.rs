//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        self.controls
            .get_provider_credential_calls
            .fetch_add(1, Ordering::SeqCst);
        let store = self.controls.provider_credentials.lock();
        Ok(store.iter().find(|p| p.provider_id == provider_id).cloned())
    }
    pub async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        self.controls
            .set_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_set_provider_credentials.lock() = Some((provider_id.clone(), creds));
        if let Some(msg) = self.controls.set_provider_credentials_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // Mark the row as configured in the in-memory store so a follow-
        // up `list_provider_credentials` reflects the save (unless a
        // one-shot post-save override is set).
        let mut store = self.controls.provider_credentials.lock();
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = true;
        }
        Ok(())
    }
    pub async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        self.controls
            .delete_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_delete_provider_credentials.lock() = Some(provider_id.clone());
        if let Some(msg) = self
            .controls
            .delete_provider_credentials_error
            .lock()
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let mut store = self.controls.provider_credentials.lock();
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = false;
            row.model_count = 0;
        }
        Ok(())
    }
    pub async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.controls
            .save_profile_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_save_profile.lock() = Some((provider_id, profile_name, definition));
        if let Some(msg) = self.controls.save_profile_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        self.controls
            .delete_profile_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_delete_profile.lock() = Some((provider_id, profile_name));
        if let Some(msg) = self.controls.delete_profile_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    // RPC-454: record the workspace; answer like a fresh repository there.
    pub async fn get_workspace_info_in_workspace(
        &self,
        workspace: String,
    ) -> Result<codelet_rpc_types::WorkspaceInfo> {
        self.record_workspace_git_request("info", &workspace);
        Ok(codelet_rpc_types::WorkspaceInfo {
            cwd: workspace,
            git_branch: Some("main".to_string()),
        })
    }
    pub async fn git_state_in_workspace(
        &self,
        workspace: String,
    ) -> Result<codelet_rpc_types::GitState> {
        self.record_workspace_git_request("git_state", &workspace);
        Ok(codelet_rpc_types::GitState::default())
    }
    // RPC-455: record the workspace, then answer like the start-workspace call.
    pub async fn search_files_in_workspace(
        &self,
        workspace: String,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>> {
        self.record_scope_request("search_files", &workspace);
        self.search_files(prefix, limit).await
    }
    // BUG-541: record the session as "search_files_in_session:<id>", then
    // fail as scripted or answer like the start-workspace call.
    pub async fn search_files_in_session(
        &self,
        session_id: SessionId,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>> {
        self.record_scope_request("search_files_in_session", &session_id.value);
        if let Some(message) = self.workspaces.session_search_error.lock().clone() {
            return Err(anyhow::anyhow!(message));
        }
        self.search_files(prefix, limit).await
    }
    pub async fn persistence_add_history_in_workspace(
        &self,
        workspace: String,
        session: SessionId,
        text: String,
    ) -> Result<()> {
        self.record_scope_request("add_history", &workspace);
        self.persistence_add_history(session, text).await
    }
    pub async fn persistence_get_history_in_workspace(
        &self,
        workspace: String,
        session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>> {
        self.record_scope_request("get_history", &workspace);
        self.persistence_get_history(session, limit).await
    }
    pub async fn persistence_search_history_in_workspace(
        &self,
        workspace: String,
        query: String,
    ) -> Result<Vec<codelet_rpc_types::HistoryMatch>> {
        self.record_scope_request("search_history", &workspace);
        self.persistence_search_history(query).await
    }
}
