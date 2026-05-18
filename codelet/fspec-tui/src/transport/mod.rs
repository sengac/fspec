//! Transport-agnostic backend surface for the fspec ratatui frontend.
//!
//! Feature: spec/features/fspec-tui-trait-surface.feature
//!
//! The [`FspecBackend`] trait is intentionally a near-1:1 of the tarpc
//! `FspecService` surface. It exists ONLY to let RPC-009/RPC-010 consumers
//! hold an `Arc<dyn FspecBackend>` and swap between the embedded and
//! WebSocket implementations without changing call sites — not to add
//! abstraction or transformation.
//!
//! Both implementations forward RPC method bodies as one-line delegates to
//! the underlying tarpc client (`self.client.<rpc>(context::current(),
//! ...).await`) and forward subscription methods as zero-cost passthroughs
//! of `broadcast::Receiver` returned by the inner transport. Envelope
//! framing for the WebSocket path stays entirely encapsulated in
//! `codelet-rpc-server`.

use anyhow::Result;
use async_trait::async_trait;
use codelet_rpc_types::{
    CheckpointCounts, HealthInfo, HistoryMatch, LogRecord, ModelInfo, SessionId, SessionInfo,
    StreamChunk, ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use thiserror::Error;
use tokio::sync::broadcast;

pub mod embedded;
pub mod websocket;

pub use embedded::EmbeddedFspecBackend;
pub use websocket::WebSocketFspecBackend;

/// RPC-011: structured error variants returned by `FspecBackend` impls.
///
/// `Disconnected` is the sentinel that the WebSocket transport's RPC
/// methods return once their internal client slot is `None` — i.e.
/// after the supervisor task has observed a WS drop and is currently
/// retrying. The App run loop renders the DisconnectDialog @
/// Priority::Critical in response, so user-visible behaviour is
/// always "a dialog, never a panic or hang".
#[derive(Debug, Error)]
pub enum BackendError {
    /// The underlying transport has lost its connection. RPC methods
    /// return this variant rather than panicking or hanging until the
    /// supervisor task reconnects.
    #[error("backend disconnected")]
    Disconnected,
}

/// Transport-agnostic surface holding both the embedded and WebSocket
/// fspec backends behind a single `Arc<dyn FspecBackend>`.
///
/// Method semantics are identical to the underlying tarpc
/// `FspecService` surface — the trait only exists to enable
/// transport-agnostic consumers in RPC-009 (real list view + REPL) and
/// RPC-010 (binary entry points).
#[async_trait]
pub trait FspecBackend: Send + Sync {
    /// List all known work units. Mirrors `FspecService::list_work_units`.
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>;

    /// List all known sessions. Mirrors `FspecService::list_sessions`.
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;

    /// Create a new session with an optional role overlay.
    async fn create_session(&self, role: Option<String>) -> Result<SessionId>;

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

    /// RPC-011 rule [4]: trigger the transport's manual-reconnect signal
    /// (resets the backoff schedule + cancels any in-flight backoff
    /// sleep). Wired to the App's `r`-press handler from the
    /// DisconnectDialog so pressing `r` while disconnected immediately
    /// attempts reconnect rather than waiting for the next backoff tick.
    ///
    /// Default impl is a no-op so embedded and other transports without
    /// a reconnect supervisor (where the call has no meaning) don't need
    /// to override.
    fn request_manual_reconnect(&self) {}
}
