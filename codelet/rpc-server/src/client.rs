//! Client-side adapter: build a tarpc client over a tokio-tungstenite
//! WebSocket stream.
//!
//! Used by RPC-005/006/007 integration tests; mirrors the connection
//! logic the future remote frontend will use. Returns a fully-spawned
//! [`FspecWsClient`] ready to dispatch RPCs AND broadcast receivers
//! that observe server-pushed `Envelope::WorkUnitsUpdate` (RPC-006),
//! `Envelope::Event` (RPC-007 chunks), and `Envelope::LogEvent`
//! (RPC-007 logs) frames.

use crate::pump::{run_envelope_pump, ClientInbound};
use crate::transport::ChannelTransport;
use codelet_rpc::FspecServiceClient;
use codelet_rpc_types::{LogRecord, SessionId, SessionStatus, StreamChunk, WorkUnitInfo};
use futures::StreamExt;
use tokio::sync::{broadcast, watch};
use tokio_tungstenite::WebSocketStream;

/// Capacity of the per-subscriber broadcast channel handed back from
/// [`FspecWsClient::work_units_rx`]. Matches the watcher capacity in
/// [`codelet_core::work_units::WorkUnitsWatcher`] (RPC-006 architecture
/// note 12).
const WORK_UNITS_BROADCAST_CAPACITY: usize = 64;

/// Capacity of the chunks broadcast channel internal to
/// [`FspecWsClient`] (RPC-007). Matches
/// `DEFAULT_CHUNKS_CAPACITY` in `codelet-rpc` so a slow WS subscriber
/// behaves like a slow embedded subscriber.
const CHUNKS_BROADCAST_CAPACITY: usize = 256;

/// Capacity of the logs broadcast channel internal to [`FspecWsClient`]
/// (RPC-007). Matches `DEFAULT_LOGS_CAPACITY` in `codelet-rpc`.
const LOGS_BROADCAST_CAPACITY: usize = 1024;

/// Capacity of the status-changes broadcast channel internal to
/// [`FspecWsClient`] (RPC-037). Matches the chunks capacity since
/// status updates piggyback on the same per-connection envelope-out
/// channel and are similar in volume.
const STATUS_BROADCAST_CAPACITY: usize = 256;

/// Client returned by [`ws_client_connect`].
///
/// Wraps the tarpc client with sibling broadcast receivers that observe
/// server-pushed `Envelope::WorkUnitsUpdate` frames (RPC-006),
/// `Envelope::Event` frames (RPC-007 chunks), and `Envelope::LogEvent`
/// frames (RPC-007 logs). The same shape as the embedded transport's
/// matching `EmbeddedTransport::*_rx` methods so a transport-agnostic
/// UI can hold either.
///
/// **Disconnect propagation (RPC-010 rule [23]).** The struct holds
/// only the *receiver* sides of the work_units watch and the chunks /
/// logs broadcasts; the corresponding senders live exclusively inside
/// the spawned envelope-pump task via [`ClientInbound`]. When the WS
/// connection drops the pump task ends, the senders are dropped along
/// with it, and any subscriber receiver returned by [`Self::chunks_rx`]
/// / [`Self::logs_rx`] (and any forwarder spawned by
/// [`Self::work_units_rx`]) observes a closed-channel error within a
/// scheduler tick — letting external clients differentiate a hang from
/// a clean shutdown.
pub struct FspecWsClient {
    /// Tarpc client for request/response RPCs. Public for backward
    /// compatibility with RPC-005/006 tests; new RPC-007+ callers
    /// should prefer the [`Self::client`] accessor.
    pub rpc: FspecServiceClient,
    /// Initial watch receiver cloned by [`Self::work_units_rx`]. The
    /// sender side is owned by the pump task only — dropping it on WS
    /// disconnect causes the cloned receiver's `changed().await` to
    /// return `Err`, which the forwarder converts to a broadcast
    /// channel close.
    work_units_watch_rx: watch::Receiver<Option<Vec<WorkUnitInfo>>>,
    /// Template receiver for the chunks broadcast. The sender side is
    /// owned by the pump task only — dropping it on WS disconnect
    /// closes every receiver returned by [`Self::chunks_rx`].
    chunks_rx_template: broadcast::Receiver<(SessionId, StreamChunk)>,
    /// Template receiver for the logs broadcast. The sender side is
    /// owned by the pump task only — dropping it on WS disconnect
    /// closes every receiver returned by [`Self::logs_rx`].
    logs_rx_template: broadcast::Receiver<LogRecord>,
    /// RPC-037: Template receiver for the status-changes broadcast.
    /// The sender side is owned by the pump task only — dropping it on
    /// WS disconnect closes every receiver returned by
    /// [`Self::status_changes_rx`].
    status_rx_template: broadcast::Receiver<(SessionId, SessionStatus)>,
}

impl FspecWsClient {
    /// Borrow the underlying tarpc client. Tests and callers use this
    /// to dispatch RPCs (e.g. `client.list_sessions(context::current())`).
    /// Returns `&FspecServiceClient`; tarpc's generated client is `Clone`
    /// internally for spawned dispatch tasks, so callers can clone if
    /// they need an owned handle.
    pub fn client(&self) -> &FspecServiceClient {
        &self.rpc
    }

    /// Subscribe to server-pushed work-units snapshots.
    ///
    /// Spawns a small forwarder task that:
    /// 1. Emits the currently cached snapshot (if any) once on the new
    ///    broadcast channel — preserving the server's initial-frame
    ///    contract from RPC-006 rule [9] even when the subscriber arrives
    ///    after the pump has already received it.
    /// 2. Forwards every subsequent `watch::changed()` notification onto
    ///    the broadcast channel.
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        let (sub_tx, sub_rx) =
            broadcast::channel::<Vec<WorkUnitInfo>>(WORK_UNITS_BROADCAST_CAPACITY);
        let mut watch_rx = self.work_units_watch_rx.clone();
        tokio::spawn(async move {
            // Replay the current cached snapshot first (if any). Mark it
            // seen so `changed().await` only fires on subsequent updates,
            // avoiding double-delivery.
            {
                let initial = watch_rx.borrow_and_update().clone();
                if let Some(payload) = initial {
                    if sub_tx.send(payload).is_err() {
                        return;
                    }
                }
            }
            while watch_rx.changed().await.is_ok() {
                let next = watch_rx.borrow_and_update().clone();
                if let Some(payload) = next {
                    if sub_tx.send(payload).is_err() {
                        break;
                    }
                }
            }
            // `changed().await` returned `Err`, meaning every watch
            // sender (owned by the pump task) was dropped — i.e. the
            // WS connection closed. Letting `sub_tx` drop here closes
            // the per-subscriber broadcast so external callers of
            // [`Self::work_units_rx`] observe `RecvError::Closed`.
        });
        sub_rx
    }

    /// Subscribe to server-pushed `(SessionId, StreamChunk)` events
    /// (RPC-007). Sibling of [`codelet_rpc_embedded::EmbeddedTransport::chunks_rx`].
    ///
    /// Because the sender is held only by the pump task, a subscriber
    /// observes `RecvError::Closed` as soon as the WS connection drops.
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_rx_template.resubscribe()
    }

    /// Subscribe to server-pushed `LogRecord` events (RPC-007). Sibling
    /// of [`codelet_rpc_embedded::EmbeddedTransport::logs_rx`].
    ///
    /// Because the sender is held only by the pump task, a subscriber
    /// observes `RecvError::Closed` as soon as the WS connection drops.
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_rx_template.resubscribe()
    }

    /// RPC-037: subscribe to server-pushed `(SessionId, SessionStatus)`
    /// status updates. Sibling of
    /// [`codelet_rpc_embedded::EmbeddedTransport::status_changes_rx`].
    ///
    /// Because the sender is held only by the pump task, a subscriber
    /// observes `RecvError::Closed` as soon as the WS connection drops.
    pub fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        self.status_rx_template.resubscribe()
    }
}

/// Build an [`FspecWsClient`] over a tokio-tungstenite WebSocket stream.
pub async fn ws_client_connect<S>(ws: WebSocketStream<S>) -> anyhow::Result<FspecWsClient>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (rpc_in_tx, rpc_in_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (rpc_out_tx, rpc_out_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    // Client never sends auxiliary envelopes; we keep the sender alive
    // so the pump's recv stays Pending forever (instead of returning
    // None and tearing down the loop).
    let (envelope_out_tx, envelope_out_rx) = tokio::sync::mpsc::unbounded_channel();
    // Watch channel: holds the latest server-pushed snapshot. `None`
    // until the first `Envelope::WorkUnitsUpdate` frame is forwarded by
    // the pump. The sender side is moved into the pump task; the
    // initial receiver is cloned by [`FspecWsClient::work_units_rx`].
    let (work_units_watch_tx, work_units_watch_rx) =
        watch::channel::<Option<Vec<WorkUnitInfo>>>(None);
    // RPC-007: broadcast channels for chunks and logs. The senders are
    // moved into the pump task so dropping them on WS disconnect
    // closes every subscriber returned by [`FspecWsClient::chunks_rx`]
    // / [`FspecWsClient::logs_rx`] (RPC-010 rule [23]).
    let (chunks_tx, chunks_rx_template) =
        broadcast::channel::<(SessionId, StreamChunk)>(CHUNKS_BROADCAST_CAPACITY);
    let (logs_tx, logs_rx_template) = broadcast::channel::<LogRecord>(LOGS_BROADCAST_CAPACITY);
    // RPC-037: broadcast channel for push-driven status updates. The
    // sender is moved into the pump task so dropping it on WS
    // disconnect closes every subscriber returned by
    // [`FspecWsClient::status_changes_rx`] (RPC-010 rule [23]).
    let (status_tx, status_rx_template) =
        broadcast::channel::<(SessionId, SessionStatus)>(STATUS_BROADCAST_CAPACITY);

    let (sink, stream) = ws.split();

    tokio::spawn(async move {
        // Hold envelope_out_tx alive for the lifetime of the pump so
        // the channel does not close prematurely.
        let _envelope_out_tx_keepalive = envelope_out_tx;
        if let Err(err) = run_envelope_pump(
            sink,
            stream,
            rpc_in_tx,
            rpc_out_rx,
            envelope_out_rx,
            ClientInbound {
                work_units_tx: work_units_watch_tx,
                chunks_tx,
                logs_tx,
                status_tx,
            },
            None,
            None,
        )
        .await
        {
            tracing::debug!(error = %err, "ws client pump ended");
        }
        // The pump task ending here drops ClientInbound, which drops
        // every sender (watch + two broadcasts). External callers of
        // `FspecWsClient::{work_units_rx, chunks_rx, logs_rx}` observe
        // `RecvError::Closed` on the next `recv().await` — the
        // disconnect propagation rule [23] depends on.
    });

    let transport = ChannelTransport::new(rpc_in_rx, rpc_out_tx);
    let rpc = FspecServiceClient::new(tarpc::client::Config::default(), transport).spawn();
    Ok(FspecWsClient {
        rpc,
        work_units_watch_rx,
        chunks_rx_template,
        logs_rx_template,
        status_rx_template,
    })
}
