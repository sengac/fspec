//! MockBackend scripting and recording helpers (part 3).

use super::*;

impl MockBackend {
    /// RPC-022: the last `(SessionId, ThinkingLevel)` pair passed to
    /// `set_thinking_level`.
    pub fn last_set_thinking_level(&self) -> Option<(SessionId, ThinkingLevel)> {
        self.core.last_set_thinking_level.lock().clone()
    }
    /// MODEL-011: script the options `get_thinking_options` returns.
    pub fn set_thinking_options(&self, options: codelet_rpc_types::ThinkingOptions) {
        *self.core.thinking_options.lock() = Some(options);
    }
    /// MODEL-011: how many times `set_thinking_level_default` was called.
    pub fn set_thinking_level_default_calls(&self) -> usize {
        self.core
            .set_thinking_level_default_calls
            .load(Ordering::SeqCst)
    }
    /// MODEL-011: the last level passed to `set_thinking_level_default`.
    pub fn last_set_thinking_level_default(&self) -> Option<ThinkingLevel> {
        self.core.last_set_thinking_level_default.lock().clone()
    }
    /// RPC-022: the last SessionId passed to `get_session_role`.
    pub fn last_get_session_role(&self) -> Option<SessionId> {
        self.core.last_get_session_role.lock().clone()
    }
    /// RPC-022: the last `(SessionId, Option<String>)` passed to
    /// `set_session_role`.
    pub fn last_set_session_role(&self) -> Option<(SessionId, Option<String>)> {
        self.core.last_set_session_role.lock().clone()
    }
    /// RPC-022: preload `(SessionId, role)` pairs so
    /// `get_session_role(id)` returns the seeded role overlay (None
    /// when the SessionId is not in the table).
    pub fn seed_session_role(&self, session: SessionId, role: Option<String>) {
        let mut roles = self.core.session_roles.lock();
        roles.retain(|(s, _)| s != &session);
        roles.push((session, role));
    }
    /// RPC-022: how many times `persistence_add_history` was awaited.
    pub fn persistence_add_history_calls(&self) -> usize {
        self.core
            .persistence_add_history_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-022: the last `(SessionId, text)` pair passed to
    /// `persistence_add_history`.
    pub fn last_persistence_add_history(&self) -> Option<(SessionId, String)> {
        self.core.last_persistence_add_history.lock().clone()
    }
    /// RPC-046: how many times `clear_history` was awaited.
    pub fn clear_history_calls(&self) -> usize {
        self.core.clear_history_calls.load(Ordering::SeqCst)
    }
    /// RPC-046: the last SessionId passed to `clear_history`.
    pub fn last_clear_history_session(&self) -> Option<SessionId> {
        self.core.last_clear_history_session.lock().clone()
    }
    /// RPC-046: force the next `clear_history` call to fail with the
    /// supplied message — exercises the error-notice branch.
    pub fn set_clear_history_error(&self, message: String) {
        *self.core.clear_history_error.lock() = Some(message);
    }
    /// RPC-047: how many times `compact_session` was awaited.
    pub fn compact_session_calls(&self) -> usize {
        self.core.compact_session_calls.load(Ordering::SeqCst)
    }
    /// RPC-047: the last SessionId passed to `compact_session`.
    pub fn last_compact_session(&self) -> Option<SessionId> {
        self.core.last_compact_session.lock().clone()
    }
    /// RPC-047: script the next `compact_session` call to return
    /// `Ok(result)`.
    pub fn set_compact_session_result_ok(&self, result: CompactionResult) {
        *self.core.compact_session_result.lock() = Ok(result);
    }
    /// RPC-047: script the next `compact_session` call to return
    /// `Err(message)` — exercises the error-notice branch.
    pub fn set_compact_session_result_err(&self, message: String) {
        *self.core.compact_session_result.lock() = Err(message);
    }
    /// RPC-049: how many times `resume_session` was awaited.
    pub fn resume_session_calls(&self) -> usize {
        self.core.resume_session_calls.load(Ordering::SeqCst)
    }
    /// RPC-049: the last SessionId passed to `resume_session`.
    pub fn last_resume_session(&self) -> Option<SessionId> {
        self.core.last_resume_session.lock().clone()
    }
    /// RPC-049: force the next `resume_session` call to fail with the
    /// supplied message — exercises the error-notice branch.
    pub fn set_resume_session_error(&self, message: String) {
        *self.core.resume_session_error.lock() = Some(message);
    }
    /// RPC-049: script the replay-buffer returned by
    /// `get_buffered_output` (used by `SessionResumeComplete`).
    pub fn set_buffered_output(&self, chunks: Vec<StreamChunk>) {
        *self.core.buffered_output.lock() = chunks;
    }
    /// TUI-115: append one chunk to the scripted transcript, the way a
    /// live session appends each event it broadcasts.
    pub fn push_buffered_output(&self, chunk: StreamChunk) {
        self.core.buffered_output.lock().push(chunk);
    }
    /// RPC-049: how many times `get_buffered_output` was awaited.
    pub fn get_buffered_output_calls(&self) -> usize {
        self.core.get_buffered_output_calls.load(Ordering::SeqCst)
    }
    /// RPC-049: the last `(SessionId, limit)` pair passed to
    /// `get_buffered_output`.
    pub fn last_get_buffered_output(&self) -> Option<(SessionId, u32)> {
        self.core.last_get_buffered_output.lock().clone()
    }
}
