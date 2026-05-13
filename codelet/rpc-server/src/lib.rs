//! codelet-rpc-server: minimal WebSocket transport for the fspec
//! dual-transport tarpc service (RPC-005 + RPC-006, extended by
//! RPC-011 with health stats + graceful drain).
//!
//! ## Wire format (RPC-005 architecture rules 5 + 6, extended in RPC-006)
//!
//! Each WebSocket *binary* frame carries a single bincode-encoded
//! [`Envelope`]. RPC-005 implemented only [`Envelope::Rpc`]; RPC-006
//! lights up [`Envelope::WorkUnitsUpdate`] as the first push variant.
//! The remaining variants are reserved and rejected with a tracing
//! warning. See [`envelope`] for the full type and rationale.
//!
//! The bytes inside [`Envelope::Rpc`] are the bincode-encoded tarpc
//! protocol message — see [`transport::ChannelTransport`] for the
//! Stream+Sink adapter that bridges to tarpc, and [`pump::run_envelope_pump`]
//! for the shared WebSocket pump used by both server and client.
//!
//! ## Shared service impl
//!
//! The server delegates to [`codelet_rpc::FspecServiceImpl`] just like
//! the embedded transport — single source of truth for business logic
//! (RPC-005 architecture rule 4). After RPC-006 the shared service
//! reads from a real `WorkUnitsWatcher` instead of a hard-coded fixture.

mod client;
mod envelope;
mod pump;
mod server;
mod transport;

pub use client::{ws_client_connect, FspecWsClient};
pub use codelet_rpc::register_log_layer;
pub use envelope::Envelope;
pub use server::{bind_and_serve, request_shutdown, ConnectedClientGuard};

use codelet_rpc::{ServerStatsHandle, SharedFspecService};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

/// Observable runtime stats for an in-process rpc-server.
///
/// Returned alongside the bind address so integration tests can directly
/// verify behavioural signals (RPC-005 Scenario 6: reserved variants
/// rejected; Scenario 4: shared impl reached by both transports) without
/// coupling to log output. After RPC-006 the rejected-variants set
/// narrows from {Event, LogEvent, WorkUnitsUpdate, CmdReq, CmdRes} to
/// {Event, LogEvent, CmdReq, CmdRes} — `WorkUnitsUpdate` is now a
/// legitimate first-class variant.
///
/// RPC-011 extensions:
///   * `connected_clients` — live count of attached WebSocket clients
///     (incremented inside `ConnectedClientGuard::new`, decremented via
///     the Drop impl).
///   * `last_watcher_event_at` — Instant of the last watcher snapshot
///     observed by the work-units fanout task. `None` until the first
///     snapshot arrives.
///   * `lag_chunks` / `lag_logs` / `lag_work_units` — cumulative
///     RecvError::Lagged counts surfaced by the three fanout tasks.
///   * `started_at` — process startup instant (matches
///     `SharedFspecService::started_at` so client-visible uptime
///     agrees between RPC-served `health()` and any sideband).
///   * `shutdown_signal` — notified once on SIGTERM/SIGINT so each
///     per-connection task can send a Close{going_away} frame before
///     the daemon aborts the join handle.
#[derive(Clone)]
pub struct ServerStats {
    /// The shared service the server is hosting. Tests can inspect its
    /// invocation counters (e.g. [`SharedFspecService::list_work_units_calls`]).
    pub service: Arc<SharedFspecService>,
    /// Count of WebSocket frames received whose decoded [`Envelope`] was a
    /// reserved variant and was therefore rejected and ignored. Increments
    /// before the tracing warning is emitted so a test that observes the
    /// counter is guaranteed the warning has at least been queued.
    pub rejected_envelopes: Arc<AtomicU64>,
    /// In-order log of the variant *names* the server has rejected.
    pub rejected_variants: Arc<Mutex<Vec<&'static str>>>,
    /// RPC-011: live count of attached WebSocket clients.
    pub connected_clients: Arc<AtomicU64>,
    /// RPC-011: instant of the most recent watcher snapshot observed
    /// by the work-units fanout task.
    pub last_watcher_event_at: Arc<Mutex<Option<Instant>>>,
    /// RPC-011: cumulative RecvError::Lagged count from the chunks
    /// fanout task.
    pub lag_chunks: Arc<AtomicU64>,
    /// RPC-011: cumulative RecvError::Lagged count from the logs
    /// fanout task.
    pub lag_logs: Arc<AtomicU64>,
    /// RPC-011: cumulative RecvError::Lagged count from the
    /// work-units fanout task.
    pub lag_work_units: Arc<AtomicU64>,
    /// RPC-011: process startup instant.
    pub started_at: Instant,
    /// RPC-011: notified once on SIGTERM/SIGINT so each per-connection
    /// task can send a Close{going_away} frame before the daemon
    /// aborts the join handle.
    pub shutdown_signal: Arc<tokio::sync::Notify>,
    /// RPC-011: companion AtomicBool to `shutdown_signal` — set
    /// `request_shutdown` BEFORE `notify_waiters` so a per-connection
    /// task that just entered its `select!` after the notify still
    /// observes the drain state on the next loop iteration. Without
    /// this flag the `Notify::notify_waiters` semantics (no held
    /// permit) makes the wakeup race-prone.
    pub shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl ServerStats {
    pub(crate) fn new(service: Arc<SharedFspecService>) -> Self {
        let started_at = service.started_at();
        Self {
            service,
            rejected_envelopes: Arc::new(AtomicU64::new(0)),
            rejected_variants: Arc::new(Mutex::new(Vec::new())),
            connected_clients: Arc::new(AtomicU64::new(0)),
            last_watcher_event_at: Arc::new(Mutex::new(None)),
            lag_chunks: Arc::new(AtomicU64::new(0)),
            lag_logs: Arc::new(AtomicU64::new(0)),
            lag_work_units: Arc::new(AtomicU64::new(0)),
            started_at,
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
            shutdown_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Read the rejected-envelope counter.
    pub fn rejected_envelopes(&self) -> u64 {
        self.rejected_envelopes.load(Ordering::SeqCst)
    }

    /// Snapshot the in-order list of rejected variant names.
    pub fn rejected_variants(&self) -> Vec<&'static str> {
        self.rejected_variants
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// RPC-011: read the live count of attached clients (incremented
    /// via `ConnectedClientGuard::new`, decremented via Drop).
    pub fn connected_clients(&self) -> u64 {
        self.connected_clients.load(Ordering::SeqCst)
    }

    /// RPC-011: cumulative `RecvError::Lagged` count from the chunks
    /// fanout task.
    pub fn lag_chunks(&self) -> u64 {
        self.lag_chunks.load(Ordering::SeqCst)
    }

    /// RPC-011: cumulative `RecvError::Lagged` count from the logs
    /// fanout task.
    pub fn lag_logs(&self) -> u64 {
        self.lag_logs.load(Ordering::SeqCst)
    }

    /// RPC-011: cumulative `RecvError::Lagged` count from the
    /// work-units fanout task.
    pub fn lag_work_units(&self) -> u64 {
        self.lag_work_units.load(Ordering::SeqCst)
    }

    /// RPC-011: snapshot of the most recent watcher snapshot Instant.
    pub fn last_watcher_event_at(&self) -> Option<Instant> {
        self.last_watcher_event_at
            .lock()
            .ok()
            .and_then(|g| *g)
    }
}

/// RPC-011: `ServerStatsHandle` adapter so the rpc crate can read the
/// counters without taking a dep on rpc-server.
#[derive(Debug)]
struct ServerStatsRead {
    connected_clients: Arc<AtomicU64>,
    last_watcher_event_at: Arc<Mutex<Option<Instant>>>,
    lag_chunks: Arc<AtomicU64>,
    lag_logs: Arc<AtomicU64>,
    lag_work_units: Arc<AtomicU64>,
}

impl ServerStatsHandle for ServerStatsRead {
    fn connected_clients(&self) -> u64 {
        self.connected_clients.load(Ordering::SeqCst)
    }

    fn last_watcher_event_secs_ago(&self) -> Option<u64> {
        let guard = self.last_watcher_event_at.lock().ok()?;
        guard.map(|instant| instant.elapsed().as_secs())
    }

    fn lag_chunks(&self) -> u64 {
        self.lag_chunks.load(Ordering::SeqCst)
    }

    fn lag_logs(&self) -> u64 {
        self.lag_logs.load(Ordering::SeqCst)
    }

    fn lag_work_units(&self) -> u64 {
        self.lag_work_units.load(Ordering::SeqCst)
    }
}

impl ServerStats {
    /// RPC-011: build a `dyn ServerStatsHandle` view of this stats
    /// channel for wiring into `SharedFspecService::set_stats`.
    pub fn handle(&self) -> Arc<dyn ServerStatsHandle> {
        Arc::new(ServerStatsRead {
            connected_clients: Arc::clone(&self.connected_clients),
            last_watcher_event_at: Arc::clone(&self.last_watcher_event_at),
            lag_chunks: Arc::clone(&self.lag_chunks),
            lag_logs: Arc::clone(&self.lag_logs),
            lag_work_units: Arc::clone(&self.lag_work_units),
        })
    }
}
