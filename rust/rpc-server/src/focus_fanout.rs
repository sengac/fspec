use crate::Envelope;
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{SessionEvent, SessionId, WorkUnitFocus};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{broadcast, mpsc};

pub(crate) async fn run(
    mut receiver: broadcast::Receiver<(SessionId, WorkUnitFocus)>,
    outgoing: mpsc::UnboundedSender<Envelope>,
    service: Arc<SharedFspecService>,
    lag_events: Arc<AtomicU64>,
) {
    loop {
        let envelope = match receiver.recv().await {
            Ok((session_id, focus)) => {
                Envelope::SessionEvent(SessionEvent::FocusChanged { session_id, focus })
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                lag_events.fetch_add(skipped, Ordering::SeqCst);
                Envelope::SessionSnapshot(service.live_sessions_snapshot())
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        if outgoing.send(envelope).is_err() {
            break;
        }
    }
}
