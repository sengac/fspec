//! RPC-445: per-connection fan-out of workspace-keyed work-unit pushes.
//!
//! Feature: spec/features/workspace-work-units-push.feature

use crate::Envelope;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{broadcast, mpsc};

/// RPC-445 / RPC-454: forward a workspace-keyed push (work units or git
/// state) to one connection as `envelope(push)`.
pub(crate) async fn run<T: Clone>(
    mut receiver: broadcast::Receiver<T>,
    outgoing: mpsc::UnboundedSender<Envelope>,
    lag_work_units: Arc<AtomicU64>,
    envelope: fn(T) -> Envelope,
) {
    loop {
        match receiver.recv().await {
            Ok(push) => {
                if outgoing.send(envelope(push)).is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                lag_work_units.fetch_add(skipped, Ordering::SeqCst);
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
