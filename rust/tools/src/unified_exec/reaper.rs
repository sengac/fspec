//! Background reaper task and session ID generation.

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

/// Spawn a background task that watches for process exit and cleans up.
///
/// Polls every 2 seconds. When the process is detected as exited, the session
/// is removed from the global ProcessStore. Also exits if the session is
/// removed externally (by `close` action or another reaper).
pub fn spawn_reaper(session_id: String) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let store = global_store();

            // Check if session still exists
            if !store.contains(&session_id).await {
                break;
            }

            // Check if process exited
            match store.try_wait(&session_id).await {
                Some(Some(_status)) => {
                    // Process exited — remove from store
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
