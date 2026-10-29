//! Embedded (in-process) [`FspecBackend`] implementation.
//!
//! Feature: spec/features/fspec-tui-embedded-backend.feature
//! Architecture note 6 (RPC-008): wraps `codelet_rpc_embedded::EmbeddedTransport`
//! and preserves the RPC-005 Q9 host-supplied-Handle invariant at the trait
//! boundary — `new` takes a non-defaulted `tokio::runtime::Handle` plus
//! `Arc<SharedFspecService>`.
//!
//! Construction immediately spawns the in-process tarpc server task on the
//! supplied runtime handle (via `EmbeddedTransport::client()`), so RPC method
//! bodies are one-line delegates to the cached client. Subscription methods
//! are zero-cost passthroughs to the underlying `EmbeddedTransport::*_rx`.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_rpc::{FspecServiceClient, SharedFspecService};
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CompactionProgress, CompactionResult, CustomModelDefinition, ExecStdinRequest, FspecResult,
    GitState, HealthInfo, HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput,
    IsolatedSessionInfo, LogRecord, MergeOutcome, MergeStrategy, ModelEntry, ModelInfo,
    OAuthDeviceStart, OAuthHeadlessStart, PauseState, ProviderCredentialInfo,
    ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob, SessionChangesSummary,
    SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo,
    StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState,
    WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use tarpc::context;
use tokio::sync::broadcast;

use super::FspecBackend;

/// In-process [`FspecBackend`] backed by `codelet_rpc_embedded::EmbeddedTransport`.
///
/// Holds the underlying transport (so subscription receivers stay alive)
/// alongside a cached tarpc client whose worker task was spawned at
/// construction time on the host-supplied runtime handle.
pub struct EmbeddedFspecBackend {
    transport: EmbeddedTransport,
    client: FspecServiceClient,
    /// RPC-037: cached reference to the underlying service so
    /// `status_changes_rx` can subscribe to the session manager's
    /// broadcast channel without going through tarpc.
    service: Arc<SharedFspecService>,
}

impl EmbeddedFspecBackend {
    /// Build an embedded backend bound to the supplied tokio runtime handle
    /// and shared service.
    ///
    /// The `handle` argument is intentionally NON-DEFAULTED so the
    /// RPC-005 Q9 invariant ("EmbeddedTransport requires a tokio Handle
    /// at construction") propagates to this trait boundary. See
    /// `rust/rpc-embedded/tests/architecture_invariants.rs::scenario_7_*`
    /// (widened by RPC-008 to scan `rust/fspec-tui/src/` too).
    pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self {
        let transport = EmbeddedTransport::new(handle, Arc::clone(&service));
        let client = transport.client();
        Self {
            transport,
            client,
            service,
        }
    }
}

include!("embedded/sessions.rs");
include!("embedded/workspace_history.rs");
include!("embedded/models_profiles.rs");
include!("embedded/session_state_transcripts.rs");
include!("embedded/focus_supervision.rs");
include!("embedded/interaction.rs");
include!("embedded/provider_auth.rs");
include!("embedded/worktrees_scheduling.rs");
include!("embedded/subscriptions.rs");

macro_rules! compose_embedded_finish {
    ($($collected:tt)*) => {
        #[async_trait]
        impl FspecBackend for EmbeddedFspecBackend {
            $($collected)*
        }
    };
}

compose_embedded_sessions! {}
