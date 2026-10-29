macro_rules! compose_backend_interaction {
    ($($collected:tt)*) => {
        compose_backend_credentials! {
            $($collected)*
    /// RPC-037: debug-capture toggle reader.
    async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> {
        Ok(false)
    }

    /// RPC-037: debug-capture toggle writer.
    async fn set_debug_enabled(&self, _session_id: SessionId, _enabled: bool) -> Result<()> {
        Ok(())
    }

    /// RPC-037: toggle debug capture; returns the resolved path string.
    async fn toggle_debug(&self, _session_id: SessionId, _debug_dir: String) -> Result<String> {
        Ok(String::new())
    }

    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Default is `Ok(())` so transports
    /// without a session manager attached compile unchanged.
    async fn set_debug_directory(&self, _path: String) -> Result<()> {
        Ok(())
    }

    /// RPC-037: resume a paused session.
    async fn pause_resume(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }

    /// RPC-037: respond to a two-choice confirm pause.
    async fn pause_confirm(&self, _session_id: SessionId, _accept: bool) -> Result<()> {
        Ok(())
    }

    /// RPC-037: respond to a three-choice approval pause.
    async fn pause_triple(&self, _session_id: SessionId, _choice: ApprovalChoice) -> Result<()> {
        Ok(())
    }

    /// RPC-037: send a Human-In-The-Loop response.
    async fn send_hitl_response(
        &self,
        _session_id: SessionId,
        _response: HitlResponse,
    ) -> Result<()> {
        Ok(())
    }

    /// RPC-037: snapshot of the pause dialog state.
    async fn get_pause_state(&self, _session_id: SessionId) -> Result<Option<PauseState>> {
        Ok(None)
    }

    /// RPC-037: snapshot of the active HITL request, if any.
    async fn get_hitl_request(&self, _session_id: SessionId) -> Result<Option<HitlRequest>> {
        Ok(None)
    }

    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure overlay — NO status flip, NO response channel.
    async fn get_exec_stdin_request(
        &self,
        _session_id: SessionId,
    ) -> Result<Option<codelet_rpc_types::ExecStdinRequest>> {
        Ok(None)
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
        Ok(())
    }

    /// RPC-037: round-trip an FspecCommandRequest reply.
    async fn send_fspec_result(&self, _session_id: SessionId, _result: FspecResult) -> Result<()> {
        Ok(())
    }

    /// RPC-037: create an isolated (worktree-backed) session.
    async fn create_isolated_session(&self, _role: Option<String>) -> Result<IsolatedSessionInfo> {
        Ok(IsolatedSessionInfo {
            session_id: SessionId::new(String::new()),
            worktree_path: String::new(),
            base_commit: String::new(),
        })
    }

    /// RPC-037: destroy a session, removing it from `list_sessions`.
    async fn destroy_session(&self, _session_id: SessionId) -> Result<()> {
        Ok(())
    }
        }
    };
}
