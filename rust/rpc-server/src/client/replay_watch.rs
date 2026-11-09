//! Watch-backed snapshot streams (work units, RPC-006; open sessions,
//! RPC-433) exposed as per-subscriber broadcasts.
//!
//! Each subscriber gets a forwarder task that:
//! 1. emits the currently cached snapshot (if any) once — preserving the
//!    server's initial-frame contract even when the subscriber arrives
//!    after the pump has already received it;
//! 2. forwards every later `watch::changed()` notification.
//!
//! When every watch sender (owned by the pump task) drops — the WS
//! connection closed — the forwarder exits and drops its broadcast
//! sender, so the subscriber observes `RecvError::Closed`.

use tokio::sync::{broadcast, watch};

pub(super) fn subscribe<T>(
    watch: &watch::Receiver<Option<T>>,
    capacity: usize,
) -> broadcast::Receiver<T>
where
    T: Clone + Send + Sync + 'static,
{
    let (sub_tx, sub_rx) = broadcast::channel::<T>(capacity);
    let mut watch_rx = watch.clone();
    tokio::spawn(async move {
        // Mark the replayed snapshot seen so `changed()` only fires on
        // subsequent updates (no double delivery).
        let initial = watch_rx.borrow_and_update().clone();
        if let Some(payload) = initial {
            if sub_tx.send(payload).is_err() {
                return;
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
    });
    sub_rx
}
