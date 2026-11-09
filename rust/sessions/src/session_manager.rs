//! Session manager that owns multiple background sessions.
//!
//! Moved here by **RPC-040** from `rust/napi/src/session_manager.rs`
//! (former lines 2135-3013). All `#[napi]` attributes were stripped and
//! the `napi::Result<T>` return types became `Result<T, String>`. The
//! handful of NAPI-private dependencies that the original methods
//! reached for (agent_loop spawning, the scheduler engine, the footer
//! poller free functions, and the `GLOBAL_CHUNK_CALLBACK` global) are
//! now injected through the new [`SessionManagerHooks`] trait so
//! `codelet-sessions` has zero transitive dependency on `codelet-napi`.
//!
//! ## Hooks design
//!
//! Six things stay napi-side:
//!
//! * `agent_loop` free function (still lives at
//!   `rust/napi/src/session_manager.rs`)
//! * The scheduler engine (`crate::scheduler::spawn_scheduler` and
//!   `crate::scheduler::LoopStore`)
//! * `spawn_footer_poller` / `stop_footer_poller`
//! * `GLOBAL_CHUNK_CALLBACK`
//!
//! Each of these is reachable from `SessionManager` via a method on the
//! [`SessionManagerHooks`] trait object stored as
//! `hooks: ArcSwap<Arc<dyn SessionManagerHooks>>`. A
//! [`NoopSessionManagerHooks`] default impl makes
//! `SessionManager::new()` work out-of-the-box for the future
//! `fspec` binary (RPC-044); the napi side replaces it at startup with
//! a `NapiSessionManagerHooks` that delegates to the existing napi
//! helpers.
//!
//! ## RPC-041 deferral
//!
//! The new broadcast-sender fields (`chunks_tx`, `logs_tx`,
//! `status_changes_tx`) ARE wired up here but
//! `BackgroundSession::handle_output` is NOT yet plumbed to use the
//! manager-owned senders — that is explicitly RPC-041's deliverable.
//! For the duration of RPC-040 the napi side keeps driving
//! `GLOBAL_CHUNK_CALLBACK` via [`SessionManagerHooks::emit_isolation_state_change`].

#![allow(clippy::expect_used)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock, Weak};

use arc_swap::ArcSwap;
use indexmap::IndexMap;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use codelet_rpc_types::{LogRecord, SessionInfo, SessionStatus, StreamChunk};
use codelet_tools::pre_tool_hook::unregister_pre_tool_hook;
use codelet_tools::McpInjection;

use crate::background_session::{BackgroundSession, PromptInput, SUPERVISOR_BROADCAST_CAPACITY};
use crate::session_creation_helper::{
    create_background_session_inner, ParsedModelInfo, SessionCreationParams,
};

/// Maximum concurrent sessions.
pub const MAX_SESSIONS: usize = 10;

/// Session ID type alias — the wire-portable `codelet_rpc_types::SessionId`
/// is used everywhere the SessionManager hands an id across the
/// broadcast or hook boundary so that cross-crate subscribers (napi
/// adapter, future Rust `fspec-tui`) see one consistent type.
pub type SessionId = codelet_rpc_types::SessionId;

/// Hook surface that the napi side installs at startup to keep existing
/// TS behaviour intact. The `fspec` binary in RPC-044 leaves the
/// default [`NoopSessionManagerHooks`] in place.
pub trait SessionManagerHooks: Send + Sync + 'static {
    /// Spawn the agent loop for a newly-created session. The default
    /// no-op impl does nothing; the napi side spawns
    /// `tokio::spawn(async move { agent_loop(session, input_rx, mcp_injection_rx).await })`.
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    );

    /// Start the scheduler for a project if it has not already been
    /// started for this manager.
    fn spawn_scheduler(&self, project: String, rt: tokio::runtime::Handle);

    /// Ensure the scheduler is running for `/loop` support.
    fn ensure_scheduler_running_for_loop(&self, project: String, rt: tokio::runtime::Handle);

    /// Spawn the per-session footer poller.
    fn spawn_footer_poller(&self, session_id: String, cwd: String, worktree_path: Option<String>);

    /// Stop the per-session footer poller.
    fn stop_footer_poller(&self, session_id: &str);

    /// Clean up scheduler loops on session destroy.
    fn cleanup_session_loops(&self, session_id: Uuid);
}

/// Default no-op implementation used by the `fspec` binary in RPC-044
/// where every NAPI subsystem is absent.
#[derive(Default)]
pub struct NoopSessionManagerHooks;

impl SessionManagerHooks for NoopSessionManagerHooks {
    fn spawn_agent_loop(
        &self,
        _session: Arc<BackgroundSession>,
        _input_rx: mpsc::Receiver<PromptInput>,
        _mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
    }

    fn spawn_scheduler(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn ensure_scheduler_running_for_loop(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn spawn_footer_poller(
        &self,
        _session_id: String,
        _cwd: String,
        _worktree_path: Option<String>,
    ) {
    }

    fn stop_footer_poller(&self, _session_id: &str) {}

    fn cleanup_session_loops(&self, _session_id: Uuid) {}
}

/// Background session manager.
///
/// Owns multiple [`BackgroundSession`] instances keyed by `Uuid`.
/// Implements RPC-040's two new responsibilities:
///
/// 1. Three new broadcast senders (`chunks_tx`, `logs_tx`,
///    `status_changes_tx`) that RPC-041 will plug into
///    `BackgroundSession::handle_output`.
/// 2. A pluggable [`SessionManagerHooks`] trait object so the napi
///    side keeps spawning the agent loop, the scheduler, the footer
///    poller, and the IsolationStateChange chunk fan-out unchanged.
pub struct SessionManager {
    pub(crate) sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    /// Tracks subordinate-supervisor relationships between sessions (WATCH-002)
    chain_of_command: crate::chain_of_command::ChainOfCommand,
    /// Tracks the currently active (attached) session for navigation (VIEWNV-001)
    active_session_id: RwLock<Option<Uuid>>,
    /// SCHED-003: Scheduler task handle for graceful shutdown
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// SCHED-004: Default model string for scheduled session spawning
    default_model: RwLock<Option<String>>,
    /// RPC-040: per-manager broadcast of session chunk events.
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    /// RPC-040: per-manager broadcast of log records.
    logs_tx: broadcast::Sender<LogRecord>,
    /// RPC-040: per-manager broadcast of session status changes.
    status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
    /// RPC-385: per-manager broadcast of session-created events. Fires
    /// whenever any session is created (TUI-initiated, scheduled, or a
    /// spawned subordinate via AgentManager) so the embedded Rust TUI can
    /// append a tab for sessions it did not itself initiate. The payload is
    /// the new session's [`SessionInfo`] (its `.id` carries the SessionId).
    session_created_tx: broadcast::Sender<SessionInfo>,
    /// RPC-433: per-manager broadcast of session-removed events. Fires
    /// from `destroy_session` ONLY when the in-memory map actually lost a
    /// session (an idempotent destroy of an unknown id emits nothing).
    /// Payload is the destroyed session's [`SessionId`] — a connected
    /// client already holds all tab state for that session. The WS server
    /// fans it out to every client as
    /// `Envelope::SessionEvent(SessionEvent::Removed(_))` (rule R2 of
    /// session-events-daemon-fanout.feature).
    session_removed_tx: broadcast::Sender<SessionId>,
    /// RPC-436: per-manager broadcast of session-cleared events. Fires
    /// from the clear path (`BackgroundSession::clear_history` — the event
    /// sender is injected at session creation, so the event lands on the
    /// broadcast of the manager that owns the session) ONLY when the
    /// clear actually succeeded (a clear of an unknown id emits nothing,
    /// mirroring the removed-broadcast idempotency). Payload is the
    /// cleared session's [`SessionId`]. The WS server fans it out to
    /// every client as
    /// `Envelope::SessionEvent(SessionEvent::Cleared(_))`. Unlike
    /// removal, a clear is REPEATABLE (a cleared session stays live), so
    /// exactly one event per successful clear.
    session_cleared_tx: broadcast::Sender<SessionId>,
    pub(crate) work_unit_focus_tx: broadcast::Sender<(SessionId, codelet_rpc_types::WorkUnitFocus)>,
    /// RPC-040: NAPI-side subsystems injected via the hooks trait.
    hooks: ArcSwap<Arc<dyn SessionManagerHooks>>,
    /// RPC-386: the manager's own Weak self-reference, stamped on each created
    /// session as its owning-manager back-reference. BUG-420: set by
    /// `Arc::new_cyclic` in [`SessionManager::new`], so it always points at
    /// this manager — there is no way to build a manager without it.
    self_weak: Weak<SessionManager>,
    /// RPC-444: the daemon's start workspace (see `default_project.rs`).
    pub(crate) default_project: OnceLock<PathBuf>,
}

mod manager_impl_1;
mod manager_impl_2;
mod manager_impl_3;
mod manager_impl_4;
mod manager_impl_5;
mod manager_impl_6;
