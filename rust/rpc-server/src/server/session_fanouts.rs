//! Per-connection fan-outs for session lifecycle events
//! (RPC-433 created/removed, RPC-436 cleared).

use crate::envelope::Envelope;
use codelet_rpc_types::SessionId;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// RPC-433: drain session-created events and forward as
/// `Envelope::SessionEvent(Created(info))` to every connected client
/// (rule R4). Mirrors `status_changes_fanout`; a lagged subscriber
/// recovers via RecvError::Lagged — the client's tab append is
/// idempotent and a missed add is re-asserted by the next
/// connect-time `SessionSnapshot` on reconnect.
pub(super) async fn session_created_fanout(
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
pub(super) async fn session_removed_fanout(
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

/// RPC-436: drain session-cleared events and forward as
/// `Envelope::SessionEvent(Cleared(id))` to every connected client
/// (rule R2 of session-events-cleared-fan-out.feature). Mirrors
/// `session_removed_fanout`; a missed clear is healed by the client's
/// reactive reset already riding the `SessionStateChange { Cleared }`
/// chunks stream (the wire event is the cross-client lifecycle signal,
/// not the local UI-reset source of truth).
pub(super) async fn session_cleared_fanout(
    mut cleared_rx: tokio::sync::broadcast::Receiver<SessionId>,
    envelope_out_tx: tokio::sync::mpsc::UnboundedSender<Envelope>,
    lag_session_events: Arc<std::sync::atomic::AtomicU64>,
) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match cleared_rx.recv().await {
            Ok(session_id) => {
                if envelope_out_tx
                    .send(Envelope::SessionEvent(
                        codelet_rpc_types::SessionEvent::Cleared(session_id),
                    ))
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                lag_session_events.fetch_add(skipped, Ordering::SeqCst);
                tracing::warn!(target: "codelet_rpc_server::server", skipped, "ws session-cleared fan-out lagged; some events dropped");
                continue;
            }
            Err(RecvError::Closed) => break,
        }
    }
}
