macro_rules! compose_backend_interaction {
    ($($collected:tt)*) => {
        compose_backend_credentials! {
            $($collected)*
    /// RPC-037: debug-capture toggle reader.
    async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> {
        Err(BackendError::Unimplemented("get_debug_enabled").into())
    }

    /// RPC-037: debug-capture toggle writer.
    async fn set_debug_enabled(&self, _session_id: SessionId, _enabled: bool) -> Result<()> {
        Err(BackendError::Unimplemented("set_debug_enabled").into())
    }

    /// RPC-037: toggle debug capture; returns the resolved path string.
    async fn toggle_debug(&self, _session_id: SessionId, _debug_dir: String) -> Result<String> {
        Err(BackendError::Unimplemented("toggle_debug").into())
    }

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Default is `Ok(())` so transports
    /// without a session manager attached compile unchanged.
    async fn set_debug_directory(&self, _path: String) -> Result<()> {
        Err(BackendError::Unimplemented("set_debug_directory").into())
    }

    /// AMGR-023: do or cancel the human action named `request_id`.
    async fn pause_human_action(
        &self,
        _session_id: SessionId,
        _request_id: String,
        _reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<()> {
        Err(BackendError::Unimplemented("pause_human_action").into())
    }

    /// RPC-037 / AMGR-020: answer the two-choice approval named `request_id`.
    async fn pause_confirm(
        &self,
        _session_id: SessionId,
        _request_id: String,
        _accept: bool,
    ) -> Result<()> {
        Err(BackendError::Unimplemented("pause_confirm").into())
    }

    /// RPC-037 / AMGR-020: answer the three-choice approval named `request_id`.
    async fn pause_triple(
        &self,
        _session_id: SessionId,
        _request_id: String,
        _choice: ApprovalChoice,
    ) -> Result<()> {
        Err(BackendError::Unimplemented("pause_triple").into())
    }

    /// RPC-037 / AMGR-020: answer the HITL question named `request_id`.
    async fn send_hitl_response(
        &self,
        _session_id: SessionId,
        _request_id: String,
        _response: HitlResponse,
    ) -> Result<()> {
        Err(BackendError::Unimplemented("send_hitl_response").into())
    }

    /// RPC-037 / AMGR-020: the pending approval with its request route.
    async fn get_pause_state(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<PendingRequest<PauseState>>> {
        Err(BackendError::Unimplemented("get_pause_state").into())
    }

    /// RPC-037 / AMGR-020: the pending HITL question with its request route.
    async fn get_hitl_request(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<PendingRequest<HitlRequest>>> {
        Err(BackendError::Unimplemented("get_hitl_request").into())
    }

    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure overlay — NO status flip, NO response channel.
    async fn get_exec_stdin_request(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<codelet_rpc_types::ExecStdinRequest>> {
        Err(BackendError::Unimplemented("get_exec_stdin_request").into())
    }

    /// TOOL-022 P2: write typed text to a live exec session's stdin
    /// (a trailing newline is appended when absent). The backend's
    /// error string is surfaced verbatim (it names the exec session id
    /// for unknown/exited sessions).
    async fn write_exec_stdin(
        &self,
        _session_id: SessionId,
        _exec_session_id: String,
        _text: String,
    ) -> Result<(), String> {
        Err("write_exec_stdin is not implemented by this backend".to_string())
    }

    /// RPC-037: round-trip an FspecCommandRequest reply.
    async fn send_fspec_result(&self, _session_id: SessionId, _result: FspecResult) -> Result<()> {
        Err(BackendError::Unimplemented("send_fspec_result").into())
    }

    /// RPC-037: create an isolated (worktree-backed) session.
    async fn create_isolated_session(&self, _role: Option<String>) -> Result<IsolatedSessionInfo> {
        Err(BackendError::Unimplemented("create_isolated_session").into())
    }

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    async fn destroy_session(&self, _session_id: SessionId) -> Result<()> {
        Err(BackendError::Unimplemented("destroy_session").into())
    }
        }
    };
}
