//! RPC-437 — bootstrap subscriber (i): `session_cleared_rx` →
//! `Action::SessionCleared`.
//!
//! Feature: spec/features/session-events-tui-cleared-consumption.feature
//!
//! Push-driven notice for sessions cleared on the daemon from ANY
//! connected machine (R1/R4): the embedded transport forwards the
//! SessionManager's `session_cleared_tx()`; the WebSocket transport
//! splits the RPC-433 `SessionEvent` wire stream onto this
//! per-variant receiver (the RPC-436 `Cleared` frame). Unlike the
//! session-removed subscriber, the cleared session STAYS LIVE (clear
//! is repeatable), so the event is a per-session RESET, never a
//! tab-cleanup — the App fold (`handle_session_cleared`,
//! app/dispatch_session_lifecycle.rs) is idempotent. Transports that
//! return a closed receiver (degenerate defaults) exit this loop
//! immediately on `RecvError::Closed`; the `Reconnected` handler's
//! abort+respawn of ALL subscriber tasks covers this one
//! automatically (R4).
//!
//! Extracted from `bootstrap.rs` to keep that file under the 300-LoC
//! source-shape ceiling (mirrors bootstrap_session_removed.rs).

use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// (i) RPC-437: spawn the `session_cleared_rx` subscriber task and
    /// push its handle onto `self.subscriber_tasks`.
    pub(crate) fn spawn_session_cleared_subscriber(&mut self) {
        let tx = self.action_tx.clone();
        let mut rx = self.backend.session_cleared_rx();
        let session_cleared_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(id) => {
                        let _ = tx.send(Action::SessionCleared(id));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("session_cleared subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(session_cleared_task);
    }
}
