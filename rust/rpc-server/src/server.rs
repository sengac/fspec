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

mod connection_fanouts;
mod fanouts;
mod session_fanouts;

use crate::envelope::Envelope;
use crate::pump::{run_envelope_pump, ServerInbound};
use crate::transport::ChannelTransport;
use crate::ServerStats;
use codelet_rpc::{FspecService, FspecServiceImpl, SharedFspecService};
use futures::StreamExt;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tarpc::server::{BaseChannel, Channel};
use tokio::net::TcpListener;

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
    serve(bind_addr, service, None).await
}

/// RPC-456: [`bind_and_serve`] over TLS — every accepted connection
/// completes the TLS handshake with `acceptor` before the WebSocket
/// handshake (and its RPC-447 bearer check).
pub async fn bind_and_serve_tls(
    bind_addr: &str,
    service: Arc<SharedFspecService>,
    acceptor: tokio_rustls::TlsAcceptor,
) -> anyhow::Result<(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)> {
    serve(bind_addr, service, Some(acceptor)).await
}

async fn serve(
    bind_addr: &str,
    service: Arc<SharedFspecService>,
    tls: Option<tokio_rustls::TlsAcceptor>,
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
            let tls = tls.clone();
            // RPC-011 tracing rule: instrument the spawned task with a
            // span carrying client_id so events emitted from inside
            // handle_connection inherit the field.
            use tracing::Instrument;
            let span = tracing::info_span!("ws_connection", client_id = %peer);
            tokio::spawn(
                async move {
                    let outcome = match tls {
                        None => handle_connection(stream, peer, svc, stats_for_conn).await,
                        // RPC-456: a failed TLS handshake ends this connection.
                        Some(acceptor) => match acceptor.accept(stream).await {
                            Ok(stream) => {
                                handle_connection(stream, peer, svc, stats_for_conn).await
                            }
                            Err(error) => Err(anyhow::anyhow!("TLS handshake failed: {error}")),
                        },
                    };
                    if let Err(e) = outcome {
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
async fn handle_connection<S>(
    stream: S,
    peer: SocketAddr,
    service: Arc<SharedFspecService>,
    stats: ServerStats,
) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Emit an explicit event so the client_id field surfaces in both
    // span-context and event-payload tracing captures.
    tracing::info!(client_id = %peer, "ws connection accepted");

    let mut bounded_reads = false;
    #[allow(
        clippy::result_large_err,
        reason = "tungstenite requires an unboxed HTTP ErrorResponse in its handshake callback"
    )]
    let ws = tokio_tungstenite::accept_hdr_async(
        stream,
        |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
         mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
            // RPC-447: a token-armed daemon only serves the matching bearer.
            let authorization = request
                .headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok());
            if !service.authorizes(authorization) {
                let mut rejection =
                    tokio_tungstenite::tungstenite::handshake::server::ErrorResponse::new(Some(
                        "missing or invalid bearer token".to_string(),
                    ));
                *rejection.status_mut() =
                    tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED;
                return Err(rejection);
            }
            bounded_reads = request.uri().path() == "/fspec-read-v2";
            if bounded_reads {
                response.headers_mut().insert(
                    "x-fspec-rpc-schema",
                    tokio_tungstenite::tungstenite::http::HeaderValue::from_static(
                        codelet_rpc::mcp_dispatch::INTERFACE,
                    ),
                );
                response.headers_mut().insert(
                    "x-fspec-read-version",
                    tokio_tungstenite::tungstenite::http::HeaderValue::from_static("2"),
                );
            }
            Ok(response)
        },
    )
    .await?;
    // RPC-011: RAII counter pairing — decrements via Drop on ANY exit
    // path (clean close, error, abort).
    let _client_guard = ConnectedClientGuard::new(&stats);
    if bounded_reads {
        return crate::bounded_reads::run(ws, service, stats).await;
    }

    let (rpc_bytes_tx, rpc_bytes_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (server_out_tx, server_out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (envelope_out_tx, envelope_out_rx) = tokio::sync::mpsc::unbounded_channel::<Envelope>();

    // RPC-006: send the initial WorkUnitsUpdate snapshot before any RPC
    // traffic so a fresh client always observes at least the current
    // workspace state without needing to issue an explicit subscribe RPC.
    let focus_rx = service.work_unit_focus_rx();
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

    let fanouts =
        connection_fanouts::ConnectionFanouts::spawn(&service, &envelope_out_tx, &stats, focus_rx);

    let transport = ChannelTransport::new(rpc_bytes_rx, server_out_tx);
    // MCP-003: this connection passed the RPC-447 handshake check above.
    let service_impl = FspecServiceImpl::for_authenticated_connection(Arc::clone(&service));
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

    fanouts.shutdown().await;
    result
}
