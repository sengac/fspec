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
    pub async fn bootstrap(&mut self) -> Result<()> {
        let units = self.backend.list_work_units().await?;
        self.dispatch(Action::WorkUnitsLoaded(units));
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

        // (b) chunks_rx → Action::ChunkReceived (filtered by active session)
        let tx = self.action_tx.clone();
        let mut rx = self.backend.chunks_rx();
        let active_rx = self.active_session_rx.clone();
        let chunks_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, chunk)) => {
                        // RPC-009 rule [8]: filter by the AgentViewStore's
                        // current_session BEFORE emitting.
                        let active = active_rx.borrow().clone();
                        if active.as_ref() == Some(&id) {
                            let _ = tx.send(Action::ChunkReceived(id, chunk));
                        }
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
    }
}
