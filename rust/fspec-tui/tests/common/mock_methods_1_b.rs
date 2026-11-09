//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        let guard = self.core.logs_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    pub fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let guard = self.core.status_changes_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    pub fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        let guard = self.core.session_created_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    /// RPC-434: subscribe to the push-driven session-removed channel
    /// (mirrors `session_created_rx` — the payload is the destroyed
    /// session's `SessionId`).
    pub fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        let guard = self.core.session_removed_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    /// RPC-437: subscribe to the push-driven session-cleared channel
    /// (mirrors `session_removed_rx` — the payload is the cleared
    /// session's `SessionId`; the session stays LIVE).
    pub fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        let guard = self.core.session_cleared_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    /// RPC-434: the scripted connect-time open-session snapshot —
    /// drains the `seed_session_snapshot` prime (the mock equivalent of
    /// the server-pushed `Envelope::SessionSnapshot` frame / the
    /// embedded `live_sessions_snapshot`).
    pub async fn session_snapshot(&self) -> Vec<SessionInfo> {
        let mut guard = self.core.scripted_session_snapshot.lock();
        std::mem::take(&mut guard)
    }
    pub fn checkpoints_progress_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress> {
        let guard = self.core.checkpoints_progress_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    /// BUG-182: subscribe to the push-driven git-state channel
    /// (mock-driven, mirrors `checkpoints_progress_rx`).
    pub fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        let guard = self.core.git_state_changed_tx.lock();
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }
    pub async fn health(&self) -> Result<codelet_rpc_types::HealthInfo> {
        Ok(codelet_rpc_types::HealthInfo {
            uptime_secs: 0,
            connected_clients: 0,
            last_watcher_event_secs_ago: None,
            lag_chunks: 0,
            lag_logs: 0,
            lag_work_units: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    pub async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        self.core
            .checkpoint_counts_calls
            .fetch_add(1, Ordering::SeqCst);
        self.checkpoint_counts_result()
    }
    pub async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        self.core
            .restore_checkpoint_file_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.core.last_restore_file.lock() = Some((work_unit_id, name, path));
        Ok(())
    }
    pub async fn restore_checkpoint_all(&self, _work_unit_id: String, _name: String) -> Result<()> {
        self.core
            .restore_checkpoint_all_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
