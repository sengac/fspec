macro_rules! compose_handle_trait_3 {
    ($($collected:tt)*) => {
        compose_handle_trait_4! {
            $($collected)*
    /// RPC-061: remove every link in which `supervisor_id` is the
    /// supervisor. Default is `Ok(())` so handles without a chain of
    /// command compile unchanged.
    fn remove_supervisor(&self, supervisor_id: &SessionId) -> Result<(), String> {
        let _ = supervisor_id;
        Ok(())
    }
    /// RPC-061: return the first subordinate registered to a
    /// supervisor. `None` when no subordinate is bound. Backward-
    /// compatible accessor that mirrors `ChainOfCommand::get_subordinate`.
    fn get_subordinate(&self, supervisor_id: &SessionId) -> Option<SessionId> {
        let _ = supervisor_id;
        None
    }
    /// RPC-061: list every subordinate of a supervisor. Mirrors
    /// `ChainOfCommand::get_subordinates`.
    fn get_subordinates(&self, supervisor_id: &SessionId) -> Result<Vec<SessionId>, String> {
        let _ = supervisor_id;
        Err("get_subordinates is not implemented by this session manager".to_string())
    }
    /// RPC-061: queue an incoming supervisor message for a subordinate
    /// session. The production handle wraps this onto
    /// `BackgroundSession::receive_incoming_message`; the stub records
    /// the payload in `recorded_incoming_messages` so cross-transport
    /// parity tests can assert both transports reach the same in-memory
    /// state.
    fn receive_incoming_message(
        &self,
        subordinate_id: &SessionId,
        message: IncomingMessageInput,
    ) -> Result<(), String> {
        let _ = (subordinate_id, message);
        Ok(())
    }
    /// RPC-037: debug-capture toggle reader.
    fn get_debug_enabled(&self, session_id: &SessionId) -> Result<bool, String> {
        let _ = session_id;
        Err("get_debug_enabled is not implemented by this session manager".to_string())
    }
    /// RPC-037: debug-capture toggle writer.
    fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool) -> Result<(), String> {
        let _ = (session_id, enabled);
        Err("set_debug_enabled is not implemented by this session manager".to_string())
    }
    /// RPC-037: toggle debug capture; returns the resolved path string.
    fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String> {
        let _ = (session_id, debug_dir);
        Err("toggle_debug is not implemented by this session manager".to_string())
    }
    /// RPC-055: set the global debug-capture directory used by the
    /// pre-session toggle path. Mirrors the NAPI
    /// `toggle_debug(Option<String>)` global helper. Default returns
    /// `Ok(())` so handles that don't yet wire a global manager — the
    /// stub used by integration tests, plus any future minimal handles
    /// — compile unchanged. The production `codelet-sessions`
    /// `SessionManager` overrides this to delegate into
    /// `codelet_common::debug_capture::DebugCaptureManager::set_debug_directory`.
    fn set_debug_directory(&self, path: PathBuf) -> Result<(), String> {
        let _ = path;
        Err("set_debug_directory is not implemented by this session manager".to_string())
    }
    /// AMGR-023: do (Done) or cancel (Cancel) the paused session's human
    /// action named `request_id`; the reply is the user's.
    fn pause_human_action(
        &self,
        session_id: &SessionId,
        request_id: &str,
        reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<(), String> {
        let _ = (session_id, request_id, reply);
        Err("pause_human_action is not implemented by this session manager".to_string())
    }
    /// RPC-037 / AMGR-020: answer the two-choice approval named `request_id`.
    fn pause_confirm(
        &self,
        session_id: &SessionId,
        request_id: &str,
        accept: bool,
    ) -> Result<(), String> {
        let _ = (session_id, request_id, accept);
        Err("pause_confirm is not implemented by this session manager".to_string())
    }
    /// RPC-037 / AMGR-020: answer the three-choice (Approve / ApproveSession
    /// / Deny) approval named `request_id`.
    fn pause_triple(
        &self,
        session_id: &SessionId,
        request_id: &str,
        choice: ApprovalChoice,
    ) -> Result<(), String> {
        let _ = (session_id, request_id, choice);
        Err("pause_triple is not implemented by this session manager".to_string())
    }
    /// RPC-037 / AMGR-020: answer the Human-In-The-Loop question named
    /// `request_id`.
    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        request_id: &str,
        response: HitlResponse,
    ) -> Result<(), String> {
        let _ = (session_id, request_id, response);
        Err("send_hitl_response is not implemented by this session manager".to_string())
    }
    /// RPC-037 / AMGR-020: the pending approval or human action with its
    /// request route.
    fn get_pause_state(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<PendingRequest<PauseState>>, String> {
        let _ = session_id;
        Err("get_pause_state is not implemented by this session manager".to_string())
    }
    /// RPC-037 / AMGR-020: the pending HITL question with its request route.
    fn get_hitl_request(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<PendingRequest<HitlRequest>>, String> {
        let _ = session_id;
        Err("get_hitl_request is not implemented by this session manager".to_string())
    }
    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure overlay — NO status flip, NO response channel.
    fn get_exec_stdin_request(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<codelet_rpc_types::ExecStdinRequest>, String> {
        let _ = session_id;
        Err("get_exec_stdin_request is not implemented by this session manager".to_string())
    }
    /// TOOL-022 P2: write typed text to a live exec session's stdin
    /// (a trailing newline is appended when absent, matching the
    /// unified_exec `write` action semantics). Unknown agent session
    /// or unknown/exited exec session returns a clean error naming
    /// the offending id.
    fn write_exec_stdin(
        &self,
        session_id: &SessionId,
        exec_session_id: &str,
        text: &str,
    ) -> Result<(), String> {
        let _ = text;
        // BUG-369: a handle without exec-stdin support must not claim the
        // text was written.
        Err(format!(
            "write_exec_stdin is not supported by this session manager handle (session {}, exec session {exec_session_id})",
            session_id.value
        ))
    }
    /// RPC-037: round-trip an `FspecCommandRequest` reply.
    fn send_fspec_result(&self, session_id: &SessionId, result: FspecResult) -> Result<(), String> {
        let _ = (session_id, result);
        Ok(())
    }
    /// RPC-037: create an isolated (worktree-backed) session.
    fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
        let _ = role;
        Err("create_isolated_session not implemented for this handle".to_string())
    }
    /// BUG-427: create an isolated (worktree-backed) session whose project is
    /// the named workspace instead of the start workspace.
    fn create_isolated_session_in_workspace(
        &self,
        workspace: &std::path::Path,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        let _ = role;
        Err(format!(
            "creating isolated sessions in {} is not supported by this handle",
            workspace.display()
        ))
    }
    /// RPC-037: subscribe to (SessionId, SessionStatus) status pushes.
    /// Default returns a fresh broadcast whose sender is dropped
    /// immediately so subscribers observe `Closed` — handles that
    /// don't yet push status updates degrade to polling
    /// `get_session_status`.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let (tx, rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        drop(tx);
        rx
    }
    /// RPC-037: cloneable handle to the status-changes broadcast.
    /// Default returns a fresh sender that nobody subscribes to.
    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        let (tx, _rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        tx
    }
    /// RPC-385: subscribe to session-created events. Each event carries the
    /// new session's [`SessionInfo`] (its `.id` is the SessionId). The
    /// default returns a degenerate receiver whose sender has been dropped
    /// (subscribers observe `RecvError::Closed`) so handles that do not push
    /// session-created events degrade gracefully. The real `SessionManager`
    /// handle overrides this to forward `session_created_tx().subscribe()`.
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        let (tx, rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        drop(tx);
        rx
    }
    /// RPC-433: subscribe to session-removed events. Each event carries the
    /// destroyed session's [`SessionId`] — a connected client already holds
    /// all tab state for that session and needs nothing else to close the
    /// tab. The default returns a degenerate receiver whose sender has been
    /// dropped (subscribers observe `RecvError::Closed`) so handles that do
    /// not push session-removed events degrade gracefully. The real
    /// `SessionManager` handle overrides this to forward
    /// `session_removed_tx().subscribe()`; `StubSessionManagerHandle`
    /// overrides it so multi-client fan-out tests can drive removals.
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        drop(tx);
        rx
    }
    /// RPC-436: subscribe to session-cleared events. Each event carries
    /// the cleared session's [`SessionId`] — a connected client already
    /// holds all tab state for that session and needs nothing else to
    /// reset its scrollback for it (mirrors `session_removed_rx`). The
    /// default returns a degenerate receiver whose sender has been
    /// dropped (subscribers observe `RecvError::Closed`) so handles that
    /// do not push session-cleared events degrade gracefully. The real
    /// `SessionManager` handle overrides this to forward
    /// `session_cleared_tx().subscribe()`; `StubSessionManagerHandle`
    /// overrides it so multi-client fan-out tests can drive clears.
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(DEFAULT_STATUS_CHANGES_CAPACITY);
        drop(tx);
        rx
    }
    /// RPC-433: the in-memory live session map as a snapshot
    /// (`Vec<SessionInfo>`), WITHOUT the persisted-manifest merge and
    /// WITHOUT project filtering. This is the connect-time replay source
    /// for the WS server's `Envelope::SessionSnapshot` frame (rule R3 of
    /// session-events-daemon-fanout.feature): parity with the BUG-206
    /// liveness gate — implementations MUST NOT read disk. The default
    /// returns an empty list so handles without a session table
    /// (`StubSessionManagerHandle` overrides; test-only handles) compile
    /// unchanged.
    fn live_session_infos(&self) -> Vec<SessionInfo> {
        Vec::new()
    }
    /// RPC-037: destroy a session, removing it from `list_sessions`.
    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }
    // ========================================================================
    // RPC-054: Provider credentials surface. Powers the new Rust ratatui
    // `ProviderSettingsView` (`/provider` slash command). Each method has a
    // default impl so existing handles compile unchanged; the production
    // `codelet-sessions` SessionManager overrides them by delegating to
    // `codelet-providers`, and `StubSessionManagerHandle` overrides them
    // with deterministic state for cross-transport parity tests.
    // ========================================================================

    /// RPC-054: list all known providers with their configured /
    /// credential-type / model-count metadata. Drives the left pane of
    /// `ProviderSettingsView`.
    fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>, String> {
        Err("list_provider_credentials is not implemented by this session manager".to_string())
    }
    /// RPC-054: return the credential summary for a single provider, or
    /// `None` when the provider id is unknown.
    fn get_provider_credential(&self, provider_id: &str) -> Result<Option<ProviderCredentialInfo>, String> {
        let _ = provider_id;
        Err("get_provider_credential is not implemented by this session manager".to_string())
    }
    /// RPC-054: persist credentials for a provider. Returns `Err`
    /// when the input is malformed (e.g. `kind: "api_key"` with no
    /// `api_key` value), when the provider id is unknown, or on I/O
    /// failure. The TUI view surfaces the error inline in its status
    /// area.
    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        let _ = (provider_id, creds);
        Ok(())
    }
        }
    };
}
