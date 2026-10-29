//! WebSocket [`FspecBackend`] implementation.
//!
//! Feature: spec/features/auto-reconnect-supervisor.feature
//! Architecture rule [21]: a SECOND constructor
//! `connect_with_supervisor(url, action_tx)` spawns a transport-layer
//! reconnect supervisor task that publishes `Action::Disconnected /
//! Reconnecting(n) / Reconnected` onto the App's action bus. The
//! existing `connect(url)` constructor stays as a no-supervisor
//! convenience for tests (RPC-005..RPC-010 signatures unchanged per
//! RPC-011 rule [16]).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use codelet_rpc_server::{ws_client_connect, FspecWsClient};
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CompactionProgress, CompactionResult, CustomModelDefinition, ExecStdinRequest, FspecResult,
    HealthInfo, HistoryMatch, HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo,
    LogRecord, MergeOutcome, MergeStrategy, ModelEntry, ModelInfo, PauseState,
    ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob,
    SessionChangesSummary, SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens,
    SessionWorktreeInfo, StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel,
    TokenRestoreState, WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
};
use tarpc::context;
use tokio::sync::{broadcast, mpsc::UnboundedSender, Notify, RwLock};

use super::{BackendError, FspecBackend};
use crate::components::Action;

/// RPC-011 backoff schedule per rule [3] / scenario "Auto-reconnect backoff schedule":
/// 250ms → 500ms → 1s → 2s → 5s cap.
const BACKOFF_SCHEDULE_MS: &[u64] = &[250, 500, 1_000, 2_000, 5_000];

/// RPC-434: capacity for the per-subscriber created/removed split
/// channels (each carries one session lifecycle frame at a time in
/// practice; 16 mirrors the other per-transport broadcast sizes).
const SESSION_EVENT_SPLITS_CAPACITY: usize = 16;

/// RPC-434: bounded wait for the connect-time `SessionSnapshot` frame
/// before `session_snapshot` degrades to an empty Vec (the pump's
/// watch replay normally delivers the cached snapshot immediately).
const SNAPSHOT_WAIT: Duration = Duration::from_millis(500);

/// WebSocket-backed [`FspecBackend`].
///
/// RPC-011: the inner `FspecWsClient` is now held behind an
/// `Arc<RwLock<Option<FspecWsClient>>>` so the transport-layer
/// supervisor task can atomically swap it out on disconnect /
/// reconnect. When the slot is `None`, RPC trait methods return
/// `Err(BackendError::Disconnected)` so the UI renders the dialog
/// rather than panicking or hanging.
pub struct WebSocketFspecBackend {
    client: Arc<RwLock<Option<FspecWsClient>>>,
    /// RPC-011: manual-reconnect signal. When the user presses 'r'
    /// from the DisconnectDialog the App calls
    /// `FspecBackend::request_manual_reconnect()` which routes here
    /// and notifies the supervisor task to cancel its current backoff
    /// sleep and retry immediately (resetting the schedule).
    manual_reconnect: Arc<Notify>,
}

include!("websocket/construction.rs");
include!("websocket/supervisor_handle.rs");

include!("websocket/provider_sessions.rs");
include!("websocket/subscriptions.rs");
include!("websocket/workspace_checkpoints.rs");
include!("websocket/history_models.rs");
include!("websocket/profiles_control.rs");
include!("websocket/conversation.rs");
include!("websocket/session_context.rs");
include!("websocket/debug_approval.rs");
include!("websocket/isolation_credentials_worktrees.rs");
include!("websocket/scheduling_loops.rs");
include!("websocket/implementation.rs");

compose_websocket_provider_sessions! {}

include!("websocket/reconnect.rs");
