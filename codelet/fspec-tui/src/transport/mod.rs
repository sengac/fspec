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
    CheckpointCounts, HealthInfo, LogRecord, SessionId, SessionInfo, StreamChunk, WorkUnitInfo,
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
