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
use codelet_core::git_state::GitStateWatcher;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, ChangedFile, CheckpointCounts, CheckpointInfo,
    CheckpointsProgress, CompactionProgress, CompactionResult, CustomModelDefinition,
    ExecStdinRequest, FspecResult, GitState, HealthInfo, HistoryMatch, HitlRequest, HitlResponse,
    IncomingMessageInput, IsolatedSessionInfo, LogRecord, MergeOutcome, MergeStrategy, ModelEntry,
    ModelInfo, OAuthDeviceStart, OAuthHeadlessStart, PauseState, ProfileDefinition,
    ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo, RegisteredLoop, ScheduledJob,
    SessionChangesSummary, SessionId, SessionInfo, SessionModel, SessionStatus, SessionTokens,
    SessionWorktreeInfo, StreamChunk, TestConnectionResult, ThinkingConfig, ThinkingLevel,
    TokenRestoreState, WorkUnitContext, WorkUnitFocus, WorkUnitInfo, WorkspaceInfo,
};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tarpc::context::Context;
use tokio::sync::broadcast;
use tokio::sync::Mutex as AsyncMutex;

mod changed_files;
#[doc(hidden)]
pub mod checkpoints;
mod log_layer;
pub mod mcp_dispatch;
mod oauth_copilot;
mod oauth_disconnect;
mod oauth_login;
pub mod work_unit_reads;
pub use log_layer::{register_log_layer, BroadcastLogLayer};

use changed_files::collect_changed_files;

include!("service_declaration/workspace.rs");
include!("service_declaration/providers_sessions.rs");
include!("service_declaration/supervision_credentials.rs");
include!("service_declaration/finish.rs");
compose_service_workspace! {}

include!("shared_state.rs");
include!("shared_construction.rs");
include!("shared_broadcasts.rs");

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

include!("service_implementation/interaction_sessions.rs");
include!("service_implementation/health_checkpoints.rs");
include!("service_implementation/workspace_history.rs");
include!("service_implementation/providers.rs");
include!("service_implementation/session_control.rs");
include!("service_implementation/session_restore_focus.rs");
include!("service_implementation/supervision_interaction.rs");
include!("service_implementation/lifecycle_credentials.rs");
include!("service_implementation/oauth_worktrees.rs");
include!("service_implementation/schedule_loops.rs");
include!("service_implementation/finish.rs");
compose_rpc_interaction_sessions! {}

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
