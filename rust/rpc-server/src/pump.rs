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
use futures::{stream::SplitSink, stream::SplitStream, SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

mod inbound;
pub use inbound::{ClientInbound, InboundHandler, ServerInbound};

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
                Envelope::StatusUpdate { session_id, status } => {
                    inbound.on_status_update(session_id, status);
                }
                Envelope::SessionEvent(event) => {
                    inbound.on_session_event(event);
                }
                Envelope::SessionSnapshot(payload) => {
                    inbound.on_session_snapshot(payload);
                }
                Envelope::WorkspaceWorkUnitsUpdate(payload) => {
                    inbound.on_workspace_work_units(payload);
                }
                Envelope::WorkspaceGitStateUpdate(payload) => {
                    inbound.on_workspace_git_state(payload);
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
