macro_rules! compose_backend_workspace {
    ($($collected:tt)*) => {
        compose_backend_models! {
            $($collected)*
    /// RPC-017: move the work unit with `id` one position UP in its
    /// current `states[<column>]` array in `spec/work-units.json`.
    /// No-op at the top boundary. Returns `Err` when the unit lives
    /// in the done column, when no cwd is attached to the shared
    /// service, or on I/O / data-integrity failure.
    ///
    /// Both transports forward to the shared `FspecService` RPC
    /// method, which delegates to
    /// `codelet_core::work_units_write::move_work_unit`. After
    /// persistence the workspace's `WorkUnitsWatcher` fires a fresh
    /// snapshot — the App's existing subscriber task converts that
    /// into `Action::WorkUnitsLoaded` and re-seeds the BoardStore,
    /// keeping the focused-column selection on the moved unit (via
    /// RPC-016's auto-scroll math).
    async fn move_work_unit_up(&self, id: String) -> Result<()>;

    /// RPC-017: mirror of [`move_work_unit_up`] for the DOWN direction.
    async fn move_work_unit_down(&self, id: String) -> Result<()>;

    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Both transports delegate to
    /// `FspecService::get_model_info`; the AgentView's SessionHeader
    /// reads the response via `Action::ModelInfoLoaded`.
    async fn get_model_info(&self, session_id: SessionId) -> Result<ModelInfo>;

    /// RPC-018: return the per-session thinking/reasoning level.
    /// Both transports delegate to `FspecService::get_thinking_level`.
    async fn get_thinking_level(&self, session_id: SessionId) -> Result<ThinkingLevel>;

    /// RPC-018: return the workspace snapshot (cwd + optional git
    /// branch) for the workspace this shared service was constructed
    /// against. Both transports delegate to
    /// `FspecService::get_workspace_info` which in turn reads
    /// `codelet_git::status::get_current_branch(cwd)`.
    async fn get_workspace_info(&self) -> Result<WorkspaceInfo>;

    /// RPC-020: search the workspace for files whose path matches the
    /// case-insensitive substring `prefix`. Returns at most `limit`
    /// paths sorted by modification time desc. Both transports delegate
    /// to `FspecService::search_files` which in turn calls
    /// `codelet_core::file_search::search(cwd, prefix, limit)`. Returns
    /// an empty Vec when no cwd is attached to the shared service or
    /// when no files match.
    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>>;

    /// RPC-025: append a submitted input to the session's command
    /// history. Both transports forward to
    /// `FspecService::persistence_add_history`. Fire-and-forget at the
    /// App dispatch layer; the underlying tarpc call still returns
    /// Result so transport-level failures can be logged.
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>;

    /// RPC-025: return the most recent `limit` history entries for the
    /// supplied session, newest-first. Used by App::dispatch to
    /// snapshot the per-session history before walking with Shift+↑.
    async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>;

    /// RPC-025: case-insensitive substring search across the full
    /// history JSONL. Returns `HistoryMatch` values with an
    /// RFC3339-formatted timestamp so the @search popup can render
    /// "<text>  <relative time>" lines without a chrono dep.
    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>;

    /// RPC-026: delete an on-disk session manifest by id. Both
    /// transports forward to `FspecService::persistence_delete_session`
    /// which in turn calls `codelet_core::persistence::delete_session`.
    /// Idempotent — deleting an unknown id silently succeeds.
    async fn persistence_delete_session(&self, id: SessionId) -> Result<()>;
        }
    };
}
