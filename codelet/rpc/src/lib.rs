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
//! `codelet/napi` for the JS frontend, the rpc-server binary for the
//! WebSocket frontend, the embedded host for the ratatui frontend) plus
//! two `tokio::sync::broadcast::Sender` channels for `(SessionId, StreamChunk)`
//! and `LogRecord` events. Both transports observe the SAME senders so
//! NAPI is one listener, not the only listener.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::{
    HealthInfo, LogRecord, SessionId, SessionInfo, SessionStatus, StreamChunk, WorkUnitInfo,
};
use arc_swap::ArcSwap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tarpc::context::Context;
use tokio::sync::broadcast;
use tokio::sync::Mutex as AsyncMutex;

mod log_layer;
pub use log_layer::{register_log_layer, BroadcastLogLayer};

/// The fspec RPC service surface.
///
/// All methods take a `tarpc::context::Context` (injected by the macro) and
/// return owned values that implement `serde::Serialize + Deserialize`.
#[tarpc::service]
pub trait FspecService {
    /// Return every work unit currently known to the shared service impl.
    async fn list_work_units() -> Vec<WorkUnitInfo>;

    /// Return public metadata for every session currently tracked.
    async fn list_sessions() -> Vec<SessionInfo>;

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
}

impl SharedFspecService {
    /// Construct the shared impl with a real workspace watcher (RPC-006).
    /// The session manager handle is left unset — calls to the session
    /// RPCs will return empty defaults until [`with_session_manager`] is
    /// used instead.
    pub fn new(watcher: Arc<WorkUnitsWatcher>) -> Self {
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: None,
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
        }
    }

    /// Construct the shared impl with both a workspace watcher and a
    /// session manager handle (RPC-007). The host (rpc-server bin,
    /// EmbeddedTransport host, or codelet/napi) constructs the concrete
    /// SessionManager and hands it here as `Arc<dyn SessionManagerHandle>`.
    pub fn with_session_manager(
        watcher: Arc<WorkUnitsWatcher>,
        session_manager: Arc<dyn SessionManagerHandle>,
    ) -> Self {
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: Some(session_manager),
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
        }
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

    /// Access the session manager handle, if one was provided.
    pub fn session_manager(&self) -> Option<&Arc<dyn SessionManagerHandle>> {
        self.session_manager.as_ref()
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

    async fn list_sessions(self, _ctx: Context) -> Vec<SessionInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_sessions(),
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
        },
        WorkUnitInfo {
            id: "AUTH-002".to_string(),
            title: "Password reset".to_string(),
            work_type: "story".to_string(),
            status: "implementing".to_string(),
            description: None,
            estimate: Some(3),
            epic: Some("authentication".to_string()),
        },
    ]
}
