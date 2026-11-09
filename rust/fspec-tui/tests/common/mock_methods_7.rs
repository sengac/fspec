//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn loop_list(
        &self,
        _session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::RegisteredLoop>> {
        self.orchestration
            .loop_list_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.loop_list_result.lock().clone() {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn create_isolated_session(
        &self,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        self.orchestration
            .create_isolated_session_calls
            .fetch_add(1, Ordering::SeqCst);
        self.orchestration
            .created_isolated_session_roles
            .lock()
            .push(role);
        match self
            .orchestration
            .create_isolated_session_result
            .lock()
            .clone()
        {
            Ok(info) => Ok(info),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn read_card_attachment(&self, workspace: String, path: String) -> Result<Vec<u8>> {
        self.workspaces
            .attachment_requests
            .lock()
            .push((workspace.clone(), path.clone()));
        // Unscripted: behave like a daemon on this machine.
        let scripted = self.workspaces.attachment.lock().clone();
        match scripted {
            Some(result) => result.map_err(|error| anyhow::anyhow!(error)),
            None => codelet_attachment_viewer::read_workspace_attachment(
                std::path::Path::new(&workspace),
                &path,
            )
            .map_err(|error| anyhow::anyhow!(error)),
        }
    }
    pub async fn create_isolated_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        *self.workspaces.isolated_created_in.lock() = Some(workspace);
        self.create_isolated_session(role).await
    }
    // ── RPC-061 supervisor / subordinate forwarders ─────────────────────

    pub async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        self.orchestration
            .get_supervisors_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .orchestration
            .supervisors_results
            .lock()
            .get(&session_id)
            .cloned()
            .unwrap_or_default())
    }
    pub async fn add_supervisor(
        &self,
        _subordinate_id: SessionId,
        _supervisor_id: SessionId,
    ) -> Result<()> {
        self.orchestration
            .add_supervisor_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.add_supervisor_result.lock().clone() {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn remove_supervisor(&self, _supervisor_id: SessionId) -> Result<()> {
        self.orchestration
            .remove_supervisor_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    pub async fn get_subordinate(&self, _supervisor_id: SessionId) -> Result<Option<SessionId>> {
        self.orchestration
            .get_subordinate_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }
    pub async fn get_subordinates(&self, supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        self.orchestration
            .get_subordinates_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(message) = self.orchestration.subordinates_error.lock().clone() {
            return Err(anyhow::anyhow!("{message}"));
        }
        Ok(self
            .orchestration
            .subordinates_results
            .lock()
            .get(&supervisor_id)
            .cloned()
            .unwrap_or_default())
    }
    pub async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        self.orchestration
            .receive_incoming_message_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.orchestration.last_received_incoming_message.lock() = Some((subordinate_id, message));
        match self
            .orchestration
            .receive_incoming_message_result
            .lock()
            .clone()
        {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    // RPC-450: details are recorded with the workspace they name.
    pub async fn get_work_unit_details(
        &self,
        workspace: String,
        id: String,
    ) -> std::result::Result<String, codelet_fspec_tui::transport::BackendError> {
        self.record_card_request("details", &workspace, &id);
        Ok(
            serde_json::json!({"id": id, "workspace": workspace, "workUnit": {"id": id}})
                .to_string(),
        )
    }
    // BUG-325: custom-model writes follow the scripted outcome.
    pub async fn add_custom_model(
        &self,
        _provider_id: String,
        _profile_name: String,
        _definition: codelet_rpc_types::CustomModelDefinition,
    ) -> Result<()> {
        self.custom_model_write_result()
    }
}
