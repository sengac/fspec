macro_rules! compose_embedded_subscriptions {
    ($($collected:tt)*) => {
        compose_embedded_blocklist_session_rules! {
            $($collected)*
    /// RPC-037: subscribe to the session manager's status broadcast.
    /// On the embedded path we read it directly from the
    /// `SharedFspecService`'s attached `SessionManagerHandle` so the
    /// receiver bypasses tarpc entirely (zero-cost path per RPC-002 §5.1).
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match self.service.session_manager() {
            Some(handle) => handle.status_changes_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-385: subscribe to the session manager's session-created broadcast.
    /// Read directly from the `SharedFspecService`'s attached
    /// `SessionManagerHandle` so the embedded TUI is notified of every newly
    /// created session — including spawned subordinates that bypass the
    /// TUI-initiated creation paths — bypassing tarpc entirely (the same
    /// zero-cost path `status_changes_rx` uses).
    fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        match self.service.session_manager() {
            Some(handle) => handle.session_created_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-434: subscribe to the session manager's session-removed
    /// broadcast (zero-cost embedded path — the same direct handle read
    /// `status_changes_rx` / `session_created_rx` use). A session
    /// destroyed on the daemon fires `SessionEvent::Removed` on every
    /// connected client; the embedded TUI observes it here.
    fn session_removed_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match self.service.session_manager() {
            Some(handle) => handle.session_removed_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-437: subscribe to the session manager's session-cleared
    /// broadcast (zero-cost embedded path — the same direct handle read
    /// `status_changes_rx` / `session_created_rx` use). A session
    /// cleared on the daemon (from ANY connected machine, including
    /// the NAPI/Telegram entry point) fires `SessionEvent::Cleared`
    /// on every connected client; the embedded TUI observes it here.
    /// Unlike `session_removed_rx`, the session stays LIVE — clear is
    /// repeatable, so the receiver must never be treated as
    /// terminal.
    fn session_cleared_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match self.service.session_manager() {
            Some(handle) => handle.session_cleared_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-434: the connect-time open-session snapshot — delegates to
    /// the shared service's in-memory live map (no disk merge, no
    /// project filter — parity with the BUG-206 liveness gate).
    async fn session_snapshot(&self) -> Vec<codelet_rpc_types::SessionInfo> {
        self.service.live_sessions_snapshot()
    }

    /// TUI-109: subscribe to the checkpoint-enumeration progress
    /// broadcast. The channel lives directly on the
    /// `SharedFspecService` (not the session manager), so the embedded
    /// path forwards it with zero cost — `FspecServiceImpl::
    /// list_checkpoints` publishes a frame per collected item while the
    /// final Vec still returns through the tarpc RPC.
    fn checkpoints_progress_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress> {
        self.service.checkpoints_progress_rx()
    }

    /// BUG-182: forward the shared service's git-state push so live
    /// `GitState` frames (watcher-driven, cwd-attached) reach the App
    /// subscriber on the embedded transport.
    fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        self.service.git_state_changed_rx()
    }
    /// BOARD-024: thin delegation to the shared canonical details RPC.
    async fn get_work_unit_details(
        &self,
        workspace: String,
        id: String,
    ) -> std::result::Result<String, super::BackendError> {
        self.client
            .get_work_unit_details(context::current(), workspace, id)
            .await
            .map_err(|error| super::BackendError::WorkUnitDetails(error.to_string()))?
            .map_err(super::BackendError::WorkUnitDetails)
    }

    async fn session_card_changes(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>> {
        self.client
            .session_card_changes(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn move_session_cards_back(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>> {
        self.client
            .move_session_cards_back(context::current(), session_id)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        self.client
            .list_workspaces(context::current())
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn list_workspace_work_units(&self, workspace: String) -> Result<Vec<WorkUnitInfo>> {
        self.client
            .list_workspace_work_units(context::current(), workspace)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn create_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<SessionId> {
        self.client
            .create_session_in_workspace(context::current(), workspace, role)
            .await?
            .map_err(anyhow::Error::msg)
    }

    fn workspace_work_units_rx(&self) -> broadcast::Receiver<WorkspaceWorkUnits> {
        self.service.workspace_work_units_rx()
    }

    fn workspace_git_state_rx(&self) -> broadcast::Receiver<codelet_rpc_types::WorkspaceGitState> {
        self.service.workspace_git_state_rx()
    }

    /// RPC-440: read the daemon's default model.
    async fn get_default_model(&self) -> Result<Option<String>> {
        Ok(self.client.get_default_model(context::current()).await?)
    }

    /// RIG-016: steer, queue or interrupt through the daemon.
    async fn submit_input(
        &self,
        id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<()> {
        self.client
            .submit_input(context::current(), id, text, mode)
            .await?
            .map_err(anyhow::Error::msg)
    }

    /// MCP-003: an in-process connection is never bearer-authenticated, so
    /// the daemon refuses; the refusal is returned as it is.
    async fn configure_session_mcp_servers(
        &self,
        id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>> {
        self.client
            .configure_session_mcp_servers(context::current(), id, servers)
            .await?
            .map_err(anyhow::Error::msg)
    }

    /// TOOL-027: forward an exec-session control to the in-process daemon.
    async fn control_exec_stdin(
        &self,
        id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<()> {
        self.client
            .control_exec_stdin(context::current(), id, exec_session_id, control)
            .await?
            .map_err(anyhow::Error::msg)
    }

    async fn create_isolated_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        self.client
            .create_isolated_session_in_workspace(context::current(), workspace, role)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn read_card_attachment(&self, workspace: String, path: String) -> Result<Vec<u8>> {
        self.client
            .read_card_attachment(context::current(), workspace, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn changed_files_in_workspace(
        &self,
        workspace: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        self.client
            .changed_files_in_workspace(context::current(), workspace)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn file_diff_in_workspace(&self, workspace: String, path: String) -> Result<Option<String>> {
        self.client
            .file_diff_in_workspace(context::current(), workspace, path)
            .await?
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_thinking_options(
        &self,
        session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions> {
        Ok(self
            .client
            .get_thinking_options(context::current(), session_id)
            .await?
            .map_err(|error| anyhow::anyhow!(error))?)
    }
    // BUG-541: file search in the session's working directory.
    async fn search_files_in_session(
        &self,
        session_id: SessionId,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>> {
        self.client
            .search_files_in_session(context::current(), session_id, prefix, limit)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
        }
    };
}
