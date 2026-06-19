//! `App::bootstrap` + subscriber-task spawn helpers (RPC-009 + RPC-012).
//!
//! RPC-012 rule [8]: `bootstrap` no longer pre-creates a session. It
//! ONLY calls `backend.list_work_units()` and seeds the BoardStore via
//! `Action::WorkUnitsLoaded`, then spawns the three subscriber tasks
//! (work_units_rx / chunks_rx / logs_rx). Session creation is deferred
//! until the user enters AgentView for the first time (lazy creation
//! inside `App::dispatch` on `Action::EnterWorkUnit`).

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-012 bootstrap: seed BoardStore via `backend.list_work_units()`
    /// and spawn the three subscriber tasks. Lazy session creation is
    /// deferred to the first `Action::EnterWorkUnit` / `OpenAgentView`.
    ///
    /// RPC-015 extension: additionally fire `backend.checkpoint_counts()`
    /// and dispatch `Action::CheckpointCountsLoaded(counts)` so the
    /// BoardView header paints the live counts on the very first frame.
    pub async fn bootstrap(&mut self) -> Result<()> {
        let units = self.backend.list_work_units().await?;
        self.dispatch(Action::WorkUnitsLoaded(units));
        // RPC-015: best-effort — failure is non-fatal because the header
        // already paints the default `Checkpoints: None` until the call
        // succeeds. We still surface the error to the tracing layer so
        // misconfigured cwds don't go silently undiagnosed.
        match self.backend.checkpoint_counts().await {
            Ok(counts) => {
                self.dispatch(Action::CheckpointCountsLoaded(counts));
            }
            Err(err) => {
                debug!("bootstrap: backend.checkpoint_counts() failed: {err}");
            }
        }
        // RPC-018: best-effort workspace fetch — failure is non-fatal
        // because the SessionFooter just hides the right-side content
        // until the real value arrives via a later refresh.
        match self.backend.get_workspace_info().await {
            Ok(info) => {
                self.dispatch(Action::WorkspaceInfoLoaded(info));
            }
            Err(err) => {
                debug!("bootstrap: backend.get_workspace_info() failed: {err}");
            }
        }
        self.spawn_subscriber_tasks();
        Ok(())
    }

    /// Spawn the three subscriber tasks per RPC-009 architecture note [2].
    /// Each task runs on the host runtime (`tokio::spawn`, NEVER
    /// `tokio::runtime::Builder` or `Runtime::new()` per RPC-005 Q9).
    pub(crate) fn spawn_subscriber_tasks(&mut self) {
        // (a) work_units_rx → Action::WorkUnitsLoaded
        let tx = self.action_tx.clone();
        let backend = self.backend.clone();
        let mut rx = self.backend.work_units_rx();
        let work_units_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(units) => {
                        let _ = tx.send(Action::WorkUnitsLoaded(units));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("work_units subscriber lagged by {n}; resyncing");
                        if let Ok(snapshot) = backend.list_work_units().await {
                            let _ = tx.send(Action::WorkUnitsLoaded(snapshot));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(work_units_task);

        // (b) chunks_rx → Action::ChunkReceived (RPC-045: no active-session
        //     filter — every (SessionId, StreamChunk) is forwarded so
        //     background sessions accumulate scrollback + per-session
        //     state regardless of focus).
        let tx = self.action_tx.clone();
        let mut rx = self.backend.chunks_rx();
        let chunks_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, chunk)) => {
                        let _ = tx.send(Action::ChunkReceived(id, chunk));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("chunks subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(chunks_task);

        // (c) logs_rx → tracing layer.
        let mut rx = self.backend.logs_rx();
        let logs_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(record) => {
                        debug!("rpc log: {record:?}");
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("logs subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(logs_task);

        // (d) RPC-045: status_changes_rx → Action::SessionStatusChanged.
        //     Push-driven replacement for the polling get_session_status
        //     path. Subscribers exit cleanly on RecvError::Closed; lag
        //     is logged via tracing::warn but does NOT crash the loop.
        let tx = self.action_tx.clone();
        let mut rx = self.backend.status_changes_rx();
        let status_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, status)) => {
                        let _ = tx.send(Action::SessionStatusChanged(id, status));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("status_changes subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(status_task);
    }
}
