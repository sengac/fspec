//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        self.controls
            .get_pending_input_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.get_pending_input_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.controls.pending_input_store.lock();
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }
    pub async fn set_pending_input(
        &self,
        session_id: SessionId,
        text: Option<String>,
    ) -> Result<()> {
        self.controls
            .set_pending_input_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .pending_input_writes
            .lock()
            .push((session_id.clone(), text.clone()));
        if let Some(msg) = self.controls.set_pending_input_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        self.controls
            .pending_input_store
            .lock()
            .insert(session_id, text);
        Ok(())
    }
    // ── RPC-053: pause / HITL surface ────────────────────────────────────

    pub async fn get_pause_state(
        &self,
        session_id: SessionId,
    ) -> Result<Option<PendingRequest<PauseState>>> {
        self.controls
            .get_pause_state_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.get_pause_state_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.controls.pause_state_store.lock();
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }
    pub async fn get_hitl_request(
        &self,
        session_id: SessionId,
    ) -> Result<Option<PendingRequest<HitlRequest>>> {
        self.controls
            .get_hitl_request_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.get_hitl_request_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.controls.hitl_request_store.lock();
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }
    pub async fn get_exec_stdin_request(
        &self,
        session_id: SessionId,
    ) -> Result<Option<ExecStdinRequest>> {
        self.controls
            .get_exec_stdin_request_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.get_exec_stdin_request_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.controls.exec_stdin_store.lock();
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }
    // RPC-453: record "method:workspace"; answer like an empty repository.
    pub async fn checkpoint_counts_in_workspace(
        &self,
        workspace: String,
    ) -> Result<codelet_rpc_types::CheckpointCounts> {
        self.record_checkpoint_request("checkpoint_counts", &workspace);
        Ok(codelet_rpc_types::CheckpointCounts::default())
    }
    pub async fn list_checkpoints_in_workspace(
        &self,
        workspace: String,
    ) -> Result<Vec<codelet_rpc_types::CheckpointInfo>> {
        self.record_checkpoint_request("list_checkpoints", &workspace);
        Ok(Vec::new())
    }
    pub async fn checkpoint_diff_files_in_workspace(
        &self,
        workspace: String,
        _work_unit_id: String,
        _name: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        self.record_checkpoint_request("checkpoint_diff_files", &workspace);
        Ok(Vec::new())
    }
    pub async fn checkpoint_file_diff_in_workspace(
        &self,
        workspace: String,
        _work_unit_id: String,
        _name: String,
        _path: String,
    ) -> Result<Option<String>> {
        self.record_checkpoint_request("checkpoint_file_diff", &workspace);
        Ok(None)
    }
    pub async fn restore_checkpoint_file_in_workspace(
        &self,
        workspace: String,
        _work_unit_id: String,
        _name: String,
        _path: String,
    ) -> Result<()> {
        self.record_checkpoint_request("restore_checkpoint_file", &workspace);
        Ok(())
    }
    pub async fn restore_checkpoint_all_in_workspace(
        &self,
        workspace: String,
        _work_unit_id: String,
        _name: String,
    ) -> Result<()> {
        self.record_checkpoint_request("restore_checkpoint_all", &workspace);
        Ok(())
    }
    pub async fn delete_checkpoint_in_workspace(
        &self,
        workspace: String,
        _work_unit_id: String,
        _name: String,
    ) -> Result<()> {
        self.record_checkpoint_request("delete_checkpoint", &workspace);
        Ok(())
    }
    pub async fn delete_all_checkpoints_in_workspace(&self, workspace: String) -> Result<()> {
        self.record_checkpoint_request("delete_all_checkpoints", &workspace);
        Ok(())
    }
}
