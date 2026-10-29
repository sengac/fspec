macro_rules! compose_backend_session_context {
    ($($collected:tt)*) => {
        compose_backend_interaction! {
            $($collected)*
    async fn get_work_unit_focus(
        &self,
        _session_id: SessionId,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, BackendError> {
        Err(BackendError::WorkUnitFocus("not supported".into()))
    }

    async fn set_work_unit_focus(
        &self,
        _session_id: SessionId,
        _work_unit_id: Option<String>,
        _source: String,
    ) -> std::result::Result<codelet_rpc_types::WorkUnitFocus, BackendError> {
        Err(BackendError::WorkUnitFocus("not supported".into()))
    }

    fn work_unit_focus_rx(
        &self,
    ) -> broadcast::Receiver<(SessionId, codelet_rpc_types::WorkUnitFocus)> {
        let (sender, receiver) = broadcast::channel(1);
        drop(sender);
        receiver
    }

    /// RPC-037: read the work-unit context bound to a session.
    async fn get_work_unit_context(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        Ok(None)
    }

    /// RPC-037: bind (or detach) a work unit on a session.
    async fn set_work_unit_context(
        &self,
        _session_id: SessionId,
        _context: Option<WorkUnitContext>,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: read the per-session pending input draft.
    async fn get_pending_input(&self, _session_id: SessionId) -> Result<Option<String>> {
        Ok(None)
    }

    /// RPC-037: write the per-session pending input draft.
    async fn set_pending_input(&self, _session_id: SessionId, _text: Option<String>) -> Result<()> {
        Ok(())
    }

    /// RPC-037: set the active session for the application.
    async fn set_active_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: clear the active session.
    async fn clear_active_session(&self) -> Result<()> {
        Ok(())
    }

    /// RPC-037: read the active session, if any.
    async fn get_active_session(&self) -> Result<Option<SessionId>> {
        Ok(None)
    }

    /// RPC-037: effective cwd for a session (worktree-aware).
    async fn get_effective_cwd(&self, _session_id: SessionId) -> Result<String> {
        Ok(String::new())
    }

    /// RPC-037: list supervisor session ids for a subordinate.
    async fn get_supervisors(&self, _session_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(Vec::new())
    }

    /// RPC-061: register `supervisor_id` as a supervisor of
    /// `subordinate_id`. Default is `Ok(())` so mock backends that
    /// don't care about the supervisor surface compile unchanged.
    async fn add_supervisor(
        &self,
        _subordinate_id: SessionId,
        _supervisor_id: SessionId,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor.
    async fn remove_supervisor(&self, _supervisor_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-061: first subordinate of the supervisor, or None.
    async fn get_subordinate(&self, _supervisor_id: SessionId) -> Result<Option<SessionId>> {
        Ok(None)
    }

    /// RPC-061: every subordinate of the supervisor.
    async fn get_subordinates(&self, _supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        Ok(Vec::new())
    }

    /// RPC-061: queue a supervisor message onto a subordinate session.
    async fn receive_incoming_message(
        &self,
        _subordinate_id: SessionId,
        _message: IncomingMessageInput,
    ) -> Result<()> {
        Ok(())
    }
        }
    };
}
