//! MockBackend scripting and recording helpers (part 4).

use super::*;

impl MockBackend {
    /// RPC-053: seed the value `get_hitl_request(session)` returns.
    /// AMGR-020: wrapped in the route `question-<session>`.
    pub fn script_hitl_request(&self, session: SessionId, value: Option<HitlRequest>) {
        let value = value.map(|payload| PendingRequest {
            route: hitl_route(&session),
            payload,
        });
        self.script_pending_hitl(session, value);
    }
    /// RPC-053: per-call counter for `get_pause_state`.
    pub fn get_pause_state_calls(&self) -> usize {
        self.controls.get_pause_state_calls.load(Ordering::SeqCst)
    }
    /// RPC-053: per-call counter for `get_hitl_request`.
    pub fn get_hitl_request_calls(&self) -> usize {
        self.controls.get_hitl_request_calls.load(Ordering::SeqCst)
    }
    /// AMGR-023: per-call counter for `pause_human_action`.
    pub fn pause_human_action_calls(&self) -> usize {
        self.controls
            .pause_human_action_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-053: per-call counter for `pause_confirm`.
    pub fn pause_confirm_calls(&self) -> usize {
        self.controls.pause_confirm_calls.load(Ordering::SeqCst)
    }
    /// RPC-053: per-call counter for `pause_triple`.
    pub fn pause_triple_calls(&self) -> usize {
        self.controls.pause_triple_calls.load(Ordering::SeqCst)
    }
    /// RPC-053: per-call counter for `send_hitl_response`.
    pub fn send_hitl_response_calls(&self) -> usize {
        self.controls
            .send_hitl_response_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-053: full call log for `pause_confirm` (every (session, accept)).
    pub fn pause_confirm_log(&self) -> Vec<(SessionId, bool)> {
        self.controls.pause_confirm_calls_log.lock().clone()
    }
    /// RPC-053: full call log for `pause_triple`.
    pub fn pause_triple_log(&self) -> Vec<(SessionId, ApprovalChoice)> {
        self.controls.pause_triple_calls_log.lock().clone()
    }
    /// AMGR-023: full call log for `pause_human_action`.
    pub fn pause_human_action_log(&self) -> Vec<(SessionId, codelet_rpc_types::HumanActionReply)> {
        self.controls.pause_human_action_calls_log.lock().clone()
    }
    /// RPC-053: full call log for `send_hitl_response`.
    pub fn send_hitl_response_log(&self) -> Vec<(SessionId, HitlResponse)> {
        self.controls.send_hitl_response_calls_log.lock().clone()
    }
    // ── TOOL-022 P2 helpers ────────────────────────────────────────────────

    /// Seed the value `get_exec_stdin_request(session)` returns.
    /// Pass `Some(request)` to simulate a pending exec-stdin prompt;
    /// pass `None` to simulate "no request" (the default for
    /// un-scripted sessions is also `None`).
    pub fn script_exec_stdin_request(&self, session: SessionId, value: Option<ExecStdinRequest>) {
        self.controls.exec_stdin_store.lock().insert(session, value);
    }
    /// Per-call counter for `get_exec_stdin_request`.
    pub fn get_exec_stdin_request_calls(&self) -> usize {
        self.controls
            .get_exec_stdin_request_calls
            .load(Ordering::SeqCst)
    }
    /// Per-call counter for `write_exec_stdin`.
    pub fn write_exec_stdin_calls(&self) -> usize {
        self.controls.write_exec_stdin_calls.load(Ordering::SeqCst)
    }
    /// Full call log for `write_exec_stdin` (every
    /// (session, exec_session_id, text)).
    pub fn write_exec_stdin_log(&self) -> Vec<(SessionId, String, String)> {
        self.controls.write_exec_stdin_calls_log.lock().clone()
    }
    /// Force the next `get_exec_stdin_request` call to fail.
    pub fn set_get_exec_stdin_request_error(&self, message: String) {
        *self.controls.get_exec_stdin_request_error.lock() = Some(message);
    }
    /// Force the next `write_exec_stdin` call to fail (String error,
    /// matching the trait's `Result<(), String>`).
    pub fn set_write_exec_stdin_error(&self, message: String) {
        *self.controls.write_exec_stdin_error.lock() = Some(message);
    }
    /// RPC-053: force the next `get_pause_state` call to fail.
    pub fn set_get_pause_state_error(&self, message: String) {
        *self.controls.get_pause_state_error.lock() = Some(message);
    }
    /// RPC-053: force the next `get_hitl_request` call to fail.
    pub fn set_get_hitl_request_error(&self, message: String) {
        *self.controls.get_hitl_request_error.lock() = Some(message);
    }
    /// AMGR-023: force the next `pause_human_action` call to fail.
    pub fn set_pause_human_action_error(&self, message: String) {
        *self.controls.pause_human_action_error.lock() = Some(message);
    }
    /// RPC-053: force the next `pause_confirm` call to fail.
    pub fn set_pause_confirm_error(&self, message: String) {
        *self.controls.pause_confirm_error.lock() = Some(message);
    }
    /// RPC-053: force the next `pause_triple` call to fail.
    pub fn set_pause_triple_error(&self, message: String) {
        *self.controls.pause_triple_error.lock() = Some(message);
    }
}
