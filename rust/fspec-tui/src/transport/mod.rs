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
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CheckpointsProgress, CompactionProgress, CompactionResult, CustomModelDefinition, FspecResult,
    GitState, HealthInfo, HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput,
    IsolatedSessionInfo, LogRecord, MergeOutcome, MergeStrategy, ModelEntry, ModelInfo,
    OAuthDeviceStart, OAuthHeadlessStart, PauseState, ProfileDefinition, ProviderCredentialInfo,
    ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob, SessionChangesSummary,
    SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo,
    StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState,
    WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use thiserror::Error;
use tokio::sync::broadcast;

pub mod embedded;
mod focus_events;
mod oauth_context;
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
    #[error(transparent)]
    Transcript(#[from] codelet_rpc_types::TranscriptError),
    #[error("transcript transport failed: {0}")]
    TranscriptTransport(String),
    /// The underlying transport has lost its connection. RPC methods
    /// return this variant rather than panicking or hanging until the
    /// supervisor task reconnects.
    #[error("backend disconnected")]
    Disconnected,
    #[error("provider settings request failed: {0}")]
    ProviderSettings(String),
    #[error("work-unit focus request failed: {0}")]
    WorkUnitFocus(String),
}

include!("backend/core.rs");
include!("backend/workspace.rs");
include!("backend/models.rs");
include!("backend/session_history.rs");
include!("backend/session_context.rs");
include!("backend/interaction.rs");
include!("backend/credentials.rs");
include!("backend/worktrees_jobs.rs");
include!("backend/subscriptions.rs");

macro_rules! compose_backend_finish {
    ($($collected:tt)*) => {
/// Transport-agnostic surface holding both the embedded and WebSocket
/// fspec backends behind a single `Arc<dyn FspecBackend>`.
///
/// Method semantics are identical to the underlying tarpc
/// `FspecService` surface — the trait only exists to enable
/// transport-agnostic consumers in RPC-009 (real list view + REPL) and
/// RPC-010 (binary entry points).
#[async_trait]
pub trait FspecBackend: Send + Sync {
    $($collected)*
}
    };
}

compose_backend_core! {}
