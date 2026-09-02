//! codelet-rpc: the fspec tarpc service trait + the single shared service
//! implementation that both transports delegate to.
//!
//! Single source of truth for the RPC surface. Both the embedded transport
//! (`codelet-rpc-embedded`) and the WebSocket transport (`codelet-rpc-server`)
//! use the [`FspecServiceImpl`] type defined here — neither transport
//! inlines its own copy of the business logic (RPC-005 architecture rule
//! "service impl written ONCE in a shared module").
//!
//! ## RPC-006 watcher integration
//!
//! After RPC-006 the shared service reads from a real
//! [`codelet_core::work_units::WorkUnitsWatcher`] instead of the
//! hard-coded RPC-005 fixture.
//!
//! ## RPC-007 session integration
//!
//! After RPC-007 the shared service additionally holds an
//! `Arc<dyn SessionManagerHandle>` (concrete impl injected by the host —
//! `rust/napi` for the JS frontend, the rpc-server binary for the
//! WebSocket frontend, the embedded host for the ratatui frontend) plus
//! two `tokio::sync::broadcast::Sender` channels for `(SessionId, StreamChunk)`
//! and `LogRecord` events. Both transports observe the SAME senders so
//! NAPI is one listener, not the only listener.

use arc_swap::ArcSwap;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CheckpointsProgress, CompactionProgress, CompactionResult, CustomModelDefinition,
    FspecResult, HealthInfo, HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput,
    IsolatedSessionInfo, LogRecord, MergeOutcome, MergeStrategy, ModelEntry, ModelInfo,
    OAuthDeviceStart, OAuthHeadlessStart, PauseState, ProfileDefinition, ProviderCredentialInfo,
    ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob, SessionChangesSummary,
    SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo,
    StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState,
    ExecStdinRequest, WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tarpc::context::Context;
use tokio::sync::broadcast;
use tokio::sync::Mutex as AsyncMutex;

mod changed_files;
#[doc(hidden)]
pub mod checkpoints;
mod log_layer;
mod oauth_copilot;
mod oauth_disconnect;
mod oauth_login;
pub use log_layer::{register_log_layer, BroadcastLogLayer};

use changed_files::collect_changed_files;
/// The fspec RPC service surface.
///
/// All methods take a `tarpc::context::Context` (injected by the macro) and
/// return owned values that implement `serde::Serialize + Deserialize`.
#[tarpc::service]
pub trait FspecService {
    /// Return every work unit currently known to the shared service impl.
    async fn list_work_units() -> Vec<WorkUnitInfo>;

    /// Return public metadata for every session currently tracked, filtered by
    /// `project_path`. RPC-427: added project_path parameter so `/resume` only
    /// shows sessions belonging to the current project.
    async fn list_sessions(project_path: String) -> Vec<SessionInfo>;

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

    /// RPC-022: list every provider and its available models. Delegates
    /// through the optional `SessionManagerHandle::list_providers` —
    /// when no handle is attached the default returns
    /// `Vec::new()`. The rust/napi `SessionManager` override reads
    /// the cached `ModelRegistry` and maps each provider/model into
    /// the transport-portable `ProviderInfo` / `ModelEntry` shape.
    async fn list_providers() -> Vec<ProviderInfo>;

    /// RPC-022: set the model bound to a session. Delegates through
    /// the optional `SessionManagerHandle::set_model`. Returns
    /// `Err(String)` only when the underlying handle returns an error
    /// (e.g. unknown model). Without an attached handle returns
    /// `Ok(())` (silent no-op, idempotent like `send_input`).
    async fn set_session_model(
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<(), String>;

    /// PROV-118: set the in-process DEFAULT model used by `create_session`
    /// when no per-session model is bound. Delegates through the optional
    /// `SessionManagerHandle::set_default_model`. Without an attached handle
    /// returns `Ok(())` (silent no-op, idempotent like `set_session_model`).
    async fn set_default_model(model: String) -> Result<(), String>;

    /// RPC-347: add a NEW custom model to a local-server profile. Delegates
    /// through `SessionManagerHandle::add_custom_model` (which calls
    /// `profile_sections::save_custom_model(.., None)`). Without an attached
    /// handle returns `Ok(())` (silent no-op, idempotent like
    /// `set_session_model`).
    async fn add_custom_model(
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String>;

    /// RPC-347: UPDATE an existing custom model in place. `original_model_id`
    /// names the entry to replace. Delegates through
    /// `SessionManagerHandle::update_custom_model`; without a handle returns
    /// `Ok(())`.
    async fn update_custom_model(
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String>;

    /// RPC-347: DELETE a custom model from a local-server profile by id.
    /// Delegates through `SessionManagerHandle::delete_custom_model`; without
    /// a handle returns `Ok(())`.
    async fn delete_custom_model(
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<(), String>;

    /// PROV-108: create or update a local-server profile. Delegates through
    /// `SessionManagerHandle::save_profile` (read-modify-write preserving
    /// customModels + sibling keys). Without an attached handle returns
    /// `Ok(())` (silent no-op, idempotent like `set_session_model`).
    async fn save_profile(
        provider_id: String,
        profile_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String>;

    /// PROV-108: delete a local-server profile. Delegates through
    /// `SessionManagerHandle::delete_profile`; without a handle returns
    /// `Ok(())`.
    async fn delete_profile(provider_id: String, profile_name: String) -> Result<(), String>;

    /// PROV-136: rename a local-server profile (or in-place update when
    /// `old_name == new_name`). Delegates through
    /// `SessionManagerHandle::rename_profile` (single read-modify-write moving
    /// the old key to `new_name`, preserving customModels, rejecting a
    /// collision). Without an attached handle returns `Ok(())`.
    async fn rename_profile(
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String>;

    /// RPC-022: set the per-session thinking/reasoning level.
    /// Delegates through `SessionManagerHandle::set_thinking_level`.
    /// Returns `Err(String)` on handle error; without an attached
    /// handle returns `Ok(())`.
    async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>;

    /// CONT-002: set the session's auto-continue state (`/continue`).
    /// Delegates through `SessionManagerHandle::set_continue_state`.
    /// Without an attached handle returns `Ok(())`.
    async fn set_continue_state(
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String>;

    /// CONT-002: read the session's auto-continue state as
    /// `(enabled, budget)`. Without an attached handle returns
    /// `(false, 10)`.
    async fn get_continue_state(session_id: SessionId) -> (bool, u32);

    /// CONT-003: set or clear the session's goal chrome state (`/goal`)
    /// as `(text, verify)`. Delegates through
    /// `SessionManagerHandle::set_goal_state`. Without an attached
    /// handle returns `Ok(())`.
    async fn set_goal_state(
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String>;

    /// CONT-003: read the session's goal chrome state as
    /// `(text, verify)`. Without an attached handle returns `None`.
    async fn get_goal_state(session_id: SessionId) -> Option<(String, Option<String>)>;

    /// RPC-022: read the session's current role overlay text.
    /// Delegates through `SessionManagerHandle::get_role`. Without an
    /// attached handle returns `None`.
    async fn get_session_role(session_id: SessionId) -> Option<String>;

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears. Delegates through
    /// `SessionManagerHandle::set_role`. Without an attached handle
    /// returns `Ok(())` (silent no-op).
    async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>;

    // ========================================================================
    // RPC-037: Widened tarpc surface for AgentView parity. Each method below
    // mirrors an addition on `SessionManagerHandle`; `FspecServiceImpl`
    // delegates via `self.inner.session_manager()` and returns safe defaults
    // when no handle is attached.
    // ========================================================================

    /// RPC-037: send user input with provider-specific thinking config.
    async fn send_input_with_thinking(
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    );

    /// RPC-037: per-session input/output token totals.
    async fn get_session_tokens(session_id: SessionId) -> SessionTokens;

    /// RPC-037: per-session model binding (provider + model + limits).
    async fn get_session_model(session_id: SessionId) -> SessionModel;

    /// RPC-037: in-flight compaction progress, if any.
    async fn get_compaction_progress(session_id: SessionId) -> Option<CompactionProgress>;

    /// RPC-037: replay-buffer of recent stream chunks for a session.
    async fn get_buffered_output(session_id: SessionId, limit: u32) -> Vec<StreamChunk>;

    /// RPC-037: clear session history.
    async fn clear_history(session_id: SessionId) -> Result<(), String>;

    /// RPC-037: compact session history and return statistics.
    async fn compact_session(session_id: SessionId) -> Result<CompactionResult, String>;

    /// RPC-037: restore session messages from raw JSONL envelopes.
    async fn restore_session_messages(
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String>;

    /// RPC-037: restore cumulative-billed counters and cache totals.
    async fn restore_session_token_state(
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String>;

    /// RPC-049: durable-restore aggregate that orchestrates load_session
    /// + get_session_message_envelopes + restore_session_messages +
    /// restore_session_token_state in a single round-trip. Used by the
    /// TUI's `/resume` flow.
    async fn resume_session(session_id: SessionId) -> Result<(), String>;

    /// RPC-037: read the work-unit context bound to a session.
    async fn get_work_unit_context(session_id: SessionId) -> Option<WorkUnitContext>;

    /// RPC-037: bind (or detach) a work unit on a session.
    async fn set_work_unit_context(
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<(), String>;

    /// RPC-037: read the per-session pending input draft.
    async fn get_pending_input(session_id: SessionId) -> Option<String>;

    /// RPC-037: write the per-session pending input draft.
    async fn set_pending_input(session_id: SessionId, text: Option<String>);

    /// RPC-037: set the active session for the application.
    async fn set_active_session(session_id: SessionId);

    /// RPC-037: clear the active session.
    async fn clear_active_session();

    /// RPC-037: read the active session, if any.
    async fn get_active_session() -> Option<SessionId>;

    /// RPC-037: effective cwd for a session (worktree-aware). Returned
    /// as a String so the wire shape stays portable.
    async fn get_effective_cwd(session_id: SessionId) -> String;

    /// RPC-037: list supervisor session ids for a subordinate.
    async fn get_supervisors(session_id: SessionId) -> Vec<SessionId>;

    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. The production handle delegates into
    /// `ChainOfCommand::add_supervisor` and surfaces its
    /// "circular supervision not allowed" / "subordinate already
    /// registered under this supervisor" error strings verbatim.
    async fn add_supervisor(
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<(), String>;

    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor.
    async fn remove_supervisor(supervisor_id: SessionId) -> Result<(), String>;

    /// RPC-061: return the first subordinate registered to a
    /// supervisor. Backward-compatible accessor mirroring
    /// `ChainOfCommand::get_subordinate`.
    async fn get_subordinate(supervisor_id: SessionId) -> Option<SessionId>;

    /// RPC-061: list every subordinate of a supervisor. Mirrors
    /// `ChainOfCommand::get_subordinates`.
    async fn get_subordinates(supervisor_id: SessionId) -> Vec<SessionId>;

    /// RPC-061: queue an incoming supervisor message for a subordinate
    /// session. Production handle wraps this onto
    /// `BackgroundSession::receive_incoming_message`; bubbles the
    /// underlying `Err(String)` (e.g. "Failed to queue supervisor
    /// input: …") back to the caller.
    async fn receive_incoming_message(
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String>;

    /// RPC-037: debug-capture toggle reader.
    async fn get_debug_enabled(session_id: SessionId) -> bool;

    /// RPC-037: debug-capture toggle writer.
    async fn set_debug_enabled(session_id: SessionId, enabled: bool);

    /// RPC-037: toggle debug capture; returns the resolved path string.
    async fn toggle_debug(session_id: SessionId, debug_dir: String) -> Result<String, String>;

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Mirrors the NAPI
    /// `toggle_debug(Option<String>)` global helper.
    async fn set_debug_directory(path: String) -> Result<(), String>;

    /// RPC-037: resume a paused session.
    async fn pause_resume(session_id: SessionId) -> Result<(), String>;

    /// RPC-037: respond to a two-choice confirm pause.
    async fn pause_confirm(session_id: SessionId, accept: bool) -> Result<(), String>;

    /// RPC-037: respond to a three-choice approval pause.
    async fn pause_triple(session_id: SessionId, choice: ApprovalChoice) -> Result<(), String>;

    /// RPC-037: send a Human-In-The-Loop response.
    async fn send_hitl_response(
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<(), String>;

    /// RPC-037: snapshot of the pause dialog state.
    async fn get_pause_state(session_id: SessionId) -> Option<PauseState>;

    /// RPC-037: snapshot of the active HITL request, if any.
    async fn get_hitl_request(session_id: SessionId) -> Option<HitlRequest>;

    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure overlay — NO status flip, NO response channel.
    async fn get_exec_stdin_request(session_id: SessionId) -> Option<ExecStdinRequest>;

    /// TOOL-022 P2: write typed text to a live exec session's stdin
    /// (a trailing newline is appended when absent, matching the
    /// unified_exec `write` action semantics).
    async fn write_exec_stdin(
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String>;

    /// RPC-037: round-trip an FspecCommandRequest reply.
    async fn send_fspec_result(session_id: SessionId, result: FspecResult) -> Result<(), String>;

    /// RPC-037: create an isolated (worktree-backed) session.
    async fn create_isolated_session(role: Option<String>) -> Result<IsolatedSessionInfo, String>;

    /// RPC-037: per-user default thinking level (closes the pre-RPC-037
    /// gap on the tarpc surface).
    async fn set_thinking_level_default(
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String>;

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    async fn destroy_session(session_id: SessionId) -> Result<(), String>;

    // ========================================================================
    // RPC-054: Provider credentials surface — backs the new Rust ratatui
    // ProviderSettingsView (`/provider` slash command). Mirrors the
    // SessionManagerHandle trait additions of the same name.
    // ========================================================================

    /// RPC-054: list all known providers with configured / credential-type
    /// / model-count metadata.
    async fn list_provider_credentials() -> Vec<ProviderCredentialInfo>;

    /// RPC-054: return the credential summary for a single provider.
    async fn get_provider_credential(provider_id: String) -> Option<ProviderCredentialInfo>;

    /// RPC-054: persist credentials for a provider.
    async fn set_provider_credentials(
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<(), String>;

    /// RPC-054: clear credentials for a provider. Idempotent.
    async fn delete_provider_credentials(provider_id: String) -> Result<(), String>;

    /// RPC-054: perform a network round-trip to the provider's base
    /// URL and return latency + success metadata.
    async fn test_provider_connection(provider_id: String) -> Result<TestConnectionResult, String>;

    /// RPC-054: refresh the provider's cached model list and return
    /// the fresh `ModelEntry` list.
    async fn refresh_models_cache(provider_id: String) -> Result<Vec<ModelEntry>, String>;

    /// PROV-112: clear the OAuth tokens for `provider_id` (disconnect/logout).
    /// Dispatches by provider to the providers-direct clear primitives
    /// (anthropic→delete claude_auth.json, github-copilot→delete copilot
    /// credential, codex/fallback→strip the tokens field preserving
    /// OPENAI_API_KEY). Idempotent.
    async fn oauth_clear_tokens(provider_id: String) -> Result<(), String>;

    /// PROV-112: whether `provider_id` currently has OAuth tokens persisted.
    /// Drives the post-disconnect nav reload (the `oauth-status` row is shown
    /// only while this is `true`).
    async fn oauth_get_tokens(provider_id: String) -> Result<bool, String>;

    /// PROV-113: run the browser OAuth login for `provider_id` to completion
    /// (providers-layer local-server flow; tokens persisted on success).
    async fn oauth_browser_login(provider_id: String) -> Result<(), String>;

    /// PROV-113: phase 1 of the anthropic headless flow — generate PKCE +
    /// authorize URL (no network).
    async fn oauth_headless_start(provider_id: String) -> Result<OAuthHeadlessStart, String>;

    /// PROV-113: phase 2 of the anthropic headless flow — validate the pasted
    /// `code#state`, exchange for tokens, persist.
    async fn oauth_headless_complete(
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<(), String>;

    /// PROV-113: phase 1 of the codex device flow — request a device code.
    async fn oauth_device_start(provider_id: String) -> Result<OAuthDeviceStart, String>;

    /// PROV-113: phase 2 of the codex device flow — poll until authorized,
    /// then exchange + persist.
    async fn oauth_device_poll(
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<(), String>;

    /// PROV-114: phase 1 of the github-copilot device flow — request a device
    /// code against github.com (`None`) or a normalized enterprise host.
    async fn oauth_copilot_device_start(
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart, String>;

    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the left pane of
    /// `BlocklistView` in the Rust ratatui frontend.
    async fn blocklist_list() -> Vec<BlocklistRuleInfo>;

    /// RPC-057: merge a session's worktree back to base. Strategy is
    /// reserved for future evolution.
    async fn merge_session_worktree(
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String>;

    /// RPC-057: discard a session's worktree changes.
    async fn discard_session_worktree(session_id: SessionId) -> Result<(), String>;

    /// RPC-057: prune orphaned session worktrees; returns the pruned
    /// session ids.
    async fn prune_orphaned_worktrees() -> Result<Vec<String>, String>;

    /// RPC-057: list every known session worktree.
    async fn list_session_worktrees() -> Vec<SessionWorktreeInfo>;

    /// RPC-057: inspect a session's pending change summary.
    async fn inspect_session_changes(
        session_id: SessionId,
    ) -> Result<SessionChangesSummary, String>;

    /// RPC-058: persist a new scheduled job.
    async fn schedule_add(job: ScheduledJob) -> Result<ScheduledJob, String>;

    /// RPC-058: list every persisted scheduled job.
    async fn schedule_list() -> Vec<ScheduledJob>;

    /// RPC-058: flip a job's status to `paused`.
    async fn schedule_pause(name: String) -> Result<ScheduledJob, String>;

    /// RPC-058: flip a job's status to `active`.
    async fn schedule_resume(name: String) -> Result<ScheduledJob, String>;

    /// RPC-058: remove a job from `spec/schedules.json`.
    async fn schedule_remove(name: String) -> Result<(), String>;

    /// RPC-059: register a session-scoped recurring prompt.
    async fn loop_add(
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String>;

    /// RPC-059: cancel a registered loop. Returns true when a matching
    /// loop existed and was removed.
    async fn loop_cancel(id: String) -> Result<bool, String>;

    /// RPC-059: list every loop registered against a session.
    async fn loop_list(session_id: SessionId) -> Vec<RegisteredLoop>;
}

/// RPC-011 broadcast capacity for the StreamChunk channel — sized to
/// absorb sustained token-delta storms across multiple connected
/// clients. Bumped from 256 → 1024 alongside the multi-client
/// hardening work in RPC-011.
pub const DEFAULT_CHUNKS_CAPACITY: usize = 1024;
/// RPC-011 broadcast capacity for the LogRecord channel — sized for
/// tracing storms plus per-client lag warnings riding the same
/// channel. Bumped from 1024 → 4096 alongside the multi-client
/// hardening work in RPC-011.
pub const DEFAULT_LOGS_CAPACITY: usize = 4096;
/// RPC-011 broadcast capacity for the work-units update channel —
/// snapshot replacement semantics mean a lagging subscriber simply
/// resyncs from the latest snapshot, so a small capacity is fine.
pub const DEFAULT_WORK_UNITS_CAPACITY: usize = 256;
/// TUI-109 broadcast capacity for the checkpoint-enumeration progress
/// channel — one frame per item (plus the done frame); a lagging
/// subscriber simply drops intermediate frames (the final done frame
/// still lands, and the list RPC itself carries the full result).
pub const DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY: usize = 256;

/// RPC-011: read-only handle to per-server runtime stats so the shared
/// service can answer `health()` from BOTH transports. The concrete
/// `ServerStats` lives in `codelet-rpc-server`; this trait abstracts
/// only the read-side accessors `health()` needs so the rpc crate
/// stays free of a server-side dependency.
pub trait ServerStatsHandle: Send + Sync + std::fmt::Debug {
    fn connected_clients(&self) -> u64;
    fn last_watcher_event_secs_ago(&self) -> Option<u64>;
    fn lag_chunks(&self) -> u64;
    fn lag_logs(&self) -> u64;
    fn lag_work_units(&self) -> u64;
}

/// The shared `FspecService` state.
///
/// Holds the workspace watcher, the session manager handle (RPC-007),
/// the per-process broadcast senders for streaming chunks and log
/// records, and a per-process invocation counter. RPC-011 additionally
/// records `started_at` (used by `health()` for uptime_secs) and an
/// optional `stats` accessor wired in by the host transport.
pub struct SharedFspecService {
    /// RPC-011 rule [26]: the watcher slot is wrapped in
    /// [`arc_swap::ArcSwap`] so the daemon's SIGHUP handler can
    /// atomically replace it with a freshly-built `WorkUnitsWatcher`
    /// without blocking concurrent `list_work_units` readers. Lock-free
    /// reads via `.load()` are cheaper than `RwLock` and the swap is a
    /// rare event (only on SIGHUP).
    watcher: ArcSwap<WorkUnitsWatcher>,
    session_manager: Option<Arc<dyn SessionManagerHandle>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    list_work_units_calls: Arc<AtomicU64>,
    /// RPC-011: process startup instant — `health()` reports
    /// `(now - started_at).as_secs()` as `uptime_secs`.
    started_at: Instant,
    /// RPC-011: read-only stats accessor wired in by the host
    /// transport (e.g. `bind_and_serve` for the WebSocket server).
    /// `None` for in-process embedded callers that don't run a server
    /// — in that case `health()` returns zeroed counters.
    stats: AsyncMutex<Option<Arc<dyn ServerStatsHandle>>>,
    /// RPC-015: workspace cwd used by `checkpoint_counts()` to walk
    /// `refs/fspec-checkpoints/...`. `None` for in-process callers
    /// that don't care about checkpoint counts (e.g. RPC-005..014
    /// tests) — in that case `checkpoint_counts()` returns
    /// `CheckpointCounts::default()`.
    cwd: Option<PathBuf>,
    /// TUI-109: per-process broadcast sender for checkpoint-enumeration
    /// progress frames. `FspecServiceImpl::list_checkpoints` publishes a
    /// frame per collected item (plus the done frame); the embedded
    /// transport's `checkpoints_progress_rx()` surfaces the receiver to
    /// the TUI. Transports that don't forward the frames degrade to
    /// spinner-only automatically (no producer → no frames).
    checkpoints_progress_tx: broadcast::Sender<CheckpointsProgress>,
}

impl SharedFspecService {
    /// Construct the shared impl with a real workspace watcher (RPC-006).
    /// The session manager handle is left unset — calls to the session
    /// RPCs will return empty defaults until [`with_session_manager`] is
    /// used instead.
    pub fn new(watcher: Arc<WorkUnitsWatcher>) -> Self {
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        let (checkpoints_progress_tx, _) =
            broadcast::channel(DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: None,
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
            cwd: None,
            checkpoints_progress_tx,
        }
    }

    /// Construct the shared impl with both a workspace watcher and a
    /// session manager handle (RPC-007). The host (rpc-server bin,
    /// EmbeddedTransport host, or rust/napi) constructs the concrete
    /// SessionManager and hands it here as `Arc<dyn SessionManagerHandle>`.
    pub fn with_session_manager(
        watcher: Arc<WorkUnitsWatcher>,
        session_manager: Arc<dyn SessionManagerHandle>,
    ) -> Self {
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        let (checkpoints_progress_tx, _) =
            broadcast::channel(DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: Some(session_manager),
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
            cwd: None,
            checkpoints_progress_tx,
        }
    }

    /// RPC-015: chainable builder method that attaches a workspace cwd
    /// to a freshly-constructed [`SharedFspecService`]. The cwd is
    /// passed into `codelet_git::ghost_commit::count_checkpoints` by
    /// `FspecService::checkpoint_counts`. When no cwd is attached,
    /// `checkpoint_counts()` returns the zero default.
    ///
    /// Example:
    /// ```ignore
    /// let service = Arc::new(
    ///     SharedFspecService::new(watcher).with_cwd(workspace_root.to_path_buf()),
    /// );
    /// ```
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// RPC-015: borrow the workspace cwd attached via [`with_cwd`].
    /// Returns `None` when no cwd has been attached.
    pub fn cwd(&self) -> Option<&PathBuf> {
        self.cwd.as_ref()
    }

    /// RPC-011 rule [25]/[26]: atomically replace the watcher with a
    /// freshly-built one. Called by the daemon's SIGHUP handler so the
    /// workspace is re-walked without restarting the process or
    /// dropping any in-flight RPCs. Existing broadcast subscribers
    /// (tied to the OLD watcher's `subscribe()` receiver) stop seeing
    /// updates after the swap — they observe the silence as "watcher
    /// re-armed" and resync via `list_work_units_snapshot()`.
    pub fn rebuild_watcher(&self, new_watcher: WorkUnitsWatcher) {
        self.watcher.store(Arc::new(new_watcher));
    }

    /// RPC-011: wire a `ServerStatsHandle` into the shared service so
    /// `health()` can return live counters. Called by the host
    /// transport (currently `bind_and_serve`) once it has constructed
    /// `ServerStats`. Idempotent — the last call wins.
    pub async fn set_stats(&self, stats: Arc<dyn ServerStatsHandle>) {
        let mut guard = self.stats.lock().await;
        *guard = Some(stats);
    }

    /// RPC-011: return the process startup instant. Exposed so the
    /// host transport (`bind_and_serve`) can pass the SAME instant
    /// into `ServerStats` so client-visible `uptime_secs` and any
    /// server-side bookkeeping agree.
    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Return the current snapshot from the watcher and increment the
    /// parity counter.
    pub fn list_work_units_snapshot(&self) -> Vec<WorkUnitInfo> {
        self.list_work_units_calls.fetch_add(1, Ordering::SeqCst);
        self.watcher.load().snapshot()
    }

    /// Read the list_work_units invocation counter.
    pub fn list_work_units_calls(&self) -> u64 {
        self.list_work_units_calls.load(Ordering::SeqCst)
    }

    /// Subscribe to live work-units updates from the underlying watcher.
    pub fn watcher_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.watcher.load().subscribe()
    }

    /// Snapshot the current watcher state without incrementing the
    /// parity counter — used by the WS fan-out task on connect to send
    /// the initial snapshot frame.
    pub fn watcher_snapshot(&self) -> Vec<WorkUnitInfo> {
        self.watcher.load().snapshot()
    }

    /// Subscribe to the `(SessionId, StreamChunk)` broadcast (RPC-007).
    /// Both transports drain this same broadcast — the embedded transport
    /// returns the receiver directly to callers, the WS server's
    /// per-connection chunks_fanout task drains it and emits
    /// `Envelope::Event` frames.
    ///
    /// When a session manager is attached, subscribes to its
    /// per-process broadcast so all listeners — NAPI, embedded callers,
    /// WS fan-out — see the same chunks. Without a session manager,
    /// subscribes to a local broadcast that no producer publishes to
    /// (yields nothing).
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        match &self.session_manager {
            Some(handle) => handle.chunks_rx(),
            None => self.chunks_tx.subscribe(),
        }
    }

    /// Cloneable handle to the chunks broadcast sender — used by the
    /// session manager implementation (and the NAPI co-listener) to
    /// publish new chunks. Delegates to the session manager when
    /// attached so all subscribers see the same broadcast.
    pub fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        match &self.session_manager {
            Some(handle) => handle.chunks_tx(),
            None => self.chunks_tx.clone(),
        }
    }

    /// Subscribe to the `LogRecord` broadcast (RPC-007).
    ///
    /// Mirrors `chunks_rx` — when a session manager is attached, returns
    /// the session manager's own logs broadcast so all subscribers
    /// (NAPI co-listener, embedded callers, WS fan-out) see the same
    /// records.
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        match &self.session_manager {
            Some(handle) => handle.logs_rx(),
            None => self.logs_tx.subscribe(),
        }
    }

    /// Cloneable handle to the logs broadcast sender — used by the
    /// host's tracing::Layer to publish structured events. Delegates to
    /// the session manager when attached so the layer publishes onto
    /// the same broadcast that listeners observe via `logs_rx`.
    pub fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        match &self.session_manager {
            Some(handle) => handle.logs_tx(),
            None => self.logs_tx.clone(),
        }
    }

    /// TUI-109: subscribe to the checkpoint-enumeration progress
    /// broadcast. The embedded transport returns the receiver directly
    /// to the TUI; transports that don't forward the frames (e.g. the
    /// WebSocket transport) degrade to spinner-only automatically —
    /// no producer on the other side, no frames, no timeout logic.
    pub fn checkpoints_progress_rx(&self) -> broadcast::Receiver<CheckpointsProgress> {
        self.checkpoints_progress_tx.subscribe()
    }

    /// TUI-109: cloneable handle to the checkpoint-enumeration progress
    /// broadcast sender — `FspecServiceImpl::list_checkpoints` publishes
    /// frames here during enumeration.
    pub fn checkpoints_progress_tx(&self) -> broadcast::Sender<CheckpointsProgress> {
        self.checkpoints_progress_tx.clone()
    }

    /// RPC-037: Subscribe to the `(SessionId, SessionStatus)` broadcast
    /// that carries push-driven session status updates. The embedded
    /// transport returns the receiver directly to callers; the WS
    /// server's per-connection `status_changes_fanout` task drains it
    /// and emits `Envelope::StatusUpdate` frames.
    ///
    /// When a session manager is attached, delegates to its
    /// per-process broadcast so all listeners — NAPI, embedded callers,
    /// WS fan-out — see the same status changes. Without a session
    /// manager, returns a degenerate receiver whose sender has been
    /// dropped (subscribers immediately observe `RecvError::Closed`).
    pub fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match &self.session_manager {
            Some(handle) => handle.status_changes_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// Access the session manager handle, if one was provided.
    pub fn session_manager(&self) -> Option<&Arc<dyn SessionManagerHandle>> {
        self.session_manager.as_ref()
    }

    /// RPC-385: subscribe to session-created events. When a session manager is
    /// attached, delegates to its per-process broadcast so the embedded TUI
    /// sees every newly-created session (including spawned subordinates).
    /// Without a session manager, returns a degenerate receiver whose sender
    /// has been dropped (subscribers immediately observe `RecvError::Closed`).
    pub fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        match &self.session_manager {
            Some(handle) => handle.session_created_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }
}

/// Cloneable adapter that lets tarpc serve `FspecService` against a single
/// `Arc<SharedFspecService>` instance without `Clone`-ing the underlying
/// state (only the `Arc` is cloned). This is the type that BOTH the
/// embedded transport and the WebSocket server pass to `BaseChannel::execute`.
#[derive(Clone)]
pub struct FspecServiceImpl {
    pub inner: Arc<SharedFspecService>,
}

impl FspecServiceImpl {
    /// Wrap a shared service in the tarpc-servable adapter.
    pub fn new(inner: Arc<SharedFspecService>) -> Self {
        Self { inner }
    }
}

impl FspecService for FspecServiceImpl {
    async fn list_work_units(self, _ctx: Context) -> Vec<WorkUnitInfo> {
        self.inner.list_work_units_snapshot()
    }

    async fn list_sessions(self, _ctx: Context, project_path: String) -> Vec<SessionInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_sessions(&project_path),
            None => Vec::new(),
        }
    }

    async fn create_session(self, _ctx: Context, role: Option<String>) -> SessionId {
        match self.inner.session_manager() {
            Some(handle) => handle.create_session(role),
            None => SessionId::new("rpc-no-session-manager"),
        }
    }

    async fn send_input(self, _ctx: Context, session_id: SessionId, text: String) {
        if let Some(handle) = self.inner.session_manager() {
            handle.send_input(&session_id, text);
        }
    }

    async fn interrupt(self, _ctx: Context, session_id: SessionId) {
        if let Some(handle) = self.inner.session_manager() {
            handle.interrupt(&session_id);
        }
    }

    async fn get_session_status(self, _ctx: Context, session_id: SessionId) -> SessionStatus {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_status(&session_id),
            None => SessionStatus::Idle,
        }
    }

    async fn health(self, _ctx: Context) -> HealthInfo {
        // RPC-011 question [12]: HealthInfo fields are typed `i64` (not
        // `u64`) so the cfg-gated `napi(object)` compiles under
        // napi-derive v3 + `napi4` feature. ServerStats keeps its
        // natural `u64` accessors; we cast at the RPC boundary.
        let uptime_secs = self.inner.started_at.elapsed().as_secs() as i64;
        let stats = self.inner.stats.lock().await.clone();
        let (connected_clients, last_watcher_event_secs_ago, lag_chunks, lag_logs, lag_work_units) =
            match stats.as_ref() {
                Some(s) => (
                    s.connected_clients() as i64,
                    s.last_watcher_event_secs_ago().map(|n| n as i64),
                    s.lag_chunks() as i64,
                    s.lag_logs() as i64,
                    s.lag_work_units() as i64,
                ),
                None => (0, None, 0, 0, 0),
            };
        HealthInfo {
            uptime_secs,
            connected_clients,
            last_watcher_event_secs_ago,
            lag_chunks,
            lag_logs,
            lag_work_units,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    async fn checkpoint_counts(self, _ctx: Context) -> CheckpointCounts {
        // RPC-015: delegate to the shared git helper when a workspace
        // cwd has been attached. Without a cwd (e.g. RPC-005..014
        // tests), return the zero default to preserve backward
        // compatibility.
        match self.inner.cwd() {
            Some(cwd) => codelet_git::ghost_commit::count_checkpoints(cwd).unwrap_or_default(),
            None => CheckpointCounts::default(),
        }
    }

    async fn changed_files(self, _ctx: Context) -> Vec<ChangedFile> {
        // RPC-355: gated on the attached cwd exactly like checkpoint_counts.
        match self.inner.cwd() {
            Some(cwd) => collect_changed_files(cwd).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    async fn file_diff(self, _ctx: Context, path: String) -> Option<String> {
        // RPC-355: delegate to the shared diff helper; None when no cwd,
        // no diff, or the file is missing (deleted) so the UI degrades
        // gracefully rather than surfacing an error.
        match self.inner.cwd() {
            Some(cwd) => codelet_git::get_file_diff(cwd, &path).ok().flatten(),
            None => None,
        }
    }

    async fn list_checkpoints(self, _ctx: Context) -> Vec<CheckpointInfo> {
        // RPC-362: gated on the attached cwd exactly like checkpoint_counts.
        // TUI-109: drive the streaming variant with a callback that
        // publishes a progress frame on the shared broadcast; the final
        // Vec still returns through the same RPC.
        match self.inner.cwd() {
            Some(cwd) => {
                let tx = self.inner.checkpoints_progress_tx();
                checkpoints::collect_checkpoints_stream(cwd, &mut |frame| {
                    // Best-effort: no live subscribers (or a lagging
                    // subscriber) is a no-op, never an error.
                    let _ = tx.send(frame);
                })
                .unwrap_or_default()
            }
            None => Vec::new(),
        }
    }

    async fn checkpoint_diff_files(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Vec<ChangedFile> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::collect_checkpoint_diff_files(cwd, &work_unit_id, &name)
                .unwrap_or_default(),
            None => Vec::new(),
        }
    }

    async fn checkpoint_file_diff(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Option<String> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::checkpoint_file_diff(cwd, &work_unit_id, &name, &path)
                .ok()
                .flatten(),
            None => None,
        }
    }

    async fn restore_checkpoint_file(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::restore_file(cwd, &work_unit_id, &name, &path)
                .map_err(|e| format!("{e}")),
            None => Err(
                "restore_checkpoint_file requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn restore_checkpoint_all(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => {
                checkpoints::restore_all(cwd, &work_unit_id, &name).map_err(|e| format!("{e}"))
            }
            None => Err(
                "restore_checkpoint_all requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn delete_checkpoint(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => {
                checkpoints::delete_one(cwd, &work_unit_id, &name).map_err(|e| format!("{e}"))
            }
            None => Err(
                "delete_checkpoint requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn delete_all_checkpoints(self, _ctx: Context) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::delete_all(cwd).map_err(|e| format!("{e}")),
            None => Err(
                "delete_all_checkpoints requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn move_work_unit_up(self, _ctx: Context, id: String) -> Result<(), String> {
        // RPC-017: delegate to the shared work-units write helper.
        // Errors are stringified at the RPC boundary so both transports
        // see the same diagnostic text.
        match self.inner.cwd() {
            Some(cwd) => codelet_core::work_units_write::move_work_unit(
                cwd,
                &id,
                codelet_core::work_units_write::Direction::Up,
            )
            .map_err(|e| format!("{e:#}")),
            None => Err(
                "move_work_unit_up requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn move_work_unit_down(self, _ctx: Context, id: String) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => codelet_core::work_units_write::move_work_unit(
                cwd,
                &id,
                codelet_core::work_units_write::Direction::Down,
            )
            .map_err(|e| format!("{e:#}")),
            None => Err(
                "move_work_unit_down requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn get_model_info(self, _ctx: Context, session_id: SessionId) -> ModelInfo {
        // RPC-018: delegate via the optional SessionManagerHandle. The
        // trait carries a default impl that returns `ModelInfo::default()`,
        // so callers without an attached handle (test fixtures, embedded
        // hosts without a session manager) get the safe sentinel.
        match self.inner.session_manager() {
            Some(handle) => handle.get_model_info(&session_id),
            None => ModelInfo::default(),
        }
    }

    async fn get_thinking_level(self, _ctx: Context, session_id: SessionId) -> ThinkingLevel {
        // RPC-018: mirror of `get_model_info`. Default = `ThinkingLevel::Off`.
        match self.inner.session_manager() {
            Some(handle) => handle.get_thinking_level(&session_id),
            None => ThinkingLevel::Off,
        }
    }

    async fn get_workspace_info(self, _ctx: Context) -> WorkspaceInfo {
        // RPC-018: build the workspace snapshot from the attached cwd +
        // `codelet_git::status::get_current_branch`. When no cwd is
        // attached we fall back to `std::env::current_dir()` for the
        // cwd string but DELIBERATELY skip the git probe so the
        // SessionFooter degrades to a bare-cwd render (no `[⌥ branch]`).
        match self.inner.cwd() {
            Some(cwd_buf) => {
                let cwd_buf = cwd_buf.clone();
                let git_branch = codelet_git::status::get_current_branch(&cwd_buf)
                    .ok()
                    .flatten();
                WorkspaceInfo {
                    cwd: cwd_buf.to_string_lossy().into_owned(),
                    git_branch,
                }
            }
            None => {
                let cwd_buf = std::env::current_dir().unwrap_or_default();
                WorkspaceInfo {
                    cwd: cwd_buf.to_string_lossy().into_owned(),
                    git_branch: None,
                }
            }
        }
    }

    async fn search_files(self, _ctx: Context, prefix: String, limit: u32) -> Vec<String> {
        // RPC-020: delegate to `codelet_core::file_search::search` when
        // a workspace cwd is attached. Without a cwd we return an empty
        // Vec — both transports surface the same "no matches" signal
        // and the @file popup degrades gracefully.
        if prefix.is_empty() {
            return Vec::new();
        }
        match self.inner.cwd() {
            Some(cwd) => codelet_core::file_search::search(cwd, &prefix, limit),
            None => Vec::new(),
        }
    }

    async fn persistence_add_history(
        self,
        _ctx: Context,
        session: SessionId,
        text: String,
    ) -> Result<(), String> {
        // RPC-025: delegate to the lifted core helper. Preserve the
        // original SessionId string via `session_id_str` so non-UUID
        // SessionIds round-trip back through `HistoryMatch` unchanged;
        // the Uuid field is best-effort parsed-or-nil for legacy NAPI
        // / JSONL compatibility.
        let session_uuid =
            uuid::Uuid::parse_str(&session.value).unwrap_or_else(|_| uuid::Uuid::nil());
        let project = self
            .inner
            .cwd()
            .cloned()
            .unwrap_or_else(|| std::path::PathBuf::from(""));
        let entry = codelet_core::persistence::HistoryEntry::with_session_id_str(
            text,
            project,
            session_uuid,
            session.value,
        );
        codelet_core::persistence::history::add(entry)
    }

    async fn persistence_get_history(
        self,
        _ctx: Context,
        _session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        // RPC-025: scope to the workspace cwd if attached; otherwise return
        // entries across all projects. The current contract returns `Vec<String>`
        // (display values only) — search returns richer HistoryMatch values.
        let project = self.inner.cwd().cloned();
        let proj_ref = project.as_deref();
        codelet_core::persistence::history::get(proj_ref, Some(limit as usize))
            .map(|entries| entries.into_iter().map(|e| e.display).collect())
    }

    async fn persistence_search_history(
        self,
        _ctx: Context,
        query: String,
    ) -> Result<Vec<HistoryMatch>, String> {
        // RPC-025: scope to workspace cwd if attached; convert each
        // matching HistoryEntry into a HistoryMatch with an RFC3339
        // timestamp so non-Rust consumers don't need chrono.
        let project = self.inner.cwd().cloned();
        let proj_ref = project.as_deref();
        codelet_core::persistence::history::search(&query, proj_ref).map(|entries| {
            entries
                .iter()
                .map(codelet_core::persistence::HistoryEntry::to_history_match)
                .collect()
        })
    }

    async fn persistence_delete_session(
        self,
        _ctx: Context,
        session: SessionId,
    ) -> Result<(), String> {
        // RPC-026: parse the SessionId string as a Uuid and delegate to
        // the lifted core helper. Non-UUID SessionIds fall back to
        // Uuid::nil() — same parse-or-nil pattern as
        // persistence_add_history — so synthetic test ids (e.g.
        // "s-1") still round-trip through the call without panicking
        // at the boundary. The core helper is idempotent for unknown
        // ids so the worst case is a silent no-op.
        let uuid = uuid::Uuid::parse_str(&session.value).unwrap_or_else(|_| uuid::Uuid::nil());
        codelet_core::persistence::delete_session(uuid)
    }

    async fn list_providers(self, _ctx: Context) -> Vec<ProviderInfo> {
        // RPC-022: delegate to the optional SessionManagerHandle. The
        // trait carries a default impl returning `Vec::new()`, so
        // callers without an attached handle get the safe sentinel —
        // matching the same defaulting pattern used by RPC-018
        // `get_model_info`.
        match self.inner.session_manager() {
            Some(handle) => handle.list_providers(),
            None => Vec::new(),
        }
    }

    async fn set_session_model(
        self,
        _ctx: Context,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        // Default impl returns `Ok(())` — silent no-op for the
        // no-handle case.
        match self.inner.session_manager() {
            Some(handle) => handle.set_model(&session_id, &provider_id, &model_id),
            None => Ok(()),
        }
    }

    /// PROV-118: delegate the in-process default-model write through the
    /// optional SessionManagerHandle; no-handle path is a silent no-op.
    async fn set_default_model(self, _ctx: Context, model: String) -> Result<(), String> {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_default_model(&model);
        }
        Ok(())
    }

    // RPC-347: custom-model write surface. Each delegates through the optional
    // SessionManagerHandle; the no-handle path returns `Ok(())` (silent no-op,
    // matching set_session_model).
    async fn add_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.add_custom_model(&provider_id, &profile_name, &definition),
            None => Ok(()),
        }
    }

    async fn update_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        original_model_id: String,
        definition: CustomModelDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.update_custom_model(
                &provider_id,
                &profile_name,
                &original_model_id,
                &definition,
            ),
            None => Ok(()),
        }
    }

    async fn delete_custom_model(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        model_id: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_custom_model(&provider_id, &profile_name, &model_id),
            None => Ok(()),
        }
    }

    async fn save_profile(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.save_profile(&provider_id, &profile_name, &definition),
            None => {
                let msg = format!(
                    "save_profile({provider_id}/{profile_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }

    async fn delete_profile(
        self,
        _ctx: Context,
        provider_id: String,
        profile_name: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_profile(&provider_id, &profile_name),
            None => {
                let msg = format!(
                    "delete_profile({provider_id}/{profile_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }

    async fn rename_profile(
        self,
        _ctx: Context,
        provider_id: String,
        old_name: String,
        new_name: String,
        definition: ProfileDefinition,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.rename_profile(&provider_id, &old_name, &new_name, &definition),
            None => {
                let msg = format!(
                    "rename_profile({provider_id}/{old_name}→{new_name}): no session manager attached"
                );
                tracing::error!(%msg);
                Err(msg)
            }
        }
    }

    async fn set_thinking_level(
        self,
        _ctx: Context,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_thinking_level(&session_id, level),
            None => Ok(()),
        }
    }

    async fn set_continue_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        enabled: bool,
        budget: u32,
    ) -> Result<(), String> {
        // CONT-002: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_continue_state(&session_id, enabled, budget),
            None => Ok(()),
        }
    }

    async fn get_continue_state(self, _ctx: Context, session_id: SessionId) -> (bool, u32) {
        // CONT-002: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.get_continue_state(&session_id),
            None => (false, 10),
        }
    }

    async fn set_goal_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) -> Result<(), String> {
        // CONT-003: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_goal_state(&session_id, goal),
            None => Ok(()),
        }
    }

    async fn get_goal_state(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<(String, Option<String>)> {
        // CONT-003: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.get_goal_state(&session_id),
            None => None,
        }
    }

    async fn get_session_role(self, _ctx: Context, session_id: SessionId) -> Option<String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        // Default impl returns `None` so callers without an attached
        // handle paint no banner — matching the RPC-018 safe-defaults
        // pattern.
        match self.inner.session_manager() {
            Some(handle) => handle.get_role(&session_id),
            None => None,
        }
    }

    async fn set_session_role(
        self,
        _ctx: Context,
        session_id: SessionId,
        role: Option<String>,
    ) -> Result<(), String> {
        // RPC-022: delegate through the optional SessionManagerHandle.
        match self.inner.session_manager() {
            Some(handle) => handle.set_role(&session_id, role),
            None => Ok(()),
        }
    }

    // ========================================================================
    // RPC-037: Widened tarpc surface implementations. Each delegates through
    // the optional SessionManagerHandle; safe defaults are returned when no
    // handle is attached so existing handle-less tests stay green.
    // ========================================================================

    async fn send_input_with_thinking(
        self,
        _ctx: Context,
        session_id: SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) {
        if let Some(handle) = self.inner.session_manager() {
            handle.send_input_with_thinking(&session_id, text, thinking);
        }
    }

    async fn get_session_tokens(self, _ctx: Context, session_id: SessionId) -> SessionTokens {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_tokens(&session_id),
            None => SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            },
        }
    }

    async fn get_session_model(self, _ctx: Context, session_id: SessionId) -> SessionModel {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_model(&session_id),
            None => SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: 0,
                max_output_tokens: 0,
                compaction_threshold: 0,
            },
        }
    }

    async fn get_compaction_progress(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<CompactionProgress> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_compaction_progress(&session_id),
            None => None,
        }
    }

    async fn get_buffered_output(
        self,
        _ctx: Context,
        session_id: SessionId,
        limit: u32,
    ) -> Vec<StreamChunk> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_buffered_output(&session_id, limit),
            None => Vec::new(),
        }
    }

    async fn clear_history(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.clear_history(&session_id),
            None => Ok(()),
        }
    }

    async fn compact_session(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<CompactionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.compact_session(&session_id),
            None => Ok(CompactionResult {
                original_tokens: 0,
                compacted_tokens: 0,
                compression_ratio: 0.0,
                turns_summarized: 0,
                turns_kept: 0,
            }),
        }
    }

    async fn restore_session_messages(
        self,
        _ctx: Context,
        session_id: SessionId,
        envelopes: Vec<String>,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.restore_session_messages(&session_id, envelopes),
            None => Ok(()),
        }
    }

    async fn restore_session_token_state(
        self,
        _ctx: Context,
        session_id: SessionId,
        state: TokenRestoreState,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.restore_session_token_state(&session_id, state),
            None => Ok(()),
        }
    }

    async fn resume_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.resume_session(&session_id),
            None => Ok(()),
        }
    }

    async fn get_work_unit_context(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<WorkUnitContext> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_work_unit_context(&session_id),
            None => None,
        }
    }

    async fn set_work_unit_context(
        self,
        _ctx: Context,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_work_unit_context(&session_id, context),
            None => Ok(()),
        }
    }

    async fn get_pending_input(self, _ctx: Context, session_id: SessionId) -> Option<String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pending_input(&session_id),
            None => None,
        }
    }

    async fn set_pending_input(self, _ctx: Context, session_id: SessionId, text: Option<String>) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_pending_input(&session_id, text);
        }
    }

    async fn set_active_session(self, _ctx: Context, session_id: SessionId) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_active_session(&session_id);
        }
    }

    async fn clear_active_session(self, _ctx: Context) {
        if let Some(handle) = self.inner.session_manager() {
            handle.clear_active_session();
        }
    }

    async fn get_active_session(self, _ctx: Context) -> Option<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_active_session(),
            None => None,
        }
    }

    async fn get_effective_cwd(self, _ctx: Context, session_id: SessionId) -> String {
        match self.inner.session_manager() {
            Some(handle) => handle
                .get_effective_cwd(&session_id)
                .to_string_lossy()
                .into_owned(),
            None => String::new(),
        }
    }

    async fn get_supervisors(self, _ctx: Context, session_id: SessionId) -> Vec<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_supervisors(&session_id),
            None => Vec::new(),
        }
    }

    async fn add_supervisor(
        self,
        _ctx: Context,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.add_supervisor(&subordinate_id, &supervisor_id),
            None => Ok(()),
        }
    }

    async fn remove_supervisor(
        self,
        _ctx: Context,
        supervisor_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.remove_supervisor(&supervisor_id),
            None => Ok(()),
        }
    }

    async fn get_subordinate(self, _ctx: Context, supervisor_id: SessionId) -> Option<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinate(&supervisor_id),
            None => None,
        }
    }

    async fn get_subordinates(self, _ctx: Context, supervisor_id: SessionId) -> Vec<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinates(&supervisor_id),
            None => Vec::new(),
        }
    }

    async fn receive_incoming_message(
        self,
        _ctx: Context,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.receive_incoming_message(&subordinate_id, message),
            None => Ok(()),
        }
    }

    async fn get_debug_enabled(self, _ctx: Context, session_id: SessionId) -> bool {
        match self.inner.session_manager() {
            Some(handle) => handle.get_debug_enabled(&session_id),
            None => false,
        }
    }

    async fn set_debug_enabled(self, _ctx: Context, session_id: SessionId, enabled: bool) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_debug_enabled(&session_id, enabled);
        }
    }

    async fn toggle_debug(
        self,
        _ctx: Context,
        session_id: SessionId,
        debug_dir: String,
    ) -> Result<String, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.toggle_debug(&session_id, &debug_dir),
            None => Ok(String::new()),
        }
    }

    async fn set_debug_directory(self, _ctx: Context, path: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_debug_directory(std::path::PathBuf::from(path)),
            None => Ok(()),
        }
    }

    async fn pause_resume(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_resume(&session_id),
            None => Ok(()),
        }
    }

    async fn pause_confirm(
        self,
        _ctx: Context,
        session_id: SessionId,
        accept: bool,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_confirm(&session_id, accept),
            None => Ok(()),
        }
    }

    async fn pause_triple(
        self,
        _ctx: Context,
        session_id: SessionId,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_triple(&session_id, choice),
            None => Ok(()),
        }
    }

    async fn send_hitl_response(
        self,
        _ctx: Context,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.send_hitl_response(&session_id, response),
            None => Ok(()),
        }
    }

    async fn get_pause_state(self, _ctx: Context, session_id: SessionId) -> Option<PauseState> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pause_state(&session_id),
            None => None,
        }
    }

    async fn get_hitl_request(self, _ctx: Context, session_id: SessionId) -> Option<HitlRequest> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_hitl_request(&session_id),
            None => None,
        }
    }

    async fn get_exec_stdin_request(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<ExecStdinRequest> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_exec_stdin_request(&session_id),
            None => None,
        }
    }

    async fn write_exec_stdin(
        self,
        _ctx: Context,
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.write_exec_stdin(&session_id, &exec_session_id, &text),
            None => Err("write_exec_stdin requires a SessionManagerHandle".to_string()),
        }
    }

    async fn send_fspec_result(
        self,
        _ctx: Context,
        session_id: SessionId,
        result: FspecResult,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.send_fspec_result(&session_id, result),
            None => Ok(()),
        }
    }

    async fn create_isolated_session(
        self,
        _ctx: Context,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.create_isolated_session(role),
            None => Err("create_isolated_session requires a SessionManagerHandle".to_string()),
        }
    }

    async fn set_thinking_level_default(
        self,
        _ctx: Context,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_thinking_level_default(&session_id, level),
            None => Ok(()),
        }
    }

    async fn destroy_session(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.destroy_session(&session_id),
            None => Ok(()),
        }
    }

    // ========================================================================
    // RPC-054: Provider credentials surface forwarders.
    // ========================================================================

    async fn list_provider_credentials(self, _ctx: Context) -> Vec<ProviderCredentialInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_provider_credentials(),
            None => Vec::new(),
        }
    }

    async fn get_provider_credential(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Option<ProviderCredentialInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_provider_credential(&provider_id),
            None => None,
        }
    }

    async fn set_provider_credentials(
        self,
        _ctx: Context,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_provider_credentials(&provider_id, creds),
            None => Ok(()),
        }
    }

    async fn delete_provider_credentials(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.delete_provider_credentials(&provider_id),
            None => Ok(()),
        }
    }

    async fn test_provider_connection(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<TestConnectionResult, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.test_provider_connection(&provider_id),
            None => Ok(TestConnectionResult {
                success: true,
                error: None,
                latency_ms: 0,
            }),
        }
    }

    async fn refresh_models_cache(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<Vec<ModelEntry>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.refresh_models_cache(&provider_id),
            None => Ok(Vec::new()),
        }
    }

    async fn oauth_clear_tokens(self, _ctx: Context, provider_id: String) -> Result<(), String> {
        // PROV-112: providers-direct clear (NOT via session_manager) so the
        // disconnect works regardless of whether a session manager is attached.
        crate::oauth_disconnect::clear_oauth_tokens(&provider_id).await
    }

    async fn oauth_get_tokens(self, _ctx: Context, provider_id: String) -> Result<bool, String> {
        crate::oauth_disconnect::has_oauth_tokens(&provider_id).await
    }

    async fn oauth_browser_login(self, _ctx: Context, provider_id: String) -> Result<(), String> {
        crate::oauth_login::browser_login(&provider_id).await
    }

    async fn oauth_headless_start(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<OAuthHeadlessStart, String> {
        crate::oauth_login::headless_start(&provider_id)
    }

    async fn oauth_headless_complete(
        self,
        _ctx: Context,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<(), String> {
        crate::oauth_login::headless_complete(&provider_id, &code_with_state, &pkce_verifier).await
    }

    async fn oauth_device_start(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<OAuthDeviceStart, String> {
        crate::oauth_login::device_start(&provider_id).await
    }

    async fn oauth_device_poll(
        self,
        _ctx: Context,
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<(), String> {
        crate::oauth_login::device_poll(&provider_id, device_auth_id, interval).await
    }

    async fn oauth_copilot_device_start(
        self,
        _ctx: Context,
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart, String> {
        crate::oauth_copilot::device_start(enterprise_host).await
    }

    async fn blocklist_list(self, _ctx: Context) -> Vec<BlocklistRuleInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.blocklist_list(),
            None => Vec::new(),
        }
    }

    async fn merge_session_worktree(
        self,
        _ctx: Context,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.merge_session_worktree(&session_id, strategy),
            None => Ok(MergeOutcome::default()),
        }
    }

    async fn discard_session_worktree(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.discard_session_worktree(&session_id),
            None => Ok(()),
        }
    }

    async fn prune_orphaned_worktrees(self, _ctx: Context) -> Result<Vec<String>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.prune_orphaned_worktrees(),
            None => Ok(Vec::new()),
        }
    }

    async fn list_session_worktrees(self, _ctx: Context) -> Vec<SessionWorktreeInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_session_worktrees(),
            None => Vec::new(),
        }
    }

    async fn inspect_session_changes(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.inspect_session_changes(&session_id),
            None => Ok(SessionChangesSummary::default()),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(self, _ctx: Context, job: ScheduledJob) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_add(job),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_list(self, _ctx: Context) -> Vec<ScheduledJob> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_list(),
            None => Vec::new(),
        }
    }

    async fn schedule_pause(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_pause(&name),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_resume(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_resume(&name),
            None => Ok(ScheduledJob::default()),
        }
    }

    async fn schedule_remove(self, _ctx: Context, name: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_remove(&name),
            None => Ok(()),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        self,
        _ctx: Context,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_add(&session_id, interval_seconds, prompt),
            None => Ok(RegisteredLoop::default()),
        }
    }

    async fn loop_cancel(self, _ctx: Context, id: String) -> Result<bool, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_cancel(&id),
            None => Ok(false),
        }
    }

    async fn loop_list(self, _ctx: Context, session_id: SessionId) -> Vec<RegisteredLoop> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_list(&session_id),
            None => Vec::new(),
        }
    }
}

/// Test-only seed fixture used by integration tests in this crate and
/// (re-exported) by the embedded transport's tests.
#[cfg(any(test, feature = "test-fixture"))]
pub fn test_fixture() -> Vec<WorkUnitInfo> {
    vec![
        WorkUnitInfo {
            id: "AUTH-001".to_string(),
            title: "User Login".to_string(),
            work_type: "story".to_string(),
            status: "done".to_string(),
            description: Some("Sign in with email/password".to_string()),
            estimate: Some(5),
            epic: Some("authentication".to_string()),
            attachments: Vec::new(),
            last_state_change_at: None,
        },
        WorkUnitInfo {
            id: "AUTH-002".to_string(),
            title: "Password reset".to_string(),
            work_type: "story".to_string(),
            status: "implementing".to_string(),
            description: None,
            estimate: Some(3),
            epic: Some("authentication".to_string()),
            attachments: Vec::new(),
            last_state_change_at: None,
        },
    ]
}
