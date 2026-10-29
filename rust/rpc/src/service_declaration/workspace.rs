macro_rules! compose_service_workspace {
    ($($collected:tt)*) => {
        compose_service_providers_sessions! {
            $($collected)*
    /// Return every work unit currently known to the shared service impl.
    async fn list_work_units() -> Vec<WorkUnitInfo>;

    /// Return public metadata for every session currently tracked, filtered by
    /// `project_path`. RPC-427: added project_path parameter so `/resume` only
    /// shows sessions belonging to the current project.
    async fn list_sessions(project_path: String) -> Vec<SessionInfo>;

    /// BUG-206: return the IDs of every session that is ALIVE in the
    /// daemon's in-memory session map. This is the liveness gate for the
    /// TUI's auto-restore path — a restore candidate that is not in this
    /// list is closed/manifest-only and must never be re-opened.
    /// Deliberately NOT disk-aware (unlike `list_sessions`, which merges
    /// persisted manifests for the /resume picker).
    async fn list_live_sessions() -> Vec<SessionId>;

    /// Create a new session with optional role. Returns the freshly-minted
    /// session id.
    async fn create_session(role: Option<String>) -> SessionId;

    /// Send user input to a session. Returns immediately — streaming
    /// output arrives on the chunks broadcast channel exposed by both
    /// transports.
    async fn send_input(session_id: SessionId, text: String);

    /// Interrupt a running session. Returns immediately.
    async fn interrupt(session_id: SessionId);

    /// Return the current lifecycle state of a session.
    async fn get_session_status(session_id: SessionId) -> SessionStatus;

    /// RPC-011: return a live snapshot of the daemon's runtime health.
    /// Both transports route through this RPC — the embedded transport
    /// reads `ServerStats` directly via its own `FspecBackend::health`
    /// short-circuit; the WebSocket transport routes through tarpc.
    async fn health() -> HealthInfo;

    /// RPC-015: return manual + auto checkpoint counts aggregated across
    /// every work unit in the workspace. Delegates to
    /// `codelet_git::ghost_commit::count_checkpoints(cwd)` where `cwd`
    /// is the workspace root the shared service was constructed with.
    /// Returns `CheckpointCounts { manual: 0, auto: 0 }` when no cwd
    /// has been attached or the cwd is not a git repository.
    async fn checkpoint_counts() -> CheckpointCounts;

    /// RPC-355: return the list of changed working-tree files (staged +
    /// unstaged + untracked), each with a derived change type (A/M/D) and a
    /// `staged` flag. Delegates to the `codelet_git` change-type helpers
    /// using the workspace cwd attached via `with_cwd`. Returns an empty Vec
    /// when no cwd has been attached or the cwd is not a git repository —
    /// gated exactly like `checkpoint_counts`.
    async fn changed_files() -> Vec<ChangedFile>;

    /// RPC-355: return the unified diff text for a single changed file, or
    /// `None` when there is no diff (or no cwd is attached). Delegates to
    /// `codelet_git::diff::get_file_diff`.
    async fn file_diff(path: String) -> Option<String>;

    /// RPC-362: list every ghost-commit checkpoint across all work units,
    /// sorted most-recent-first and capped at 200. Delegates to
    /// `codelet_git::ghost_commit::list_all_ghost_checkpoints` + the metadata
    /// index reader. Returns an empty Vec when no cwd is attached.
    async fn list_checkpoints() -> Vec<CheckpointInfo>;

    /// RPC-362: list the files that differ between a checkpoint and the current
    /// working tree. Delegates to `get_checkpoint_diff_files`. Empty when no cwd.
    async fn checkpoint_diff_files(work_unit_id: String, name: String) -> Vec<ChangedFile>;

    /// RPC-362: unified diff text for one file against a checkpoint ref.
    /// Delegates to `codelet_git::get_checkpoint_file_diff`. `None` when no cwd
    /// or no diff.
    async fn checkpoint_file_diff(
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Option<String>;

    /// RPC-362: restore a single file from a checkpoint into the working tree.
    /// Returns `Err(String)` on failure or when no cwd is attached.
    async fn restore_checkpoint_file(
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<(), String>;

    /// RPC-362: restore the entire working tree to a checkpoint. Delegates to
    /// `restore_ghost_commit`. `Err(String)` on failure / no cwd.
    async fn restore_checkpoint_all(work_unit_id: String, name: String) -> Result<(), String>;

    /// RPC-362: delete one checkpoint (ref + index entry). `Err(String)` on
    /// failure / no cwd.
    async fn delete_checkpoint(work_unit_id: String, name: String) -> Result<(), String>;

    /// RPC-362: delete every checkpoint across all work units + unlink index
    /// sidecars. `Err(String)` on failure / no cwd.
    async fn delete_all_checkpoints() -> Result<(), String>;

    /// RPC-017: move the work unit with `id` one position UP in its
    /// current `states[<column>]` array. No-op at the top boundary.
    /// Returns `Err(String)` when the unit lives in the done column,
    /// when no cwd is attached to the shared service, or on I/O /
    /// data-integrity failure. The error string is serialised as
    /// part of the tarpc payload so both transports surface the
    /// same diagnostics to callers.
    async fn move_work_unit_up(id: String) -> Result<(), String>;

    /// RPC-017: mirror of [`move_work_unit_up`] for the DOWN direction.
    async fn move_work_unit_down(id: String) -> Result<(), String>;

    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Delegates through the attached
    /// `SessionManagerHandle`; when no session manager is attached the
    /// SAFE default (`ModelInfo::default()`) is returned. Both the
    /// SessionHeader and any future ModelSelector modal dialog consume
    /// this single source of truth.
    async fn get_model_info(session_id: SessionId) -> ModelInfo;

    /// RPC-018: return the per-session thinking/reasoning level.
    /// Default is `ThinkingLevel::Off`; the real wiring lands when the
    /// rust/napi `SessionManager` overrides the
    /// `SessionManagerHandle::get_thinking_level` trait method in RPC-022.
    async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel;

    /// RPC-018: return the workspace snapshot (cwd + optional git
    /// branch) for the workspace this shared service was constructed
    /// against. Built from the cwd attached via `with_cwd` plus
    /// `codelet_git::status::get_current_branch(cwd)`. When no cwd has
    /// been attached, falls back to `std::env::current_dir()` + a `None`
    /// branch so the SessionFooter still paints something sensible.
    async fn get_workspace_info() -> WorkspaceInfo;

    /// RPC-020: search the workspace for files whose path matches the
    /// case-insensitive substring `prefix` (using the same
    /// `**/*<prefix>*` glob as the TS @file popup). Returns at most
    /// `limit` paths sorted by modification time descending. Delegates
    /// to `codelet_core::file_search::search(cwd, prefix, limit)` where
    /// `cwd` is the workspace root attached via `with_cwd`. Returns an
    /// empty Vec when no cwd is attached or when no files match.
    async fn search_files(prefix: String, limit: u32) -> Vec<String>;

    /// RPC-025: append a submitted input to the session's command
    /// history. Delegates to `codelet_core::persistence::history::add`
    /// using the workspace cwd attached via `with_cwd` as the project
    /// filter. Returns `Err(String)` only on I/O / serialisation failure
    /// against the JSONL store.
    async fn persistence_add_history(session: SessionId, text: String) -> Result<(), String>;

    /// RPC-025: return the most recent `limit` history entries for the
    /// supplied `session`, newest-first. Delegates to
    /// `codelet_core::persistence::history::get` with the workspace cwd
    /// as the project filter.
    async fn persistence_get_history(session: SessionId, limit: u32)
        -> Result<Vec<String>, String>;

    /// RPC-025: case-insensitive substring search across history
    /// entries, scoped to the workspace cwd. Returns transport-portable
    /// `HistoryMatch` values whose `timestamp_iso` field is the
    /// RFC3339-formatted entry timestamp.
    async fn persistence_search_history(query: String) -> Result<Vec<HistoryMatch>, String>;

    /// RPC-026: delete an on-disk session manifest. Delegates to
    /// `codelet_core::persistence::delete_session` after parsing the
    /// `SessionId.value` field as a Uuid. Returns `Err(String)` only
    /// on I/O failure or parse failure; deleting an unknown id is
    /// idempotent (silently succeeds, matching the underlying core
    /// helper's contract).
    async fn persistence_delete_session(session: SessionId) -> Result<(), String>;
        }
    };
}
