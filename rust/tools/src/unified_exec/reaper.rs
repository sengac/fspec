//! Background reaper task and session ID generation.

use std::time::Duration;

use super::process_store::global_store;
use uuid::Uuid;

/// Generate a short session ID from a UUIDv4.
pub fn generate_session_id() -> String {
    Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap_or("0")
        .to_string()
}

/// TOOL-022 P4: grace window before the reaper removes an exited session.
///
/// An exited process whose session is still being actively polled
/// (touched within this window) is left in the store for one more
/// reaper tick, so the poller observes the exit through the normal
/// `try_wait` path (full final output) rather than racing the removal
/// (which would recover the exit code but lose the trailing bytes).
/// Long enough to cover a full LLM poll window; short enough that an
/// abandoned session is still cleaned up promptly.
const REAPER_GRACE: Duration = Duration::from_secs(5);

/// Spawn a background task that watches for process exit and cleans up.
///
/// Polls every 2 seconds. When the process is detected as exited, the
/// session is removed from the global ProcessStore — UNLESS it is still
/// being actively polled (grace window, TOOL-022 P4), in which case the
/// removal is deferred to the next tick. Also exits if the session is
/// removed externally (by `close` action or another reaper).
pub fn spawn_reaper(session_id: String) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let store = global_store();

            // Check if session still exists
            if !store.contains(&session_id).await {
                break;
            }

            // Check if process exited
            match store.try_wait(&session_id).await {
                Some(Some(status)) => {
                    // Process exited. If a poller is actively touching the
                    // session, defer removal by one tick (grace) so the
                    // poller can observe the exit + final output itself.
                    if store.is_recently_used(&session_id, REAPER_GRACE).await {
                        continue;
                    }
                    // Stash the status (a poller may have just raced the
                    // removal and needs the real exit code, not the
                    // reaper-race -1) and remove from store.
                    store.stash_exit(&session_id, status).await;
                    store.remove(&session_id).await;
                    break;
                }
                Some(None) => {
                    // Still running, keep watching
                }
                None => {
                    // Session gone (removed by user or another reaper)
                    break;
                }
            }
        }
    });
}
