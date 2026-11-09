macro_rules! compose_websocket_scheduling_loops {
    ($($collected:tt)*) => {
        compose_websocket_blocklist_session_rules! {
            $($collected)*
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_add(context::current(), job)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_list(&self) -> Result<Vec<ScheduledJob>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().schedule_list(context::current()).await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }

    async fn schedule_pause(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_pause(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_resume(&self, name: String) -> Result<ScheduledJob> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_resume(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn schedule_remove(&self, name: String) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .schedule_remove(context::current(), name)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_add(context::current(), session_id, interval_seconds, prompt)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_cancel(&self, id: String) -> Result<bool> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .loop_cancel(context::current(), id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn loop_list(&self, session_id: SessionId) -> Result<Vec<RegisteredLoop>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .loop_list(context::current(), session_id)
            .await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }

    /// BOARD-024: thin delegation to the shared canonical details RPC,
    /// preserving the disconnected-client guard.
    async fn get_work_unit_details(
        &self,
        workspace: String,
        id: String,
    ) -> std::result::Result<String, BackendError> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .get_work_unit_details(context::current(), workspace, id)
            .await
            .map_err(|error| BackendError::WorkUnitDetails(error.to_string()))?
            .map_err(BackendError::WorkUnitDetails)
    }

    fn workspace_work_units_rx(&self) -> broadcast::Receiver<WorkspaceWorkUnits> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.workspace_work_units_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn workspace_git_state_rx(&self) -> broadcast::Receiver<codelet_rpc_types::WorkspaceGitState> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.workspace_git_state_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .list_workspaces(context::current())
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn list_workspace_work_units(&self, workspace: String) -> Result<Vec<WorkUnitInfo>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .list_workspace_work_units(context::current(), workspace)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn create_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<SessionId> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .create_session_in_workspace(context::current(), workspace, role)
            .await?
            .map_err(anyhow::Error::msg)
    }

    /// RPC-440: read the daemon's default model.
    async fn get_default_model(&self) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client.client().get_default_model(context::current()).await?)
    }

    /// RIG-016: steer, queue or interrupt through the daemon.
    async fn submit_input(
        &self,
        id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .submit_input(context::current(), id, text, mode)
            .await?
            .map_err(anyhow::Error::msg)
    }

    /// MCP-003: the daemon checks this connection's bearer authentication.
    async fn configure_session_mcp_servers(
        &self,
        id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .configure_session_mcp_servers(context::current(), id, servers)
            .await?
            .map_err(anyhow::Error::msg)
    }

    /// TOOL-027: forward an exec-session control to the daemon.
    async fn control_exec_stdin(
        &self,
        id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<()> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .control_exec_stdin(context::current(), id, exec_session_id, control)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn create_isolated_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .create_isolated_session_in_workspace(context::current(), workspace, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn read_card_attachment(&self, workspace: String, path: String) -> Result<Vec<u8>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .read_card_attachment(context::current(), workspace, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn changed_files_in_workspace(
        &self,
        workspace: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .changed_files_in_workspace(context::current(), workspace)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn file_diff_in_workspace(&self, workspace: String, path: String) -> Result<Option<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .file_diff_in_workspace(context::current(), workspace, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_thinking_options(
        &self,
        session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        Ok(client
            .client()
            .get_thinking_options(context::current(), session_id)
            .await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }
    // BUG-541: file search in the session's working directory.
    async fn search_files_in_session(
        &self,
        session_id: SessionId,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or(BackendError::Disconnected)?;
        client
            .client()
            .search_files_in_session(context::current(), session_id, prefix, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
        }
    };
}
