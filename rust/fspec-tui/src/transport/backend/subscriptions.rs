macro_rules! compose_backend_subscriptions {
    ($($collected:tt)*) => {
        compose_backend_blocklist_session_rules! {
            $($collected)*
    /// RPC-037: subscribe to push-driven (SessionId, SessionStatus)
    /// transitions. Mirror of `chunks_rx` / `logs_rx` — each call
    /// returns a fresh receiver bound to the per-transport broadcast.
    /// The default returns a closed receiver so transports that don't
    /// yet wire push-status compile unchanged and gracefully degrade.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-385: subscribe to session-created events so the embedded TUI can
    /// append a tab for sessions it did not itself initiate (spawned
    /// subordinates via AgentManager). The payload is the new session's
    /// [`SessionInfo`] (its `.id` carries the SessionId). The embedded
    /// backend forwards the SessionManager's `session_created_tx().subscribe()`;
    /// the default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged and gracefully degrade. Full remote
    /// (tarpc/websocket) parity is an explicit OUT-OF-SCOPE RPC follow-up.
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-434: subscribe to session-removed events — a session was
    /// destroyed on the daemon (from this client or ANY other connected
    /// machine). The payload is the destroyed session's `SessionId`
    /// (nothing else: the client already holds all tab state). The
    /// embedded backend forwards the SessionManager's
    /// `session_removed_tx()`; the WebSocket backend splits the RPC-433
    /// `SessionEvent` wire stream onto this per-variant receiver. The
    /// default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged (RPC-037 / PROV-109
    /// convention) — the App subscriber observes `RecvError::Closed`
    /// and exits (respawned on Reconnected per R5).
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-437: subscribe to session-cleared events — a session's
    /// history was cleared on the daemon (from this client or ANY other
    /// connected machine, including the NAPI/Telegram clear entry
    /// point). The payload is the cleared session's `SessionId` (the
    /// session stays LIVE — unlike `Removed`, nothing is destroyed).
    /// The embedded backend forwards the SessionManager's
    /// `session_cleared_tx()`; the WebSocket backend splits the RPC-433
    /// `SessionEvent` wire stream onto this per-variant receiver. The
    /// default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged (RPC-037 / PROV-109
    /// convention) — the App subscriber observes `RecvError::Closed`
    /// and exits (respawned on Reconnected per R5).
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-434: the connect-time open-session snapshot — the
    /// in-memory live session set the daemon holds at connect time
    /// (WS: the server-pushed `Envelope::SessionSnapshot` frame;
    /// embedded: `SharedFspecService::live_sessions_snapshot()`).
    /// `App::bootstrap` / `handle_reconnected` fold every entry
    /// through the existing `Action::SessionCreated` handler
    /// (idempotent append) BEFORE `spawn_session_restore`. The default
    /// returns an empty Vec so test doubles and degenerate backends
    /// compile unchanged — an empty snapshot seeds no tabs.
    async fn session_snapshot(&self) -> Vec<SessionInfo> {
        Vec::new()
    }

    /// TUI-109: subscribe to per-item checkpoint-enumeration progress
    /// frames (the `CheckpointsProgress` broadcast fed by
    /// `FspecServiceImpl::list_checkpoints`). The embedded transport
    /// forwards `SharedFspecService::checkpoints_progress_rx()`; the
    /// default returns a closed receiver so transports that don't
    /// forward the frames (e.g. WebSocket) degrade to spinner + stage
    /// label only — the subscriber observes `RecvError::Closed`
    /// immediately and the dialog never receives a counter (TUI-107
    /// behavior, no timeout logic needed).
    fn checkpoints_progress_rx(&self) -> broadcast::Receiver<CheckpointsProgress> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// BUG-182: subscribe to the git-state push — fresh `GitState`
    /// snapshots broadcast by the shared layer's `GitStateWatcher` on
    /// every debounced `.git` change and every poll tick that produced a
    /// different snapshot (dedup). The embedded transport forwards
    /// `SharedFspecService::git_state_changed_rx()`; the default returns
    /// a closed receiver so transports that don't forward the frames
    /// (e.g. WebSocket) degrade gracefully — the App subscriber observes
    /// `RecvError::Closed` immediately and the header falls back to the
    /// bootstrap value + in-view `RefreshCheckpointCounts` re-poll
    /// (documented, like `checkpoints_progress_rx` for TUI-109).
    fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// BOARD-024: complete canonical work-unit record as the JSON string
    /// envelope `{id, workspace, workUnit}`. Backends that cannot reach the
    /// daemon's canonical store report an explicit unsupported error.
    /// RPC-450: `workspace` names the board's workspace (its path).
    async fn get_work_unit_details(
        &self,
        workspace: String,
        id: String,
    ) -> std::result::Result<String, BackendError> {
        let _ = workspace;
        Err(BackendError::WorkUnitDetails(format!(
            "unsupported backend: cannot read canonical details for {id}"
        )))
    }

    /// WT-018: the cards a session created, moved or edited on the
    /// shared board (none when it changed no card).
    async fn session_card_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>> {
        Err(BackendError::Unimplemented("session_card_changes").into())
    }

    /// WT-018: move every card the session moved back to its status
    /// before the session, from the main checkout.
    async fn move_session_cards_back(
        &self,
        _session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>> {
        Err(BackendError::Unimplemented("move_session_cards_back").into())
    }

    /// RPC-445: every workspace the daemon serves, start workspace first.
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceEntry>> {
        Err(BackendError::Unimplemented("list_workspaces").into())
    }

    /// RPC-445: the work units of the named workspace.
    async fn list_workspace_work_units(&self, workspace: String) -> Result<Vec<WorkUnitInfo>> {
        Err(anyhow::anyhow!("workspace {workspace} is not available on this backend"))
    }

    /// RPC-445: create a session whose project is the named workspace.
    async fn create_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<SessionId> {
        Err(anyhow::anyhow!("cannot create a session in {workspace} on this backend ({role:?})"))
    }

    /// RPC-445: work-unit pushes for workspaces opened besides the start
    /// workspace, tagged with their canonical path.
    fn workspace_work_units_rx(&self) -> broadcast::Receiver<WorkspaceWorkUnits> {
        broadcast::channel(1).1
    }

    /// RPC-454: git-state pushes of workspaces opened besides the start
    /// workspace. A backend without them yields a closed receiver.
    fn workspace_git_state_rx(&self) -> broadcast::Receiver<codelet_rpc_types::WorkspaceGitState> {
        broadcast::channel(1).1
    }

    /// RPC-440: read the daemon's default model. Default `Ok(None)` so
    /// mock/test backends compile unchanged; both transports override it.
    async fn get_default_model(&self) -> Result<Option<String>> {
        Err(BackendError::Unimplemented("get_default_model").into())
    }

    /// RIG-016: submit input under a delivery mode (steer, queue or
    /// interrupt). BUG-331: required — every backend states how it delivers
    /// each mode; both transports use the `submit_input` RPC.
    async fn submit_input(
        &self,
        id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<()>;

    /// MCP-003: configure a session's MCP servers. Required: the daemon only
    /// accepts it on a bearer-authenticated WebSocket connection.
    async fn configure_session_mcp_servers(
        &self,
        id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>>;

    /// TOOL-027: send an interrupt (SIGINT to the command only) or end of
    /// input to a live exec session of agent session `id`.
    async fn control_exec_stdin(
        &self,
        id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<()>;

    /// BUG-427: create an isolated session whose project is the named
    /// workspace. A backend without the RPC refuses.
    async fn create_isolated_session_in_workspace(
        &self,
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo> {
        Err(anyhow::anyhow!(
            "cannot create an isolated session in {workspace} on this backend ({role:?})"
        ))
    }

    /// BUG-431: the bytes of a card attachment in `workspace`, read by the
    /// daemon. A backend without the RPC refuses.
    async fn read_card_attachment(&self, workspace: String, path: String) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!(
            "cannot read attachment {path} of {workspace} on this backend"
        ))
    }

    /// RPC-452: the changed files of `workspace`. A backend without the RPC
    /// refuses.
    async fn changed_files_in_workspace(
        &self,
        workspace: String,
    ) -> Result<Vec<codelet_rpc_types::ChangedFile>> {
        Err(anyhow::anyhow!(
            "cannot list the changed files of {workspace} on this backend"
        ))
    }

    /// RPC-452: the diff of `path` in `workspace`. A backend without the RPC
    /// refuses.
    async fn file_diff_in_workspace(&self, workspace: String, path: String) -> Result<Option<String>> {
        Err(anyhow::anyhow!(
            "cannot read the diff of {path} in {workspace} on this backend"
        ))
    }

    // MODEL-011: the thinking levels the session's model offers; a backend
    // without the RPC refuses (appended, RPC-449).
    async fn get_thinking_options(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions> {
        Err(BackendError::Unimplemented("get_thinking_options").into())
    }
    // BUG-541: file search in a session's working directory (its worktree
    // when isolated); a backend without the RPC refuses.
    async fn search_files_in_session(
        &self,
        _session_id: SessionId,
        _prefix: String,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("search_files_in_session").into())
    }
        }
    };
}
