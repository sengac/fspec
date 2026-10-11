//! RPC-434 — bootstrap subscriber (h): `session_removed_rx` →
//! `Action::SessionRemoved`.
//!
//! Feature: spec/features/session-events-tui-consumption.feature
//!
//! Push-driven notice for sessions destroyed on the daemon from ANY
//! connected machine (R5): the embedded transport forwards the
//! SessionManager's `session_removed_tx()`; the WebSocket transport
//! splits the RPC-433 `SessionEvent` wire stream onto this per-variant
//! receiver. The App fold (`handle_session_removed`,
//! app/dispatch_session_lifecycle.rs) is idempotent — a double-fire
//! (local close + remote event) is a no-op. Transports that return a
//! closed receiver (degenerate defaults) exit this loop immediately on
//! `RecvError::Closed`; the `Reconnected` handler's abort+respawn of
//! ALL subscriber tasks covers this one automatically (R5).
//!
//! Extracted from `bootstrap.rs` to keep that file under the 300-LoC
//! source-shape ceiling (mirrors bootstrap_git_state.rs).

use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// (h) RPC-434: spawn the `session_removed_rx` subscriber task and
    /// push its handle onto `self.subscriber_tasks`.
    pub(crate) fn spawn_session_removed_subscriber(&mut self) {
        let tx = self.action_tx.clone();
        let mut rx = self.backend.session_removed_rx();
        let session_removed_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(id) => {
                        let _ = tx.send(Action::SessionRemoved(id));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("session_removed subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(session_removed_task);
    }
}
