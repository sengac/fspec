//! Detector polling helpers shared by the BUG-171 detector suites
//! (exec_stdin_detector_clear_bug171.rs and
//! exec_stdin_detector_cooldown_bug171.rs). REFAC-060 moved them here
//! unchanged from exec_stdin_push_chunk_bug171.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_rpc_types::{SessionId, StreamChunk};
use tokio::sync::broadcast;

/// Wait until the detector has stored a request (TEST-042: it fires on its
/// own cadence; the test waits on the stored state, under the deadlock
/// guard, never a fixed time).
pub async fn wait_for_stored_request(
    session: &Arc<codelet_sessions::background_session::BackgroundSession>,
) {
    let stored =
        codelet_test_helpers::wait_until("the detector stores the exec-stdin request", || {
            std::future::ready(matches!(session.get_exec_stdin_request(), Ok(Some(_))))
        })
        .await;
    assert!(stored.is_ok(), "{stored:?}");
}

/// Read the chunk stream until `predicate` holds (TEST-042: the test waits
/// on the chunks themselves; only the deadlock guard ends a stream that
/// never delivers them).
///
/// Returns EVERY chunk observed (in arrival order) — callers must
/// assert on the returned vec, NOT re-drain the receiver afterwards
/// (the chunks are consumed here). A closed stream returns what it saw.
pub async fn wait_for_chunks(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
    predicate: &dyn Fn(&[StreamChunk]) -> bool,
) -> Vec<StreamChunk> {
    let mut seen = Vec::new();
    let read = codelet_test_helpers::deadlock_guard("the awaited exec-stdin chunks", async {
        loop {
            match rx.recv().await {
                Ok((_, chunk)) => {
                    seen.push(chunk);
                    if predicate(&seen) {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    })
    .await;
    assert!(read.is_ok(), "{read:?}; chunks so far: {seen:?}");
    seen
}
