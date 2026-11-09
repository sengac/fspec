//! Session manager handle abstraction (RPC-007).
//!
//! Defines the [`SessionManagerHandle`] trait that the dual-transport RPC
//! layer (rust/rpc) consumes via dependency injection. The concrete
//! 8,649 LOC `SessionManager` implementation lives in `rust/napi/src/
//! session_manager.rs` — rust/core defines only the trait surface so
//! that rust/rpc never imports rust/napi (rpc → napi forbidden;
//! rpc → core permitted; napi → core permitted).
//!
//! ## NAPI shared contract invariant
//!
//! The trait surface here, plus the five new types in rust/rpc-types
//! (SessionId, SessionInfo, SessionStatus, StreamChunk, LogRecord), are
//! the contract that all three frontends consume identically:
//!   * the JS frontend via rust/napi's #[napi] re-exports,
//!   * the built-in ratatui frontend via EmbeddedTransport calling
//!     Arc<dyn SessionManagerHandle> directly,
//!   * the WebSocket frontend via tarpc-generated FspecServiceClient
//!     over bincode-encoded Envelope.
//!
//! ## Test stub
//!
//! [`StubSessionManagerHandle`] is a minimal in-memory implementation
//! used by integration tests so they can exercise the full RPC + push
//! channel surface without dragging in the real SessionManager and its
//! dependency tree (codelet-cli, codelet-git, codelet-tools,
//! codelet-providers, OAuth, persistence, ghost commits, etc.).

use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CompactionProgress, CompactionResult, CustomModelDefinition,
    FspecResult, HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord,
    MergeOutcome, MergeStatus, MergeStrategy, ModelInfo, PauseState, PendingRequest,
    ProfileDefinition, ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo,
    RegisteredLoop, ScheduledJob, SessionChangesSummary, SessionId, SessionInfo, SessionModel,
    SessionState, SessionStatus, SessionTokens, SessionWorktreeInfo, StreamChunk,
    TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState, WorkUnitContext,
    WorkUnitFocus,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::broadcast;

/// RPC-037: broadcast capacity for the per-process status_changes
/// channel. Matches the chunks/logs channel capacities in the stub so
/// a slow subscriber behaves consistently across all three streams.
pub const DEFAULT_STATUS_CHANGES_CAPACITY: usize = 256;

fn status_str(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Idle => "idle",
        SessionStatus::Running => "running",
        SessionStatus::Paused => "paused",
        SessionStatus::Compacting => "compacting",
        SessionStatus::Interrupted => "interrupted",
        SessionStatus::Cleared => "cleared",
    }
}

include!("session_manager_handle/handle_trait_1.rs");
include!("session_manager_handle/handle_trait_2.rs");
include!("session_manager_handle/handle_trait_3.rs");
include!("session_manager_handle/handle_trait_4.rs");

macro_rules! compose_handle_trait_finish {
    ($($collected:tt)*) => {
/// Trait implemented by the concrete `SessionManager` in rust/napi
/// and by [`StubSessionManagerHandle`] in tests.
///
/// All methods are synchronous and non-blocking — the actual session
/// machinery (LLM streams, tool execution, compaction) is owned by
/// the implementation and runs on the host runtime.
pub trait SessionManagerHandle: Send + Sync + 'static {
    $($collected)*
}
    };
}

compose_handle_trait_1! {}

mod stub;
pub use stub::{StubSessionManagerHandle, StubTurnGate};

/// BUG-328: the refusal every handle returns for input to a session it does
/// not hold live.
pub fn session_not_live(session_id: &SessionId) -> String {
    format!(
        "Session not found: {} (resume it with resume_session before sending input)",
        session_id.value
    )
}
