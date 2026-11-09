macro_rules! compose_rpc_supervision_interaction {
    ($($collected:tt)*) => {
        compose_rpc_lifecycle_credentials! {
            $($collected)*
    async fn get_supervisors(self, _ctx: Context, session_id: SessionId) -> Result<Vec<SessionId>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_supervisors(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn add_supervisor(
        self,
        _ctx: Context,
        subordinate_id: SessionId,
        supervisor_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.add_supervisor(&subordinate_id, &supervisor_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn remove_supervisor(
        self,
        _ctx: Context,
        supervisor_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.remove_supervisor(&supervisor_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_subordinate(self, _ctx: Context, supervisor_id: SessionId) -> Option<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinate(&supervisor_id),
            None => None,
        }
    }

    async fn get_subordinates(self, _ctx: Context, supervisor_id: SessionId) -> Result<Vec<SessionId>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinates(&supervisor_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn receive_incoming_message(
        self,
        _ctx: Context,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.receive_incoming_message(&subordinate_id, message),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_debug_enabled(self, _ctx: Context, session_id: SessionId) -> Result<bool, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_debug_enabled(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_debug_enabled(self, _ctx: Context, session_id: SessionId, enabled: bool) -> Result<(), String> {
        // BUG-464: no session manager and an unknown session are errors.
        match self.inner.session_manager() {
            Some(handle) => handle.set_debug_enabled(&session_id, enabled),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn toggle_debug(
        self,
        _ctx: Context,
        session_id: SessionId,
        debug_dir: String,
    ) -> Result<String, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.toggle_debug(&session_id, &debug_dir),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn set_debug_directory(self, _ctx: Context, path: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_debug_directory(std::path::PathBuf::from(path)),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    /// AMGR-023: removed; the slot keeps its frozen index (RPC-449) and
    /// answers nothing.
    async fn pause_resume(
        self,
        _ctx: Context,
        _session_id: SessionId,
        request_id: String,
    ) -> Result<(), String> {
        Err(format!(
            "pause_resume is removed and answers nothing; answer request {request_id} with pause_human_action"
        ))
    }

    async fn pause_confirm(
        self,
        _ctx: Context,
        session_id: SessionId,
        request_id: String,
        accept: bool,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_confirm(&session_id, &request_id, accept),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn pause_triple(
        self,
        _ctx: Context,
        session_id: SessionId,
        request_id: String,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_triple(&session_id, &request_id, choice),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn send_hitl_response(
        self,
        _ctx: Context,
        session_id: SessionId,
        request_id: String,
        response: HitlResponse,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.send_hitl_response(&session_id, &request_id, response),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_pause_state(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Option<PendingRequest<PauseState>>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pause_state(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_hitl_request(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Option<PendingRequest<HitlRequest>>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_hitl_request(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_exec_stdin_request(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Option<ExecStdinRequest>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_exec_stdin_request(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn write_exec_stdin(
        self,
        _ctx: Context,
        session_id: SessionId,
        exec_session_id: String,
        text: String,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.write_exec_stdin(&session_id, &exec_session_id, &text),
            None => Err("write_exec_stdin requires a SessionManagerHandle".to_string()),
        }
    }
        }
    };
}
