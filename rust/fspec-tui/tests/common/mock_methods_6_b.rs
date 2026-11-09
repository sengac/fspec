//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn detach_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        self.orchestration
            .detach_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .orchestration
            .detach_session_worktree_error
            .lock()
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    pub async fn schedule_add(
        &self,
        _job: codelet_rpc_types::ScheduledJob,
    ) -> Result<codelet_rpc_types::ScheduledJob> {
        self.orchestration
            .schedule_add_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.schedule_add_result.lock().clone() {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn schedule_list(&self) -> Result<Vec<codelet_rpc_types::ScheduledJob>> {
        self.orchestration
            .schedule_list_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.schedule_list_result.lock().clone() {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn schedule_pause(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob> {
        self.orchestration
            .schedule_pause_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.schedule_pause_result.lock().clone() {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn schedule_resume(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob> {
        self.orchestration
            .schedule_resume_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.schedule_resume_result.lock().clone() {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn schedule_remove(&self, _name: String) -> Result<()> {
        self.orchestration
            .schedule_remove_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.schedule_remove_result.lock().clone() {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop.
    // ─────────────────────────────────────────────────────────────────

    pub async fn loop_add(
        &self,
        _session_id: SessionId,
        _interval_seconds: u32,
        _prompt: String,
    ) -> Result<codelet_rpc_types::RegisteredLoop> {
        self.orchestration
            .loop_add_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.loop_add_result.lock().clone() {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    pub async fn loop_cancel(&self, _id: String) -> Result<bool> {
        self.orchestration
            .loop_cancel_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.orchestration.loop_cancel_result.lock().clone() {
            Ok(b) => Ok(b),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
    // RPC-452: record the workspace; a scripted failure is returned.
    pub async fn changed_files_in_workspace(
        &self,
        workspace: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        self.workspaces
            .changed_files_requests
            .lock()
            .push(workspace);
        match self.workspaces.changed_files_error.lock().clone() {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(Vec::new()),
        }
    }
    pub async fn file_diff_in_workspace(
        &self,
        _workspace: String,
        _path: String,
    ) -> Result<Option<String>> {
        Ok(None)
    }
}
