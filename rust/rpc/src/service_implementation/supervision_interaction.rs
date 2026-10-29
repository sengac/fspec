macro_rules! compose_rpc_supervision_interaction {
    ($($collected:tt)*) => {
        compose_rpc_lifecycle_credentials! {
            $($collected)*
    async fn get_supervisors(self, _ctx: Context, session_id: SessionId) -> Vec<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_supervisors(&session_id),
            None => Vec::new(),
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
            None => Ok(()),
        }
    }

    async fn remove_supervisor(
        self,
        _ctx: Context,
        supervisor_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.remove_supervisor(&supervisor_id),
            None => Ok(()),
        }
    }

    async fn get_subordinate(self, _ctx: Context, supervisor_id: SessionId) -> Option<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinate(&supervisor_id),
            None => None,
        }
    }

    async fn get_subordinates(self, _ctx: Context, supervisor_id: SessionId) -> Vec<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_subordinates(&supervisor_id),
            None => Vec::new(),
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
            None => Ok(()),
        }
    }

    async fn get_debug_enabled(self, _ctx: Context, session_id: SessionId) -> bool {
        match self.inner.session_manager() {
            Some(handle) => handle.get_debug_enabled(&session_id),
            None => false,
        }
    }

    async fn set_debug_enabled(self, _ctx: Context, session_id: SessionId, enabled: bool) {
        if let Some(handle) = self.inner.session_manager() {
            handle.set_debug_enabled(&session_id, enabled);
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
            None => Ok(String::new()),
        }
    }

    async fn set_debug_directory(self, _ctx: Context, path: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.set_debug_directory(std::path::PathBuf::from(path)),
            None => Ok(()),
        }
    }

    async fn pause_resume(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_resume(&session_id),
            None => Ok(()),
        }
    }

    async fn pause_confirm(
        self,
        _ctx: Context,
        session_id: SessionId,
        accept: bool,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_confirm(&session_id, accept),
            None => Ok(()),
        }
    }

    async fn pause_triple(
        self,
        _ctx: Context,
        session_id: SessionId,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.pause_triple(&session_id, choice),
            None => Ok(()),
        }
    }

    async fn send_hitl_response(
        self,
        _ctx: Context,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.send_hitl_response(&session_id, response),
            None => Ok(()),
        }
    }

    async fn get_pause_state(self, _ctx: Context, session_id: SessionId) -> Option<PauseState> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_pause_state(&session_id),
            None => None,
        }
    }

    async fn get_hitl_request(self, _ctx: Context, session_id: SessionId) -> Option<HitlRequest> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_hitl_request(&session_id),
            None => None,
        }
    }

    async fn get_exec_stdin_request(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Option<ExecStdinRequest> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_exec_stdin_request(&session_id),
            None => None,
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
