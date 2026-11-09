//! MockBackend scripting and recording helpers (part 3).

use super::*;

impl MockBackend {
    /// TUI-113: script the active session returned by `get_active_session`.
    /// `None` (the default) models a daemon with no in-memory pointer.
    pub fn seed_active_session(&self, session: SessionId) {
        *self.core.scripted_active_session.lock() = Some(session);
    }
    /// BUG-206: script the daemon's in-memory session IDs returned by
    /// `list_live_sessions` (the auto-restore liveness gate). Default
    /// is an empty list (no live sessions — nothing restorable).
    pub fn seed_live_sessions(&self, sessions: Vec<SessionId>) {
        *self.core.scripted_live_sessions.lock() = sessions;
    }
    /// BUG-206: how many times `list_live_sessions` was awaited (the
    /// auto-restore liveness gate must be consulted exactly on the
    /// restore path, never elsewhere).
    pub fn list_live_sessions_calls(&self) -> usize {
        self.core.list_live_sessions_calls.load(Ordering::SeqCst)
    }
    /// BUG-206: how many times `list_sessions` (the disk-merging /resume
    /// list) was awaited — the auto-restore path must keep this at 0.
    pub fn list_sessions_calls(&self) -> usize {
        self.core.list_sessions_calls.load(Ordering::SeqCst)
    }
    /// TUI-113: how many times `set_active_session` was awaited.
    pub fn set_active_session_calls(&self) -> usize {
        self.core.set_active_session_calls.load(Ordering::SeqCst)
    }
    /// TUI-113: the last SessionId passed to `set_active_session`
    /// (R3 pointer-tracking assertions).
    pub fn last_set_active_session(&self) -> Option<SessionId> {
        self.core.last_set_active_session.lock().clone()
    }
    /// RPC-050: how many times `set_work_unit_context` was awaited.
    pub fn set_work_unit_context_calls(&self) -> usize {
        self.core.set_work_unit_context_calls.load(Ordering::SeqCst)
    }
    /// RPC-050: how many times `get_work_unit_context` was awaited.
    pub fn get_work_unit_context_calls(&self) -> usize {
        self.core.get_work_unit_context_calls.load(Ordering::SeqCst)
    }
    /// RPC-050: the last `(SessionId, Option<WorkUnitContext>)` passed to
    /// `set_work_unit_context`. `None` until the first call.
    pub fn last_set_work_unit_context(&self) -> Option<(SessionId, Option<WorkUnitContext>)> {
        self.core.last_set_work_unit_context.lock().clone()
    }
    /// RPC-050: force the next `set_work_unit_context` call to fail
    /// with the supplied message — exercises the error-notice branch.
    pub fn set_work_unit_context_error(&self, message: String) {
        *self.core.set_work_unit_context_error.lock() = Some(message);
    }
    /// RPC-051: seed the snapshot the App's Shift+↑ first-press fetch
    /// task receives via `persistence_get_history`. Defaults to an
    /// empty Vec for sessions that have not been scripted.
    pub fn script_history(&self, session: SessionId, entries: Vec<String>) {
        self.controls
            .scripted_history
            .lock()
            .insert(session, entries);
    }
    // ── RPC-052 helpers ──────────────────────────────────────────────────

    /// RPC-052: how many times `set_pending_input` was awaited.
    pub fn set_pending_input_calls(&self) -> usize {
        self.controls.set_pending_input_calls.load(Ordering::SeqCst)
    }
    /// RPC-052: how many times `get_pending_input` was awaited.
    pub fn get_pending_input_calls(&self) -> usize {
        self.controls.get_pending_input_calls.load(Ordering::SeqCst)
    }
    /// RPC-052: every `(SessionId, Option<String>)` write captured by
    /// `set_pending_input`, in call order. Lets debounce-coalescing
    /// tests assert that only ONE write landed AND its final value.
    pub fn pending_input_writes(&self) -> Vec<(SessionId, Option<String>)> {
        self.controls.pending_input_writes.lock().clone()
    }
    /// RPC-052: convenience accessor for the most recent
    /// `set_pending_input` argument tuple.
    pub fn last_set_pending_input(&self) -> Option<(SessionId, Option<String>)> {
        self.controls.pending_input_writes.lock().last().cloned()
    }
    /// RPC-052: seed the value `get_pending_input(session)` returns.
    /// Pass `Some(text)` to simulate a restored draft; pass `None` to
    /// simulate "no draft" (the default for un-scripted sessions is
    /// also `None`).
    pub fn script_pending_input(&self, session: SessionId, value: Option<String>) {
        self.controls
            .pending_input_store
            .lock()
            .insert(session, value);
    }
    /// RPC-052: force the next `get_pending_input` call to fail with
    /// the supplied message.
    pub fn set_get_pending_input_error(&self, message: String) {
        *self.controls.get_pending_input_error.lock() = Some(message);
    }
    /// RPC-052: force the next `set_pending_input` call to fail with
    /// the supplied message.
    pub fn set_set_pending_input_error(&self, message: String) {
        *self.controls.set_pending_input_error.lock() = Some(message);
    }
    // ── RPC-053 helpers ──────────────────────────────────────────────────

    /// RPC-053: seed the value `get_pause_state(session)` returns.
    /// Pass `Some(state)` to simulate an active pause; pass `None` to
    /// simulate "no pause" (the default for un-scripted sessions is
    /// also `None`).
    /// AMGR-020: wrapped in the route `pause-<session>`.
    pub fn script_pause_state(&self, session: SessionId, value: Option<PauseState>) {
        let value = value.map(|payload| PendingRequest {
            route: pause_route(&session),
            payload,
        });
        self.script_pending_pause(session, value);
    }
}
