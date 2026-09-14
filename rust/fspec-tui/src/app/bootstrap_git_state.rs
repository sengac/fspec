//! BUG-182 — bootstrap subscriber (g): `git_state_changed_rx` →
//! `Action::GitStateChanged`.
//!
//! Feature: spec/features/git-state-watcher.feature
//!
//! The shared layer's `GitStateWatcher` (the ONE centralized git
//! polling mechanism — BUG-181's checkpoint-counts push is folded into
//! this stream) re-captures a full `GitState` snapshot on every
//! debounced `.git` change AND every 10-second poll tick, broadcasting
//! ONLY when the snapshot differs (dedup). This subscriber forwards
//! each frame onto the bus; the App dispatch arm folds it onto the
//! EXISTING `CheckpointCountsLoaded` / `WorkspaceInfoLoaded` writer
//! paths (no second writer, DRY) and re-fetches the visible lazy views
//! (R3/R4 — bodies in `app/dispatch_git_state.rs`). Transports that do
//! not forward the push (websocket) return a closed receiver — this
//! loop exits immediately on `RecvError::Closed` and the header falls
//! back to the bootstrap value + `RefreshCheckpointCounts` (documented
//! like TUI-109).
//!
//! Extracted from `bootstrap.rs` to keep that file under the 300-LoC
//! source-shape ceiling.

use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// (g) BUG-182: spawn the `git_state_changed_rx` subscriber task and
    /// push its handle onto `self.subscriber_tasks`.
    pub(crate) fn spawn_git_state_subscriber(&mut self) {
        let tx = self.action_tx.clone();
        let mut rx = self.backend.git_state_changed_rx();
        let git_state_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(state) => {
                        let _ = tx.send(Action::GitStateChanged(state));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("git_state subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(git_state_task);
    }
}
