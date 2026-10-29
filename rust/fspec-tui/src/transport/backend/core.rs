macro_rules! compose_backend_core {
    ($($collected:tt)*) => {
        compose_backend_workspace! {
            $($collected)*
    async fn list_provider_settings(
        &self,
    ) -> std::result::Result<Vec<codelet_rpc_types::ProviderSettingsConfig>, BackendError> {
        Err(BackendError::ProviderSettings(
            "unsupported backend".to_string(),
        ))
    }

    async fn save_provider_settings(
        &self,
        provider_id: String,
        definition: ProfileDefinition,
    ) -> std::result::Result<(), BackendError> {
        let _ = (provider_id, definition);
        Err(BackendError::ProviderSettings(
            "unsupported backend".to_string(),
        ))
    }

    /// List all known work units. Mirrors `FspecService::list_work_units`.
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>;

    /// List all known sessions, filtered by `project_path`. Mirrors
    /// `FspecService::list_sessions`. RPC-427: added `project_path` parameter
    /// so `/resume` only shows sessions belonging to the current project.
    async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>>;

    /// BUG-206: the IDs of every session ALIVE in the daemon's in-memory
    /// session map — the auto-restore liveness gate. Unlike
    /// `list_sessions` (which merges persisted manifests for the /resume
    /// picker) this NEVER reads disk: a candidate not in this list is
    /// closed/manifest-only and must not be re-opened. Both transports
    /// delegate to the shared `FspecService::list_live_sessions` RPC
    /// method. The default impl returns an empty Vec so test doubles
    /// compile unchanged (RPC-037 / PROV-109 convention) — and an empty
    /// list is also the honest answer for a backend with no live
    /// sessions, which is exactly the "nothing restorable → board" case.
    async fn list_live_sessions(&self) -> Result<Vec<SessionId>> {
        Ok(Vec::new())
    }

    /// Create a new session with an optional role overlay.
    async fn create_session(&self, role: Option<String>) -> Result<SessionId>;

    async fn session_manifest_issues(
        &self,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>> {
        Ok(Vec::new())
    }

    /// Append user input to the session with the given id.
    async fn send_input(&self, id: SessionId, text: String) -> Result<()>;

    /// Interrupt an in-flight session generation.
    async fn interrupt(&self, id: SessionId) -> Result<()>;

    /// Subscribe to broadcasted work-units snapshots (RPC-006). Each call
    /// returns a fresh receiver; senders fan out to all live receivers.
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;

    /// Subscribe to broadcasted session stream chunks (RPC-007).
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;

    /// Subscribe to broadcasted log records (RPC-007).
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;

    /// RPC-011: return a live snapshot of the daemon's runtime health.
    /// Embedded backends short-circuit and read `ServerStats` directly;
    /// the WebSocket backend routes through tarpc `FspecService::health`.
    async fn health(&self) -> Result<HealthInfo>;

    /// RPC-015: return manual + auto checkpoint counts aggregated across
    /// every work unit in the workspace. Both transports delegate to
    /// the shared `FspecService::checkpoint_counts` RPC method which
    /// in turn calls `codelet_git::ghost_commit::count_checkpoints`.
    async fn checkpoint_counts(&self) -> Result<CheckpointCounts>;

    /// RPC-355: return the list of changed working-tree files (staged +
    /// unstaged + untracked), each with a derived change type (A/M/D) and a
    /// `staged` flag. Both transports delegate to the shared
    /// `FspecService::changed_files` RPC method which builds the combined
    /// list from the `codelet_git` change-type helpers. The default impl
    /// returns an empty Vec so test doubles compile unchanged (RPC-037 /
    /// PROV-109 convention).
    async fn changed_files(&self) -> Result<Vec<ChangedFile>> {
        Ok(Vec::new())
    }

    /// RPC-355: return the unified diff text for one changed file, or `None`
    /// when there is no diff / no cwd. Both transports delegate to
    /// `FspecService::file_diff`. The default impl returns `None`.
    async fn file_diff(&self, _path: String) -> Result<Option<String>> {
        Ok(None)
    }

    /// RPC-362: list every ghost-commit checkpoint across all work units,
    /// sorted most-recent-first and capped at 200. Both transports delegate to
    /// `FspecService::list_checkpoints`. The default impl returns an empty Vec
    /// so test doubles compile unchanged.
    async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        Ok(Vec::new())
    }

    /// RPC-362: list the files that differ between a checkpoint and the working
    /// tree. Delegates to `FspecService::checkpoint_diff_files`. Default: empty.
    async fn checkpoint_diff_files(
        &self,
        _work_unit_id: String,
        _name: String,
    ) -> Result<Vec<ChangedFile>> {
        Ok(Vec::new())
    }

    /// RPC-362: unified diff text for one file against a checkpoint ref.
    /// Delegates to `FspecService::checkpoint_file_diff`. Default: `None`.
    async fn checkpoint_file_diff(
        &self,
        _work_unit_id: String,
        _name: String,
        _path: String,
    ) -> Result<Option<String>> {
        Ok(None)
    }

    /// RPC-362: restore a single file from a checkpoint into the working tree.
    /// Delegates to `FspecService::restore_checkpoint_file`. Default: `Ok(())`.
    async fn restore_checkpoint_file(
        &self,
        _work_unit_id: String,
        _name: String,
        _path: String,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-362: restore the entire working tree to a checkpoint. Delegates to
    /// `FspecService::restore_checkpoint_all`. Default: `Ok(())`.
    async fn restore_checkpoint_all(&self, _work_unit_id: String, _name: String) -> Result<()> {
        Ok(())
    }

    /// RPC-362: delete one checkpoint. Delegates to
    /// `FspecService::delete_checkpoint`. Default: `Ok(())`.
    async fn delete_checkpoint(&self, _work_unit_id: String, _name: String) -> Result<()> {
        Ok(())
    }

    /// RPC-362: delete every checkpoint across all work units. Delegates to
    /// `FspecService::delete_all_checkpoints`. Default: `Ok(())`.
    async fn delete_all_checkpoints(&self) -> Result<()> {
        Ok(())
    }
        }
    };
}
