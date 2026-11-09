//! MockBackend scripting and recording helpers (part 1).

use super::*;

impl MockBackend {
    /// BUG-640: the App bootstrap reads the last used model through the
    /// user-directory resolver, so a mock-backed test isolates it first.
    pub fn new() -> Self {
        let isolated = codelet_test_helpers::isolated_user_directory();
        assert!(
            isolated.is_ok(),
            "BUG-640: isolate the user directory: {isolated:?}"
        );
        Self::default()
    }
    /// Seed the in-memory work_units with the supplied list.
    pub fn seed_work_units(&self, units: Vec<WorkUnitInfo>) {
        *self.core.work_units.lock() = Ok(units);
    }
    /// BUG-361: make every later `list_work_units` fail with `message`.
    pub fn fail_list_work_units(&self, message: &str) {
        *self.core.work_units.lock() = Err(message.to_string());
    }
    /// Push a fresh work-units snapshot onto the broadcast channel.
    pub fn push_work_units(&self, units: Vec<WorkUnitInfo>) {
        if let Some(tx) = self.core.work_units_tx.lock().as_ref() {
            let _ = tx.send(units);
        }
    }
    /// TEST-042: how many receivers are bound to the current work_units
    /// sender (a (re)spawned subscriber binds one), so a test pushes once
    /// the subscriber is bound instead of after a delay.
    pub fn work_units_receiver_count(&self) -> usize {
        self.core
            .work_units_tx
            .lock()
            .as_ref()
            .map_or(0, broadcast::Sender::receiver_count)
    }
    /// TEST-042: work_units pushes not yet received by every bound receiver.
    pub fn work_units_unreceived(&self) -> usize {
        self.core
            .work_units_tx
            .lock()
            .as_ref()
            .map_or(0, broadcast::Sender::len)
    }
    /// Script the next `create_session` call to return this SessionId.
    pub fn script_create_session(&self, id: SessionId) {
        *self.core.scripted_session.lock() = Some(Ok(id));
    }
    /// BUG-410: script the next `create_session` call to fail with this error.
    pub fn script_create_session_error(&self, error: &str) {
        *self.core.scripted_session.lock() = Some(Err(error.to_string()));
    }
    /// RPC-427: Return the last project_path passed to `list_sessions()`.
    pub fn list_sessions_project(&self) -> Option<String> {
        self.core.last_list_sessions_project.lock().clone()
    }
    /// Push a chunk onto the chunks broadcast (RPC-009 test helper).
    pub fn push_chunk(&self, id: SessionId, chunk: StreamChunk) {
        if let Some(tx) = self.core.chunks_tx.lock().as_ref() {
            let _ = tx.send((id, chunk));
        }
    }
    /// RPC-045: drop the chunks_tx Sender to simulate a SessionManager
    /// shutdown. Subsequent `chunks_rx().recv()` calls observe
    /// `RecvError::Closed` and the subscriber loop exits cleanly.
    pub fn close_chunks_tx(&self) {
        *self.core.chunks_tx.lock() = None;
    }
    /// RPC-045: push a (SessionId, SessionStatus) broadcast frame so
    /// status-subscriber tests can drive scripted transitions without
    /// touching a real SessionManager.
    pub fn push_status_change(&self, id: SessionId, status: SessionStatus) {
        if let Some(tx) = self.core.status_changes_tx.lock().as_ref() {
            let _ = tx.send((id, status));
        }
    }
    /// RPC-385: push a session-created broadcast frame so the new
    /// session-created subscriber task can be exercised (idempotent tab
    /// append + lag recovery) without a real SessionManager spawn.
    pub fn push_session_created(&self, id: SessionId) {
        let info = SessionInfo {
            id: id.value,
            name: String::new(),
            status: "idle".to_string(),
            project: String::new(),
            message_count: 0,
            provider_id: None,
            model_id: None,
            is_isolated: false,
            worktree_path: None,
            role: None,
            updated_at_ms: None,
            work_unit_focus: Default::default(),
        };
        if let Some(tx) = self.core.session_created_tx.lock().as_ref() {
            let _ = tx.send(info);
        }
    }
    /// RPC-434: push a session-removed broadcast frame so the new
    /// session-removed subscriber task can be exercised (a session
    /// destroyed from ANOTHER client) without a real SessionManager.
    /// Mirrors `push_session_created` — the payload is the destroyed
    /// session's SessionId.
    pub fn push_session_removed(&self, id: SessionId) {
        if let Some(tx) = self.core.session_removed_tx.lock().as_ref() {
            let _ = tx.send(id);
        }
    }
    /// RPC-437: push a session-cleared broadcast frame so the new
    /// session-cleared subscriber task can be exercised (a session
    /// cleared from ANOTHER client) without a real SessionManager.
    /// Mirrors `push_session_removed` — the payload is the cleared
    /// session's SessionId (the session stays LIVE: clear is
    /// repeatable, unlike removal).
    pub fn push_session_cleared(&self, id: SessionId) {
        if let Some(tx) = self.core.session_cleared_tx.lock().as_ref() {
            let _ = tx.send(id);
        }
    }
    /// RPC-434: seed the connect-time open-session snapshot that
    /// `session_snapshot()` will hand to the App bootstrap. The mock's
    /// equivalent of the server-pushed `Envelope::SessionSnapshot` frame
    /// (WS) / `SharedFspecService::live_sessions_snapshot` (embedded).
    pub fn seed_session_snapshot(&self, sessions: Vec<SessionId>) {
        let infos = sessions
            .into_iter()
            .map(|id| SessionInfo {
                id: id.value,
                name: String::new(),
                status: "idle".to_string(),
                project: String::new(),
                message_count: 0,
                provider_id: None,
                model_id: None,
                is_isolated: false,
                worktree_path: None,
                role: None,
                updated_at_ms: None,
                work_unit_focus: Default::default(),
            })
            .collect();
        *self.core.scripted_session_snapshot.lock() = infos;
    }
}
