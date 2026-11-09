//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn list_provider_settings(
        &self,
    ) -> std::result::Result<
        Vec<codelet_rpc_types::ProviderSettingsConfig>,
        codelet_fspec_tui::transport::BackendError,
    > {
        let snapshot = self.provider_settings_snapshot();
        self.gates.pass("list_provider_settings").await;
        snapshot
    }
    pub async fn save_provider_settings(
        &self,
        provider_id: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> std::result::Result<(), codelet_fspec_tui::transport::BackendError> {
        self.store_provider_settings(provider_id, definition)
    }
    pub async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        self.core
            .list_work_units_calls
            .fetch_add(1, Ordering::SeqCst);
        self.core
            .work_units
            .lock()
            .clone()
            .map_err(anyhow::Error::msg)
    }
    pub async fn list_workspaces(&self) -> Result<Vec<codelet_rpc_types::WorkspaceEntry>> {
        Ok(self.workspaces.entries.lock().clone())
    }
    pub async fn list_workspace_work_units(&self, workspace: String) -> Result<Vec<WorkUnitInfo>> {
        let units = self.workspaces.units.lock();
        Ok(units.get(&workspace).cloned().unwrap_or_default())
    }
    pub async fn create_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<SessionId> {
        *self.workspaces.created_in.lock() = Some(workspace);
        self.create_session(role).await
    }
    pub fn workspace_work_units_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::WorkspaceWorkUnits> {
        self.workspaces.tx.subscribe()
    }
    /// BUG-206: scripted in-memory session IDs (auto-restore liveness gate).
    pub async fn list_live_sessions(&self) -> Result<Vec<SessionId>> {
        self.core
            .list_live_sessions_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self.core.scripted_live_sessions.lock().clone())
    }
    pub async fn create_session(&self, role: Option<String>) -> Result<SessionId> {
        self.core
            .create_session_calls
            .fetch_add(1, Ordering::SeqCst);
        self.core.created_session_roles.lock().push(role.clone());
        let scripted = self.core.scripted_session.lock().clone();
        let session = scripted
            .unwrap_or_else(|| Ok(SessionId::new("s-mock-default")))
            .map_err(anyhow::Error::msg)?;
        // TUI-114: like the daemon, a session keeps the role it was created
        // with, so a later `get_session_role` returns it.
        if role.is_some() {
            self.seed_session_role(session.clone(), role);
        }
        self.gates.pass("create_session").await;
        Ok(session)
    }
    pub async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.core.send_input_calls.fetch_add(1, Ordering::SeqCst);
        *self.core.last_send_input.lock() = Some((id, text));
        self.send_input_result()
    }
    pub async fn submit_input(
        &self,
        id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<()> {
        self.orchestration
            .submitted_inputs
            .lock()
            .push((id, text, mode));
        Ok(())
    }
    pub async fn configure_session_mcp_servers(
        &self,
        id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>> {
        let _ = (id, servers);
        Err(anyhow::anyhow!(
            "MCP server configuration is not available on this test backend"
        ))
    }
    pub async fn control_exec_stdin(
        &self,
        id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<()> {
        self.exec_control
            .record((id, exec_session_id, control))
            .map_err(anyhow::Error::msg)
    }
    pub async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.core.interrupt_calls.fetch_add(1, Ordering::SeqCst);
        *self.core.last_interrupt.lock() = Some(id);
        Ok(())
    }
    /// RPC-098: ESC exit-confirmation tests rely on this override to
    /// verify that `Close` reaches the backend and that
    /// `Cancel` does NOT (BUG-205: the Detach option was removed).
    pub async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        self.core
            .destroy_session_calls
            .fetch_add(1, Ordering::SeqCst);
        // TUI-124: a scripted failure leaves the session live.
        if let Some(message) = self
            .orchestration
            .destroy_session_errors
            .lock()
            .get(&session_id)
            .cloned()
        {
            return Err(anyhow::anyhow!("{message}"));
        }
        for subordinates in self.orchestration.subordinates_results.lock().values_mut() {
            subordinates.retain(|id| *id != session_id);
        }
        self.orchestration
            .destroyed_sessions
            .lock()
            .push(session_id.clone());
        *self.core.last_destroyed_session.lock() = Some(session_id);
        Ok(())
    }
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        let guard = self.core.work_units_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                // RPC-415: disconnected — hand back a pre-closed receiver
                // so the subscriber's next recv() returns RecvError::Closed.
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        let guard = self.core.chunks_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                // chunks_tx was dropped by `close_chunks_tx` — hand back a
                // pre-closed receiver so the subscriber's next recv()
                // returns `RecvError::Closed`.
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
}
