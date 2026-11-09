//! MockBackend scripting and recording helpers (part 6).

use super::*;

impl MockBackend {
    #[allow(dead_code)]
    pub fn schedule_add_calls(&self) -> usize {
        self.orchestration.schedule_add_calls.load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn schedule_list_calls(&self) -> usize {
        self.orchestration
            .schedule_list_calls
            .load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn schedule_pause_calls(&self) -> usize {
        self.orchestration
            .schedule_pause_calls
            .load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn schedule_resume_calls(&self) -> usize {
        self.orchestration
            .schedule_resume_calls
            .load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn schedule_remove_calls(&self) -> usize {
        self.orchestration
            .schedule_remove_calls
            .load(Ordering::SeqCst)
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop seeds + counters.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_loop_add_result(
        &self,
        result: std::result::Result<codelet_rpc_types::RegisteredLoop, String>,
    ) {
        *self.orchestration.loop_add_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_loop_cancel_result(&self, result: std::result::Result<bool, String>) {
        *self.orchestration.loop_cancel_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn seed_loop_list_result(
        &self,
        result: std::result::Result<Vec<codelet_rpc_types::RegisteredLoop>, String>,
    ) {
        *self.orchestration.loop_list_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn loop_add_calls(&self) -> usize {
        self.orchestration.loop_add_calls.load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn loop_cancel_calls(&self) -> usize {
        self.orchestration.loop_cancel_calls.load(Ordering::SeqCst)
    }
    #[allow(dead_code)]
    pub fn loop_list_calls(&self) -> usize {
        self.orchestration.loop_list_calls.load(Ordering::SeqCst)
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-060 — create_isolated_session seeds + counter.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_create_isolated_session_result(
        &self,
        result: std::result::Result<IsolatedSessionInfo, String>,
    ) {
        *self.orchestration.create_isolated_session_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn create_isolated_session_calls(&self) -> usize {
        self.orchestration
            .create_isolated_session_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-060: the role passed to each `create_isolated_session` call, in order.
    #[allow(dead_code)]
    pub fn created_isolated_session_roles(&self) -> Vec<Option<String>> {
        self.orchestration
            .created_isolated_session_roles
            .lock()
            .clone()
    }
    // ── RPC-061 seeders / accessors ─────────────────────────────────────

    /// Seed the per-session supervisors list returned by `get_supervisors`.
    pub fn seed_supervisors_for(&self, session: SessionId, supervisors: Vec<SessionId>) {
        self.orchestration
            .supervisors_results
            .lock()
            .insert(session, supervisors);
    }
    /// TUI-124: seed the subordinates `get_subordinates` lists for `supervisor`.
    #[allow(dead_code)]
    pub fn seed_subordinates_for(&self, supervisor: SessionId, subordinates: Vec<SessionId>) {
        self.orchestration
            .subordinates_results
            .lock()
            .insert(supervisor, subordinates);
    }
    /// TUI-124: make every `get_subordinates` call fail with `message`.
    #[allow(dead_code)]
    pub fn fail_get_subordinates(&self, message: &str) {
        *self.orchestration.subordinates_error.lock() = Some(message.to_string());
    }
    /// TUI-124: make `destroy_session(session)` fail with `message`.
    #[allow(dead_code)]
    pub fn fail_destroy_session(&self, session: SessionId, message: &str) {
        self.orchestration
            .destroy_session_errors
            .lock()
            .insert(session, message.to_string());
    }
    /// TUI-124: every session `destroy_session` closed, in order.
    #[allow(dead_code)]
    pub fn destroyed_sessions(&self) -> Vec<SessionId> {
        self.orchestration.destroyed_sessions.lock().clone()
    }
    /// Seed the `Result<(), String>` returned by `add_supervisor`.
    #[allow(dead_code)]
    pub fn seed_add_supervisor_result(&self, result: std::result::Result<(), String>) {
        *self.orchestration.add_supervisor_result.lock() = result;
    }
    /// Seed the `Result<(), String>` returned by `receive_incoming_message`.
    pub fn seed_receive_incoming_message_result(&self, result: std::result::Result<(), String>) {
        *self.orchestration.receive_incoming_message_result.lock() = result;
    }
    #[allow(dead_code)]
    pub fn add_supervisor_calls(&self) -> u64 {
        self.orchestration
            .add_supervisor_calls
            .load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn remove_supervisor_calls(&self) -> u64 {
        self.orchestration
            .remove_supervisor_calls
            .load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_supervisors_calls(&self) -> u64 {
        self.orchestration
            .get_supervisors_calls
            .load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_subordinate_calls(&self) -> u64 {
        self.orchestration
            .get_subordinate_calls
            .load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_subordinates_calls(&self) -> u64 {
        self.orchestration
            .get_subordinates_calls
            .load(Ordering::SeqCst) as u64
    }
    pub fn receive_incoming_message_calls(&self) -> u64 {
        self.orchestration
            .receive_incoming_message_calls
            .load(Ordering::SeqCst) as u64
    }
    /// Borrow the last `(SessionId, IncomingMessageInput)` payload
    /// passed to `receive_incoming_message`.
    pub fn last_received_incoming_message(&self) -> Option<(SessionId, IncomingMessageInput)> {
        self.orchestration
            .last_received_incoming_message
            .lock()
            .clone()
    }
}
