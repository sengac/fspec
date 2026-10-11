//! Server-side WebSocket bind/accept loop and per-connection handler.
//!
//! RPC-011 hardening:
//!   * `ConnectedClientGuard` RAII pairs with `ServerStats.connected_clients`
//!     so the live-client count survives any error path.
//!   * `request_shutdown(stats)` notifies `stats.shutdown_signal`; each
//!     per-connection task observes the notify and sends a WS Close
//!     frame with code 1001 (going_away) on its envelope-out channel
//!     before breaking out of the pump.
//!   * The three fanout tasks now feed lag counters into ServerStats so
//!     `health()` can surface broadcast pressure.
//!   * `work_units_fanout` stamps `ServerStats.last_watcher_event_at`
//!     each time it observes a fresh snapshot.

use crate::envelope::Envelope;
use crate::pump::{run_envelope_pump, ServerInbound};
use crate::transport::ChannelTransport;
use crate::ServerStats;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use codelet_rpc_types::{LogRecord, SessionId, SessionStatus, StreamChunk};
use futures::StreamExt;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tarpc::server::{BaseChannel, Channel};
use tokio::net::{TcpListener, TcpStream};

/// RPC-011: RAII guard pairing each WebSocket connection with the
/// `ServerStats.connected_clients` counter.
///
/// Constructed inside `handle_connection` right after the WS upgrade
/// succeeds (incrementing the counter). The Drop impl decrements the
/// counter so EVERY exit path — clean Close, error, abort — restores
/// the counter without manual try/finally bookkeeping.
pub struct ConnectedClientGuard {
    connected_clients: Arc<std::sync::atomic::AtomicU64>,
}

impl ConnectedClientGuard {
    /// Increment `stats.connected_clients` and return a guard that
    /// decrements on Drop.
    pub fn new(stats: &ServerStats) -> Self {
        stats.connected_clients.fetch_add(1, Ordering::SeqCst);
        Self {
            connected_clients: Arc::clone(&stats.connected_clients),
        }
    }
}

impl Drop for ConnectedClientGuard {
    fn drop(&mut self) {
        self.connected_clients.fetch_sub(1, Ordering::SeqCst);
    }
}

/// RPC-011: trigger a graceful drain on the supplied `ServerStats`.
/// All currently-attached connection tasks observe
/// `shutdown_signal.notified()` on their next poll and send a WS Close
/// frame with code 1001 (going_away) before tearing down the pump.
/// The daemon's shutdown loop calls this BEFORE aborting the
/// `bind_and_serve` `JoinHandle`.
pub fn request_shutdown(stats: &ServerStats) {
    // Set the flag BEFORE notify so a per-connection task that races
    // past the select arm still observes the drain on its next iteration.
    stats
        .shutdown_flag
        .store(true, std::sync::atomic::Ordering::SeqCst);
    stats.shutdown_signal.notify_waiters();
}

/// Bind a TCP listener to `127.0.0.1:0` (or the supplied address) and serve
/// the FspecService over WebSocket against the supplied shared service.
///
/// Returns the [`SocketAddr`] of the bound listener so the caller can read
/// the ephemeral port BEFORE the server task starts accepting connections,
/// a [`ServerStats`] handle for behavioural assertions in tests, and a
/// [`tokio::task::JoinHandle`] callers may abort to shut down.
pub async fn bind_and_serve(
    bind_addr: &str,
    service: Arc<SharedFspecService>,
) -> anyhow::Result<(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local = listener.local_addr()?;
    let stats = ServerStats::new(Arc::clone(&service));
    // RPC-011: wire the stats accessor into the shared service so
    // `health()` can read live counters.
    service.set_stats(stats.handle()).await;
    let stats_for_loop = stats.clone();

    let join = tokio::spawn(async move {
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(error = %e, "accept failed");
                    continue;
                }
            };
            let svc = Arc::clone(&service);
            let stats_for_conn = stats_for_loop.clone();
            // RPC-011 tracing rule: instrument the spawned task with a
            // span carrying client_id so events emitted from inside
            // handle_connection inherit the field — works across tokio
            // worker threads with thread-local subscribers because the
            // span context is captured at spawn time and reattached on
            // each poll via the Instrumented future.
            use tracing::Instrument;
            let span = tracing::info_span!("ws_connection", client_id = %peer);
            tokio::spawn(
                async move {
                    if let Err(e) = handle_connection(stream, peer, svc, stats_for_conn).await {
                        tracing::warn!(peer = %peer, error = %e, "ws connection ended with error");
                    }
                }
                .instrument(span),
            );
        }
    });

    Ok((local, stats, join))
}

#[tracing::instrument(skip_all, fields(client_id = %peer))]
async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    service: Arc<SharedFspecService>,
    stats: ServerStats,
) -> anyhow::Result<()> {
    // Emit an explicit event so the client_id field surfaces in both
    // span-context and event-payload tracing captures.
    tracing::info!(client_id = %peer, "ws connection accepted");

    let ws = tokio_tungstenite::accept_async(stream).await?;
    // RPC-011: RAII counter pairing — decrements via Drop on ANY exit
    // path (clean close, error, abort).
    let _client_guard = ConnectedClientGuard::new(&stats);

    let (rpc_bytes_tx, rpc_bytes_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (server_out_tx, server_out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (envelope_out_tx, envelope_out_rx) = tokio::sync::mpsc::unbounded_channel::<Envelope>();

    // RPC-006: send the initial WorkUnitsUpdate snapshot before any RPC
    // traffic so a fresh client always observes at least the current
    // workspace state without needing to issue an explicit subscribe RPC.
    let initial_snapshot = service.watcher_snapshot();
    if let Ok(mut guard) = stats.last_watcher_event_at.lock() {
        *guard = Some(Instant::now());
    }
    let _ = envelope_out_tx.send(Envelope::WorkUnitsUpdate(initial_snapshot));

    // RPC-433: connect-time open-session snapshot (rule R5). Sent right
    // after the initial WorkUnitsUpdate and ALWAYS — even when empty — so
    // a late-joining client deterministically learns about every session
    // already open (and can distinguish "checked, none open" from
    // "never asked"). In-memory only (no disk merge, no project filter):
    // parity with the BUG-206 liveness gate.
    let live_sessions = service.live_sessions_snapshot();
    tracing::debug!(
        live_sessions = live_sessions.len(),
        client_id = %peer,
        "ws connect-time SessionSnapshot sent"
    );
    let _ = envelope_out_tx.send(Envelope::SessionSnapshot(live_sessions));

    let watcher_rx = service.watcher_rx();
    let envelope_out_tx_for_watcher = envelope_out_tx.clone();
    let lag_work_units = Arc::clone(&stats.lag_work_units);
    let last_watcher_event_at = Arc::clone(&stats.last_watcher_event_at);
    let watcher_fanout = tokio::spawn(work_units_fanout(
        watcher_rx,
        envelope_out_tx_for_watcher,
        lag_work_units,
        last_watcher_event_at,
    ));

    let chunks_rx = service.chunks_rx();
    let envelope_out_tx_for_chunks = envelope_out_tx.clone();
    let lag_chunks = Arc::clone(&stats.lag_chunks);
    let chunks_fanout = tokio::spawn(stream_chunks_fanout(
        chunks_rx,
        envelope_out_tx_for_chunks,
        lag_chunks,
    ));

    let logs_rx = service.logs_rx();
    let envelope_out_tx_for_logs = envelope_out_tx.clone();
    let lag_logs = Arc::clone(&stats.lag_logs);
    let logs_fanout = tokio::spawn(log_events_fanout(
        logs_rx,
        envelope_out_tx_for_logs,
        lag_logs,
    ));

    // RPC-037: per-connection fanout for push-driven session status
    // updates. Drains `SharedFspecService::status_changes_rx` (which
    // delegates to the attached `SessionManagerHandle`) and forwards
    // each `(SessionId, SessionStatus)` tuple as an
    // `Envelope::StatusUpdate` frame onto the per-connection
    // envelope-out channel — mirroring the chunks/logs fanout pattern.
    let status_rx = service.status_changes_rx();
    let envelope_out_tx_for_status = envelope_out_tx.clone();
    let lag_status = Arc::clone(&stats.lag_status);
    let status_fanout = tokio::spawn(status_changes_fanout(
        status_rx,
        envelope_out_tx_for_status,
        lag_status,
    ));

    // RPC-433: per-connection fanout for session lifecycle events
    // (rule R4). Two tasks mirror `status_changes_fanout`: one drains
    // `SharedFspecService::session_created_rx` and wraps each
    // `SessionInfo` in `Envelope::SessionEvent(Created)`, the other
    // drains `session_removed_rx` and wraps each id in
    // `Envelope::SessionEvent(Removed)`. Both fan out to EVERY
    // connected client — including the one that issued the RPC (no
    // per-client filtering, RPC-007 chunk fan-out precedent). They
    // share a single lag counter (`lag_session_events`), like
    // `lag_status` for the status fanout.
    let session_created_rx = service.session_created_rx();
    let envelope_out_tx_for_session_created = envelope_out_tx.clone();
    let lag_session_events = Arc::clone(&stats.lag_session_events);
    let session_created_fanout = tokio::spawn(session_created_fanout(
        session_created_rx,
        envelope_out_tx_for_session_created,
        lag_session_events,
    ));

    let session_removed_rx = service.session_removed_rx();
    let envelope_out_tx_for_session_removed = envelope_out_tx.clone();
    let session_removed_fanout = tokio::spawn(session_removed_fanout(
        session_removed_rx,
        envelope_out_tx_for_session_removed,
        Arc::clone(&stats.lag_session_events),
    ));

    let transport = ChannelTransport::new(rpc_bytes_rx, server_out_tx);
    let service_impl = FspecServiceImpl::new(Arc::clone(&service));
    let server = BaseChannel::with_defaults(transport);
    let serve_fut = server
        .execute(service_impl.serve())
        .for_each(|response| async move {
            tokio::spawn(response);
        });

    let (ws_sink, ws_stream) = ws.split();
    let pump_fut = run_envelope_pump(
        ws_sink,
        ws_stream,
        rpc_bytes_tx,
        server_out_rx,
        envelope_out_rx,
        ServerInbound {
            stats: stats.clone(),
        },
        Some(Arc::clone(&stats.shutdown_signal)),
        Some(Arc::clone(&stats.shutdown_flag)),
    );

    let result = tokio::select! {
        _ = serve_fut => Ok(()),
        result = pump_fut => result,
    };

    watcher_fanout.abort();
    chunks_fanout.abort();
    logs_fanout.abort();
    status_fanout.abort();
    session_created_fanout.abort();
    session_removed_fanout.abort();
    let _ = watcher_fanout.await;
    let _ = chunks_fanout.await;
    let _ = logs_fanout.await;
    let _ = status_fanout.await;
    let _ = session_created_fanout.await;
    let _ = session_removed_fanout.await;
    result
}

/// Drain the watcher's broadcast receiver and forward each snapshot as
/// `Envelope::WorkUnitsUpdate` onto the per-connection envelope-out
/// channel. Stamps `ServerStats.last_watcher_event_at` on each Ok
/// snapshot and increments `lag_work_units` on RecvError::Lagged.
async fn work_units_fanout(
    mut watcher_rx: tokio::sync::broadcast::Receiver<Vec<codelet_rpc_types::WorkUnitInfo>>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_work_units: Arc<std::sync::atomic::AtomicU64>,
    last_watcher_event_at: Arc<std::sync::Mutex<Option<Instant>>>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match watcher_rx.recv().await {
            Ok(snapshot) => {
                if let Ok(mut guard) = last_watcher_event_at.lock() {
                    *guard = Some(Instant::now());
                }
                if envelope_out_tx
                    .send(Envelope::WorkUnitsUpdate(snapshot))
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_work_units.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws work-units fan-out lagged; resyncing");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// RPC-007 + RPC-011: drain chunks and forward as `Envelope::Event`.
async fn stream_chunks_fanout(
    mut chunks_rx: tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_chunks: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match chunks_rx.recv().await {
            Ok((session_id, chunk)) => {
                if envelope_out_tx
                    .send(Envelope::Event { session_id, chunk })
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_chunks.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws chunks fan-out lagged; some chunks dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// RPC-007 + RPC-011: drain logs and forward as `Envelope::LogEvent`.
async fn log_events_fanout(
    mut logs_rx: tokio::sync::broadcast::Receiver<LogRecord>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_logs: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match logs_rx.recv().await {
            Ok(record) => {
                if envelope_out_tx.send(Envelope::LogEvent(record)).is_err() {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_logs.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws logs fan-out lagged; some log records dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// RPC-037: drain push-driven status updates and forward as
/// `Envelope::StatusUpdate`. Mirrors `stream_chunks_fanout` /
/// `log_events_fanout` — see [`crate::pump::ClientInbound::on_status_update`]
/// for the client-side decoder.
async fn status_changes_fanout(
    mut status_rx: tokio::sync::broadcast::Receiver<(SessionId, SessionStatus)>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_status: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match status_rx.recv().await {
            Ok((session_id, status)) => {
                if envelope_out_tx
                    .send(Envelope::StatusUpdate { session_id, status })
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_status.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws status fan-out lagged; some status updates dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// RPC-433: drain session-created events and forward as
/// `Envelope::SessionEvent(Created(info))` to every connected client
/// (rule R4). Mirrors `status_changes_fanout`; a lagged subscriber
/// recovers via RecvError::Lagged — the client's tab append is
/// idempotent and a missed add is re-asserted by the next
/// connect-time `SessionSnapshot` on reconnect.
async fn session_created_fanout(
    mut created_rx: tokio::sync::broadcast::Receiver<codelet_rpc_types::SessionInfo>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_session_events: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match created_rx.recv().await {
            Ok(info) => {
                if envelope_out_tx
                    .send(Envelope::SessionEvent(
                        codelet_rpc_types::SessionEvent::Created(info),
                    ))
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_session_events.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws session-created fan-out lagged; some events dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}

/// RPC-433: drain session-removed events and forward as
/// `Envelope::SessionEvent(Removed(id))` to every connected client
/// (rule R4). Mirrors `session_created_fanout`; a missed removal is
/// healed by the next connect-time `SessionSnapshot` on reconnect
/// (known limitation: add-only replay, no stale-tab pruning).
async fn session_removed_fanout(
    mut removed_rx: tokio::sync::broadcast::Receiver<SessionId>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_session_events: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match removed_rx.recv().await {
            Ok(session_id) => {
                if envelope_out_tx
                    .send(Envelope::SessionEvent(
                        codelet_rpc_types::SessionEvent::Removed(session_id),
                    ))
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_session_events.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws session-removed fan-out lagged; some events dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}
