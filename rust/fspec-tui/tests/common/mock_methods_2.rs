//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        self.core
            .delete_checkpoint_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_delete_checkpoint.lock() = Some((work_unit_id, name));
        Ok(())
    }
    pub async fn delete_all_checkpoints(&self) -> Result<()> {
        self.core
            .delete_all_checkpoints_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    pub async fn move_work_unit_up(&self, workspace: String, id: String) -> Result<()> {
        self.core
            .move_work_unit_up_calls
            .fetch_add(1, Ordering::SeqCst);
        self.record_card_request("up", &workspace, &id);
        *self.core.last_move_work_unit_up_id.lock() = Some(id);
        self.scripted_move_error()
    }
    pub async fn move_work_unit_down(&self, workspace: String, id: String) -> Result<()> {
        self.core
            .move_work_unit_down_calls
            .fetch_add(1, Ordering::SeqCst);
        self.record_card_request("down", &workspace, &id);
        *self.core.last_move_work_unit_down_id.lock() = Some(id);
        self.scripted_move_error()
    }
    pub async fn get_model_info(&self, _session_id: SessionId) -> Result<ModelInfo> {
        Ok(self.core.model_info.lock().clone())
    }
    pub async fn get_thinking_level(&self, _session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(self.core.thinking_level.lock().clone())
    }
    pub async fn get_thinking_options(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions> {
        self.core
            .thinking_options
            .lock()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no thinking options are scripted"))
    }
    pub async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        if let Some(msg) = self.core.workspace_info_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self.core.workspace_info.lock().clone())
    }
    /// BUG-182: count the lazy-view RPC calls (the trait defaults would
    /// otherwise hide them from the mux-entry / refresh assertions).
    /// BUG-184: honor the scripted payload so tests can drive a LOADED
    /// pane (empty when unscripted — the prior behavior).
    pub async fn changed_files(&self) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        self.core.changed_files_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.core.scripted_changed_files.lock().clone())
    }
    pub async fn list_checkpoints(&self) -> Result<Vec<codelet_rpc_types::CheckpointInfo>> {
        self.core
            .list_checkpoints_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self.core.scripted_checkpoints.lock().clone())
    }
    /// BUG-184: the scripted checkpoint-files payload (key match →
    /// payload; anything else → empty, as before).
    pub async fn checkpoint_diff_files(
        &self,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        Ok(self
            .core
            .scripted_checkpoint_files
            .lock()
            .clone()
            .filter(|(wu, n, _)| *wu == work_unit_id && *n == name)
            .map(|(_, _, files)| files)
            .unwrap_or_default())
    }
    /// BUG-184: the scripted checkpoint-file-diff payload (key match →
    /// payload; anything else → `None`, as before).
    pub async fn checkpoint_file_diff(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>> {
        Ok(self
            .core
            .scripted_checkpoint_file_diffs
            .lock()
            .clone()
            .filter(|(wu, n, p, _)| *wu == work_unit_id && *n == name && *p == path)
            .and_then(|(_, _, _, diff)| diff))
    }
    pub async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: MockBackend returns scripted matches (or an empty
        // Vec) so tests can drive the file search popup without
        // touching the real filesystem.
        let all = self.core.file_search_results.lock().clone();
        let filtered: Vec<String> = all
            .into_iter()
            .filter(|p| p.to_lowercase().contains(&prefix.to_lowercase()))
            .take(limit as usize)
            .collect();
        Ok(filtered)
    }
    pub async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        self.core
            .persistence_add_history_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_persistence_add_history.lock() = Some((session, text));
        Ok(())
    }
    pub async fn persistence_get_history(
        &self,
        session: SessionId,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Ok(self
            .controls
            .scripted_history
            .lock()
            .get(&session)
            .cloned()
            .unwrap_or_default())
    }
}
