//! StubSessionManagerHandle inherent helpers (part 2).

use super::*;

impl StubSessionManagerHandle {
    /// BUG-541: give `session` a working directory (an isolated session's
    /// worktree, say) for `get_effective_cwd` to report.
    pub fn set_effective_cwd(&self, session: SessionId, cwd: PathBuf) {
        stub_lock(&self.effective_cwds, "effective_cwds").insert(session, cwd);
    }

    /// RPC-058: seed a single scheduled job as the only row the stub
    /// will return for `schedule_list` / `schedule_add` / etc.
    pub fn seed_scheduled_job(&self, job: ScheduledJob) {
        {
            let mut g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            *g = vec![job];
        }
    }
    /// RPC-058: seed a list of scheduled jobs the stub will return for
    /// `schedule_list`.
    pub fn seed_scheduled_jobs(&self, jobs: Vec<ScheduledJob>) {
        {
            let mut g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            *g = jobs;
        }
    }
    /// RPC-058: how many times `schedule_add` has been called on this
    /// stub.
    pub fn schedule_add_calls(&self) -> u64 {
        self.schedule_add_calls.load(Ordering::SeqCst)
    }
    /// RPC-058: how many times `schedule_list` has been called on this
    /// stub.
    pub fn schedule_list_calls(&self) -> u64 {
        self.schedule_list_calls.load(Ordering::SeqCst)
    }
    /// RPC-058: how many times `schedule_pause` has been called on this
    /// stub.
    pub fn schedule_pause_calls(&self) -> u64 {
        self.schedule_pause_calls.load(Ordering::SeqCst)
    }
    /// RPC-058: how many times `schedule_resume` has been called on this
    /// stub.
    pub fn schedule_resume_calls(&self) -> u64 {
        self.schedule_resume_calls.load(Ordering::SeqCst)
    }
    /// RPC-058: how many times `schedule_remove` has been called on this
    /// stub.
    pub fn schedule_remove_calls(&self) -> u64 {
        self.schedule_remove_calls.load(Ordering::SeqCst)
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop stub state + seeds.
    // ─────────────────────────────────────────────────────────────────

    /// RPC-059: seed a single registered loop as the only row the stub
    /// will return for `loop_list` / `loop_add` / etc.
    pub fn seed_registered_loop(&self, entry: RegisteredLoop) {
        {
            let mut g = stub_lock(&self.registered_loops, "registered_loops");
            *g = vec![entry];
        }
    }
    /// RPC-059: seed a list of registered loops the stub will return
    /// for `loop_list`.
    pub fn seed_registered_loops(&self, entries: Vec<RegisteredLoop>) {
        {
            let mut g = stub_lock(&self.registered_loops, "registered_loops");
            *g = entries;
        }
    }
    /// RPC-059: seed the boolean returned by `loop_cancel`. Defaults to
    /// `true`; tests that need a "not found" outcome pass `false`.
    pub fn seed_loop_cancel_result(&self, result: bool) {
        self.loop_cancel_result.store(result, Ordering::SeqCst);
    }
    /// RPC-059: how many times `loop_add` has been called on this stub.
    pub fn loop_add_calls(&self) -> u64 {
        self.loop_add_calls.load(Ordering::SeqCst)
    }
    /// RPC-059: how many times `loop_cancel` has been called on this stub.
    pub fn loop_cancel_calls(&self) -> u64 {
        self.loop_cancel_calls.load(Ordering::SeqCst)
    }
    /// RPC-059: how many times `loop_list` has been called on this stub.
    pub fn loop_list_calls(&self) -> u64 {
        self.loop_list_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `add_supervisor`.
    pub fn add_supervisor_calls(&self) -> u64 {
        self.add_supervisor_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `remove_supervisor`.
    pub fn remove_supervisor_calls(&self) -> u64 {
        self.remove_supervisor_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `get_supervisors`.
    pub fn get_supervisors_calls(&self) -> u64 {
        self.get_supervisors_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `get_subordinate`.
    pub fn get_subordinate_calls(&self) -> u64 {
        self.get_subordinate_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `get_subordinates`.
    pub fn get_subordinates_calls(&self) -> u64 {
        self.get_subordinates_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: per-call counter for `receive_incoming_message`.
    pub fn receive_incoming_message_calls(&self) -> u64 {
        self.receive_incoming_message_calls.load(Ordering::SeqCst)
    }
    /// RPC-061: snapshot of every `(subordinate_id, IncomingMessageInput)`
    /// payload recorded by `receive_incoming_message`. Cross-transport
    /// parity tests assert both transports land their respective
    /// payloads in this Vec.
    pub fn recorded_incoming_messages(&self) -> Vec<(SessionId, IncomingMessageInput)> {
        stub_lock(
            &self.recorded_incoming_messages,
            "recorded_incoming_messages",
        )
        .clone()
    }
    /// RPC-054: seed a [`ProviderCredentialInfo`] row so a test can
    /// observe `list_provider_credentials` / `get_provider_credential`
    /// returning a non-empty list without going through `set_...`.
    pub fn seed_provider_credential(&self, info: ProviderCredentialInfo) {
        {
            let mut guard = stub_lock(&self.provider_credentials, "provider_credentials");
            guard.insert(info.provider_id.clone(), info);
        }
    }
    /// RPC-037: seed a `SessionTokens` value for a session id.
    pub fn seed_session_tokens(&self, session_id: SessionId, tokens: SessionTokens) {
        {
            let mut guard = stub_lock(&self.tokens, "tokens");
            guard.insert(session_id, tokens);
        }
    }
    /// RPC-037: seed a `SessionModel` value for a session id.
    pub fn seed_session_model(&self, session_id: SessionId, model: SessionModel) {
        {
            let mut guard = stub_lock(&self.models, "models");
            guard.insert(session_id, model);
        }
    }
    /// RPC-037 / AMGR-020: seed a pending approval for a session id.
    pub fn seed_pause_state(&self, session_id: SessionId, state: PendingRequest<PauseState>) {
        {
            let mut guard = stub_lock(&self.pause_state, "pause_state");
            guard.insert(session_id, state);
        }
    }
    /// AMGR-020 / BUG-631: answer the pending pause named `request_id`
    /// when `answers` accepts it (the reply action is for its kind); any
    /// other reply answers nothing.
    pub(super) fn answer_stub_pause(
        &self,
        session_id: &SessionId,
        request_id: &str,
        action: &str,
        answers: impl FnOnce(&PendingRequest<PauseState>) -> bool,
    ) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.pause_state, "pause_state");
            match guard.get(session_id) {
                Some(pending) if pending.route.request_id == request_id => {
                    if !answers(pending) {
                        return Err(format!(
                            "Request {request_id} is a {:?} {:?} pause; {action} does not answer it",
                            pending.route.kind, pending.payload.kind
                        ));
                    }
                    guard.remove(session_id);
                }
                _ => return Err("Stale or already answered interaction".to_string()),
            }
        }
        self.set_status(session_id, SessionStatus::Running);
        let _ = self
            .status_changes_tx
            .send((session_id.clone(), SessionStatus::Running));
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::session_state_change(SessionState::Running),
        ));
        Ok(())
    }
    /// RPC-037 / AMGR-020: seed a pending question for a session id.
    pub fn seed_hitl_request(&self, session_id: SessionId, request: PendingRequest<HitlRequest>) {
        {
            let mut guard = stub_lock(&self.hitl_request, "hitl_request");
            guard.insert(session_id, request);
        }
    }
    /// RPC-022: pre-seed the provider/model registry returned by
    /// `list_providers`. Used by cross-transport parity tests that need
    /// the stub to return a non-empty registry without dragging in the
    /// real `SessionManager` + `ProviderManager` dependency tree.
    pub fn set_providers(&self, providers: Vec<ProviderInfo>) {
        {
            let mut guard = stub_lock(&self.providers, "providers");
            *guard = providers;
        }
    }
    /// Get a clonable handle to the chunks broadcast sender so the host
    /// can dual-fanout chunks (e.g. NAPI ThreadsafeFunction co-listener).
    pub fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }
    /// Get a clonable handle to the logs broadcast sender so the host's
    /// tracing::Layer can push records.
    pub fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }
    /// BUG-520: hold the turns `send_input` starts from now on until the
    /// returned gate is released or dropped, so a test can observe the
    /// session while a turn is running. Without a hold, turns finish on
    /// their own as before.
    pub fn hold_turns(&self) -> super::StubTurnGate {
        let (released, held) = tokio::sync::watch::channel(false);
        *stub_lock(&self.turn_hold, "turn_hold") = Some(held);
        super::StubTurnGate { released }
    }
    pub(super) fn set_status(&self, session_id: &SessionId, status: SessionStatus) {
        {
            let mut sessions = stub_lock(&self.sessions, "sessions");
            for record in sessions.iter_mut() {
                if record.id == *session_id {
                    record.status = status;
                    return;
                }
            }
        }
    }
}
