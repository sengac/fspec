//! MockBackend scripting and recording helpers (part 1).

use super::*;

impl MockBackend {
    /// TUI-109: push a checkpoint-enumeration progress frame so the
    /// checkpoints-progress subscriber test can drive synthetic frames
    /// without a real RPC server.
    pub fn push_checkpoints_progress(&self, progress: codelet_rpc_types::CheckpointsProgress) {
        if let Some(tx) = self.core.checkpoints_progress_tx.lock().as_ref() {
            let _ = tx.send(progress);
        }
    }
    /// BUG-182: push a git-state frame so the git-state subscriber test
    /// can drive synthetic frames without a real watcher (mirrors
    /// `push_checkpoints_progress`).
    pub fn push_git_state_changed(&self, state: GitState) {
        if let Some(tx) = self.core.git_state_changed_tx.lock().as_ref() {
            let _ = tx.send(state);
        }
    }
    /// RPC-415: drop ALL broadcast Senders to simulate the transport
    /// supervisor dropping the old RPC client on a WS disconnect. Every
    /// live subscriber `Receiver` then observes `RecvError::Closed` on its
    /// next `recv().await`, so all App subscriber loops exit.
    pub fn disconnect_all(&self) {
        self.core.focus_state.disconnect();
        *self.core.work_units_tx.lock() = None;
        *self.core.chunks_tx.lock() = None;
        *self.core.logs_tx.lock() = None;
        *self.core.status_changes_tx.lock() = None;
        *self.core.session_created_tx.lock() = None;
        *self.core.session_removed_tx.lock() = None;
        *self.core.session_cleared_tx.lock() = None;
        *self.core.checkpoints_progress_tx.lock() = None;
        *self.core.git_state_changed_tx.lock() = None;
    }
    /// RPC-415: install a FRESH Sender for every broadcast stream,
    /// modelling the transport supervisor swapping in a brand-new RPC
    /// client after a successful reconnect. The new Senders are distinct
    /// from any dropped by `disconnect_all()`, so a subscriber that
    /// re-subscribes now is bound to the NEW client's receivers. Any
    /// receiver still holding an OLD Sender's `Receiver` will never see
    /// events pushed after this call — which is exactly how we prove the
    /// respawn rebinds to the current client.
    pub fn reconnect_all(&self) {
        self.core.focus_state.reconnect();
        let (work_units_tx, _) = broadcast::channel(64);
        let (chunks_tx, _) = broadcast::channel(64);
        let (logs_tx, _) = broadcast::channel(64);
        let (status_changes_tx, _) = broadcast::channel(64);
        let (session_created_tx, _) = broadcast::channel(64);
        let (session_removed_tx, _) = broadcast::channel(64);
        let (session_cleared_tx, _) = broadcast::channel(64);
        let (checkpoints_progress_tx, _) = broadcast::channel(64);
        let (git_state_changed_tx, _) = broadcast::channel(64);
        *self.core.work_units_tx.lock() = Some(work_units_tx);
        *self.core.chunks_tx.lock() = Some(chunks_tx);
        *self.core.logs_tx.lock() = Some(logs_tx);
        *self.core.status_changes_tx.lock() = Some(status_changes_tx);
        *self.core.session_created_tx.lock() = Some(session_created_tx);
        *self.core.session_removed_tx.lock() = Some(session_removed_tx);
        *self.core.session_cleared_tx.lock() = Some(session_cleared_tx);
        *self.core.checkpoints_progress_tx.lock() = Some(checkpoints_progress_tx);
        *self.core.git_state_changed_tx.lock() = Some(git_state_changed_tx);
    }
    /// RPC-045: per-call counter for `send_fspec_result`.
    pub fn send_fspec_result_calls(&self) -> usize {
        self.core.send_fspec_result_calls.load(Ordering::SeqCst)
    }
    /// RPC-045: the most recent `FspecResult` captured by
    /// `send_fspec_result`. `None` until the runner round-trips.
    pub fn last_fspec_result(&self) -> Option<FspecResult> {
        self.core
            .last_fspec_result
            .lock()
            .as_ref()
            .map(|(_, r)| r.clone())
    }
    /// RPC-045: the matching `(SessionId, FspecResult)` tuple captured
    /// by `send_fspec_result`. Useful when a test seeds multiple
    /// sessions and needs to assert which one the result was routed to.
    pub fn last_fspec_result_with_session(&self) -> Option<(SessionId, FspecResult)> {
        self.core.last_fspec_result.lock().clone()
    }
    pub fn list_work_units_calls(&self) -> usize {
        self.core.list_work_units_calls.load(Ordering::SeqCst)
    }
    pub fn create_session_calls(&self) -> usize {
        self.core.create_session_calls.load(Ordering::SeqCst)
    }

    /// TUI-114: the role passed to each `create_session` call, in order.
    pub fn created_session_roles(&self) -> Vec<Option<String>> {
        self.core.created_session_roles.lock().clone()
    }
    pub fn send_input_calls(&self) -> usize {
        self.core.send_input_calls.load(Ordering::SeqCst)
    }
    pub fn interrupt_calls(&self) -> usize {
        self.core.interrupt_calls.load(Ordering::SeqCst)
    }
    /// RIG-016: every `submit_input` call, in order.
    pub fn submitted_inputs(&self) -> Vec<(SessionId, String, codelet_rpc_types::InputMode)> {
        self.orchestration.submitted_inputs.lock().clone()
    }
    pub fn last_send_input(&self) -> Option<(SessionId, String)> {
        self.core.last_send_input.lock().clone()
    }
}
