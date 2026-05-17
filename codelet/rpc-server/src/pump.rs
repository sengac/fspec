//! Shared WebSocket → Envelope → tarpc-bytes pump.
//!
//! Both server-side ([`crate::server::handle_connection`]) and client-side
//! ([`crate::client::ws_client_connect`]) need to wrap outgoing tarpc bytes
//! in [`crate::Envelope::Rpc`] before sending them as binary WebSocket frames
//! and unwrap the same envelope on receive. RPC-005 originally inlined this
//! identical loop in two places — the second one growing later for stats
//! and reserved-variant rejection — which is a DRY violation. This module
//! collapses both into a single state machine parameterised by an
//! [`InboundHandler`] hook so the server can count/reject reserved variants
//! while the client only forwards Rpc bodies.
//!
//! ## RPC-006 + RPC-007 push channels
//!
//! Both sides also need a sibling outbound path for arbitrary
//! [`Envelope`] values that are NOT plain `Rpc(bytes)`. The server uses
//! it to push `Envelope::WorkUnitsUpdate(snapshot)` frames produced by
//! the per-connection fan-out tasks; RPC-007 adds `Envelope::Event` and
//! `Envelope::LogEvent` push frames produced by per-connection
//! chunks_fanout / logs_fanout tasks. The client never sends on this
//! channel (it keeps the sender alive so the receiver stays `Pending`).

use crate::envelope::Envelope;
use codelet_rpc_types::{LogRecord, SessionId, StreamChunk, WorkUnitInfo};
use futures::{stream::SplitSink, stream::SplitStream, SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

/// Side-specific behaviour for envelopes that the local side does not
/// know how to act on by itself.
///
/// On the server side, reserved variants (and inbound `WorkUnitsUpdate`
/// — which a client should never push) are counted in
/// [`crate::ServerStats`] and a tracing warning is emitted. RPC-007:
/// inbound `Event` and `LogEvent` from a client are also unexpected
/// (they only flow server → client), so the server-side handler routes
/// them through `on_reserved` exactly the same way. On the client side,
/// `WorkUnitsUpdate(payload)` is demultiplexed onto a
/// `tokio::sync::watch` sender, `Event` payloads are demultiplexed onto
/// the chunks broadcast, and `LogEvent` payloads onto the logs
/// broadcast; everything else is debug-logged.
pub trait InboundHandler: Send + 'static {
    /// Called when a non-`Rpc`, non-`WorkUnitsUpdate`, non-`Event`,
    /// non-`LogEvent` envelope variant arrives — i.e. one of the
    /// reserved-and-rejected variants on the server side, or any
    /// unexpected payload-bearing variant on the client side.
    fn on_reserved(&self, variant: &'static str);

    /// Called when an `Envelope::WorkUnitsUpdate(payload)` arrives.
    ///
    /// Default implementation defers to [`Self::on_reserved`] — that's
    /// the right behaviour on the server side, where a client should
    /// never push a snapshot. Client-side handlers override this to
    /// forward the payload to their broadcast sender.
    fn on_work_units_update(&self, _payload: Vec<WorkUnitInfo>) {
        self.on_reserved("WorkUnitsUpdate");
    }

    /// Called when an `Envelope::Event { session_id, chunk }` arrives
    /// (RPC-007). Default implementation defers to [`Self::on_reserved`]
    /// — chunks only flow server → client, so a client-pushed `Event`
    /// is rejected on the server side. Client-side handlers override
    /// this to forward the payload to their chunks broadcast.
    fn on_event(&self, _session_id: SessionId, _chunk: StreamChunk) {
        self.on_reserved("Event");
    }

    /// Called when an `Envelope::LogEvent(record)` arrives (RPC-007).
    /// Default implementation defers to [`Self::on_reserved`] — logs
    /// only flow server → client, so a client-pushed `LogEvent` is
    /// rejected on the server side. Client-side handlers override this
    /// to forward the payload to their logs broadcast.
    fn on_log_event(&self, _record: LogRecord) {
        self.on_reserved("LogEvent");
    }
}

/// Server-side reserved-variant handler — increments a counter and warns.
pub struct ServerInbound {
    pub stats: crate::ServerStats,
}

impl InboundHandler for ServerInbound {
    fn on_reserved(&self, variant: &'static str) {
        self.stats
            .rejected_envelopes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut log) = self.stats.rejected_variants.lock() {
            log.push(variant);
        }
        tracing::warn!(
            variant = variant,
            "unsupported envelope variant; ignoring frame"
        );
    }
}

/// Client-side handler — stores `WorkUnitsUpdate` payloads in a
/// `tokio::sync::watch` sender and forwards `Event`/`LogEvent` payloads
/// onto their respective `tokio::sync::broadcast` senders so that
/// `FspecWsClient::chunks_rx()` / `logs_rx()` subscribers see them.
///
/// `WorkUnitsUpdate` uses a watch (RPC-006 rule [9] + cross-transport
/// parity scenario): subscribers created AFTER an initial frame has
/// been forwarded by the server still observe the latest snapshot.
/// Broadcast was insufficient there because broadcast receivers only
/// see messages sent *after* they call `subscribe()`, which raced
/// against the server's initial-snapshot push during connection
/// handshake.
///
/// `Event` and `LogEvent` use plain broadcast: chunks/logs are
/// individual events (not snapshot replacements), and the embedded
/// path's matching `chunks_rx`/`logs_rx` already returns a broadcast
/// receiver — keeping the same surface preserves transport-agnostic
/// UI code.
pub struct ClientInbound {
    pub work_units_tx: tokio::sync::watch::Sender<Option<Vec<WorkUnitInfo>>>,
    pub chunks_tx: tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>,
    pub logs_tx: tokio::sync::broadcast::Sender<LogRecord>,
}

impl InboundHandler for ClientInbound {
    fn on_reserved(&self, variant: &'static str) {
        tracing::debug!(
            variant = variant,
            "unexpected non-Rpc envelope from server; ignoring"
        );
    }

    fn on_work_units_update(&self, payload: Vec<WorkUnitInfo>) {
        // `send_replace` always succeeds (does not require subscribers).
        // We deliberately collapse intermediate updates: the watcher's
        // semantics are "latest snapshot wins", which matches the
        // RPC-006 broadcast-payload contract (architecture note 12).
        let _ = self.work_units_tx.send_replace(Some(payload));
    }

    fn on_event(&self, session_id: SessionId, chunk: StreamChunk) {
        // Broadcast to all subscribers. `send` returns `Err` only if
        // there are no active receivers — that's fine, the next
        // subscribe() call will see the next chunk.
        let _ = self.chunks_tx.send((session_id, chunk));
    }

    fn on_log_event(&self, record: LogRecord) {
        let _ = self.logs_tx.send(record);
    }
}

/// Run the WebSocket ⇄ Envelope ⇄ tarpc-bytes pump until either side
/// terminates the stream.
///
/// `rpc_bytes_tx` receives bytes from incoming `Envelope::Rpc` frames so
/// they can be fed into a tarpc transport. `rpc_out_rx` produces bytes
/// emitted by the tarpc transport so they can be wrapped in
/// `Envelope::Rpc` and sent as a binary WS frame. `envelope_out_rx`
/// produces fully-formed [`Envelope`] values from auxiliary tasks (the
/// server's WorkUnitsUpdate / chunks / logs fan-out tasks) — they are
/// bincode-encoded and emitted as binary WS frames alongside the tarpc
/// traffic.
#[allow(clippy::too_many_arguments)]
pub async fn run_envelope_pump<S, H>(
    mut ws_sink: SplitSink<WebSocketStream<S>, Message>,
    mut ws_stream: SplitStream<WebSocketStream<S>>,
    rpc_bytes_tx: UnboundedSender<Vec<u8>>,
    mut rpc_out_rx: UnboundedReceiver<Vec<u8>>,
    mut envelope_out_rx: UnboundedReceiver<Envelope>,
    inbound: H,
    shutdown_signal: Option<std::sync::Arc<tokio::sync::Notify>>,
    shutdown_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    H: InboundHandler,
{
    use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};

    async fn send_going_away<W: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
        ws_sink: &mut SplitSink<WebSocketStream<W>, Message>,
    ) {
        let close_frame = CloseFrame {
            code: CloseCode::Away,
            reason: "going_away".into(),
        };
        let _ = ws_sink.send(Message::Close(Some(close_frame))).await;
    }

    if let Some(signal) = shutdown_signal {
        loop {
            // Check the companion flag BEFORE awaiting — covers the
            // race where `notify_waiters()` fired between iterations.
            if let Some(flag) = shutdown_flag.as_ref() {
                if flag.load(std::sync::atomic::Ordering::SeqCst) {
                    send_going_away(&mut ws_sink).await;
                    break;
                }
            }
            tokio::select! {
                outgoing_rpc = rpc_out_rx.recv() => {
                    let Some(bytes) = outgoing_rpc else { break };
                    let env = Envelope::Rpc(bytes);
                    let encoded = bincode::serialize(&env)?;
                    ws_sink.send(Message::Binary(encoded.into())).await?;
                }
                outgoing_envelope = envelope_out_rx.recv() => {
                    let Some(env) = outgoing_envelope else { break };
                    let encoded = bincode::serialize(&env)?;
                    ws_sink.send(Message::Binary(encoded.into())).await?;
                }
                _ = signal.notified() => {
                    send_going_away(&mut ws_sink).await;
                    break;
                }
                incoming = ws_stream.next() => {
                    let Some(msg) = incoming else { break };
                    let msg = msg?;
                    if pump_dispatch_inbound(msg, &rpc_bytes_tx, &inbound).is_break() {
                        break;
                    }
                }
            }
        }
    } else {
        loop {
            tokio::select! {
                outgoing_rpc = rpc_out_rx.recv() => {
                    let Some(bytes) = outgoing_rpc else { break };
                    let env = Envelope::Rpc(bytes);
                    let encoded = bincode::serialize(&env)?;
                    ws_sink.send(Message::Binary(encoded.into())).await?;
                }
                outgoing_envelope = envelope_out_rx.recv() => {
                    let Some(env) = outgoing_envelope else { break };
                    let encoded = bincode::serialize(&env)?;
                    ws_sink.send(Message::Binary(encoded.into())).await?;
                }
                incoming = ws_stream.next() => {
                    let Some(msg) = incoming else { break };
                    let msg = msg?;
                    if pump_dispatch_inbound(msg, &rpc_bytes_tx, &inbound).is_break() {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Single-frame dispatch shared by both branches of [`run_envelope_pump`].
fn pump_dispatch_inbound<H: InboundHandler>(
    msg: Message,
    rpc_bytes_tx: &UnboundedSender<Vec<u8>>,
    inbound: &H,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow;
    match msg {
        Message::Binary(bytes) => {
            let env: Envelope = match bincode::deserialize(&bytes) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to bincode-decode envelope; closing");
                    return ControlFlow::Break(());
                }
            };
            match env {
                Envelope::Rpc(inner) => {
                    if rpc_bytes_tx.send(inner).is_err() {
                        return ControlFlow::Break(());
                    }
                }
                Envelope::WorkUnitsUpdate(payload) => {
                    inbound.on_work_units_update(payload);
                }
                Envelope::Event { session_id, chunk } => {
                    inbound.on_event(session_id, chunk);
                }
                Envelope::LogEvent(record) => {
                    inbound.on_log_event(record);
                }
                other => {
                    inbound.on_reserved(other.variant_name());
                }
            }
        }
        Message::Close(_) => return ControlFlow::Break(()),
        Message::Ping(_) | Message::Pong(_) => {}
        Message::Text(_) | Message::Frame(_) => {
            tracing::warn!("non-binary WS message; ignoring");
        }
    }
    ControlFlow::Continue(())
}
