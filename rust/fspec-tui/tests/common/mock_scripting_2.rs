//! MockBackend scripting and recording helpers (part 2).

use super::*;

impl MockBackend {
    pub fn last_interrupt(&self) -> Option<SessionId> {
        self.core.last_interrupt.lock().clone()
    }
    /// RPC-098: count of `destroy_session` calls observed by this mock.
    pub fn destroy_session_calls(&self) -> usize {
        self.core.destroy_session_calls.load(Ordering::SeqCst)
    }
    /// RPC-098: most recently destroyed SessionId.
    pub fn last_destroyed_session(&self) -> Option<SessionId> {
        self.core.last_destroyed_session.lock().clone()
    }
    /// RPC-015: preload the CheckpointCounts the next `checkpoint_counts`
    /// call returns.
    pub fn set_checkpoint_counts(&self, counts: CheckpointCounts) {
        *self.core.checkpoint_counts.lock() = Ok(counts);
    }
    /// BUG-379: make every later `checkpoint_counts` fail with `message`.
    pub fn fail_checkpoint_counts(&self, message: &str) {
        *self.core.checkpoint_counts.lock() = Err(message.to_string());
    }
    /// BUG-379: the scripted `checkpoint_counts` result.
    pub(crate) fn checkpoint_counts_result(&self) -> Result<CheckpointCounts> {
        self.core
            .checkpoint_counts
            .lock()
            .clone()
            .map_err(anyhow::Error::msg)
    }
    /// RPC-015: how many times `checkpoint_counts()` has been awaited.
    /// BUG-182: how many times `changed_files()` was called (mux entry /
    /// git-state refresh drives it).
    pub fn changed_files_calls(&self) -> usize {
        self.core.changed_files_calls.load(Ordering::SeqCst)
    }
    /// BUG-182: how many times `list_checkpoints()` was called (mux
    /// entry / git-state refresh drives it).
    pub fn list_checkpoints_calls(&self) -> usize {
        self.core.list_checkpoints_calls.load(Ordering::SeqCst)
    }
    /// BUG-184: script the payload `changed_files()` returns so an
    /// App-level test can drive a LOADED changed-files pane (the trait
    /// default returns an empty Vec).
    pub fn set_changed_files(&self, files: Vec<codelet_rpc_types::ChangedFile>) {
        *self.core.scripted_changed_files.lock() = files;
    }
    /// BUG-184: script the payload `list_checkpoints()` returns.
    pub fn set_checkpoints(&self, checkpoints: Vec<codelet_rpc_types::CheckpointInfo>) {
        *self.core.scripted_checkpoints.lock() = checkpoints;
    }
    /// BUG-184: snapshot of the scripted `changed_files()` payload (the
    /// count a test expects the pane to display).
    pub fn scripted_changed_files_snapshot(&self) -> usize {
        self.core.scripted_changed_files.lock().len()
    }
    /// BUG-184: script the payload `checkpoint_diff_files(wu, name)`
    /// returns for ONE (work_unit_id, name) key.
    pub fn set_checkpoint_files(
        &self,
        work_unit_id: &str,
        name: &str,
        files: Vec<codelet_rpc_types::ChangedFile>,
    ) {
        *self.core.scripted_checkpoint_files.lock() =
            Some((work_unit_id.to_string(), name.to_string(), files));
    }
    /// BUG-184: script the payload `checkpoint_file_diff(wu, name, path)`
    /// returns for ONE key.
    pub fn set_checkpoint_file_diff(
        &self,
        work_unit_id: &str,
        name: &str,
        path: &str,
        diff: Option<String>,
    ) {
        *self.core.scripted_checkpoint_file_diffs.lock() = Some((
            work_unit_id.to_string(),
            name.to_string(),
            path.to_string(),
            diff,
        ));
    }
    pub fn checkpoint_counts_calls(&self) -> usize {
        self.core.checkpoint_counts_calls.load(Ordering::SeqCst)
    }
    /// RPC-365: how many times `restore_checkpoint_file()` has been
    /// awaited.
    pub fn restore_checkpoint_file_calls(&self) -> usize {
        self.core
            .restore_checkpoint_file_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-365: the `(work_unit_id, name, path)` of the most recent
    /// `restore_checkpoint_file()` call, if any.
    pub fn last_restore_file(&self) -> Option<(String, String, String)> {
        self.core.last_restore_file.lock().clone()
    }
    /// RPC-365: how many times `restore_checkpoint_all()` has been
    /// awaited.
    pub fn restore_checkpoint_all_calls(&self) -> usize {
        self.core
            .restore_checkpoint_all_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-366: how many times `delete_checkpoint()` has been awaited.
    pub fn delete_checkpoint_calls(&self) -> usize {
        self.core.delete_checkpoint_calls.load(Ordering::SeqCst)
    }
    /// RPC-366: the `(work_unit_id, name)` of the most recent
    /// `delete_checkpoint()` call, if any.
    pub fn last_delete_checkpoint(&self) -> Option<(String, String)> {
        self.core.last_delete_checkpoint.lock().clone()
    }
    /// RPC-366: how many times `delete_all_checkpoints()` has been
    /// awaited.
    pub fn delete_all_checkpoints_calls(&self) -> usize {
        self.core
            .delete_all_checkpoints_calls
            .load(Ordering::SeqCst)
    }
}
