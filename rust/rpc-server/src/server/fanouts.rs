//! Per-connection fan-outs for work units, stream chunks, logs and
//! session status (RPC-006/007/011/037).

use crate::envelope::Envelope;
use codelet_rpc_types::{LogRecord, SessionId, SessionStatus, StreamChunk};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

/// Drain the watcher's broadcast receiver and forward each snapshot as
/// `Envelope::WorkUnitsUpdate` onto the per-connection envelope-out
/// channel. Stamps `ServerStats.last_watcher_event_at` on each Ok
/// snapshot and increments `lag_work_units` on RecvError::Lagged.
pub(super) async fn work_units_fanout(
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
pub(super) async fn stream_chunks_fanout(
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
pub(super) async fn log_events_fanout(
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
pub(super) async fn status_changes_fanout(
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
