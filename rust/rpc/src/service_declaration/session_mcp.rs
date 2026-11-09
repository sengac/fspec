macro_rules! compose_service_session_mcp {
    ($($collected:tt)*) => {
        compose_service_finish! {
            $($collected)*
    // RPC-449: new methods are appended here, after every existing method.

    /// MCP-003: set one live, idle session's MCP servers; MCP-008: the list is
    /// the session's whole MCP set (unlisted servers are disconnected, an empty
    /// list is valid) and a failure leaves the previous set intact. Refused unless the daemon was started with FSPEC_DAEMON_TOKEN and
    /// this connection authenticated with its bearer.
    async fn configure_session_mcp_servers(
        session_id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>, String>;

    /// TOOL-027: send an interrupt (SIGINT to the command only) or end of
    /// input to a live exec session of an agent session.
    async fn control_exec_stdin(
        session_id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<(), String>;

    /// BUG-427: create an isolated (worktree-backed) session whose project is
    /// the named workspace, opening that workspace on the daemon first.
    async fn create_isolated_session_in_workspace(
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String>;

    /// BUG-431: the bytes of a card attachment in the named workspace, read
    /// and validated by the daemon (the client may be on another machine).
    async fn read_card_attachment(workspace: String, path: String) -> Result<Vec<u8>, String>;

    /// RPC-452: the changed files of the named workspace (opened on the
    /// daemon). A git failure or an unknown workspace is an error.
    async fn changed_files_in_workspace(workspace: String) -> Result<Vec<ChangedFile>, String>;

    /// RPC-452: the diff of `path` in the named workspace; `None` when the
    /// file has no diff. A git failure or an unknown workspace is an error.
    async fn file_diff_in_workspace(
        workspace: String,
        path: String,
    ) -> Result<Option<String>, String>;

    /// RPC-453: the named workspace's checkpoint counts (a git failure is
    /// an error, not zero counts).
    async fn checkpoint_counts_in_workspace(workspace: String) -> Result<CheckpointCounts, String>;

    /// RPC-453: the named workspace's checkpoints (progress frames go out on
    /// the shared checkpoints-progress broadcast).
    async fn list_checkpoints_in_workspace(workspace: String) -> Result<Vec<CheckpointInfo>, String>;

    /// RPC-453: the files a checkpoint of the named workspace changes.
    async fn checkpoint_diff_files_in_workspace(
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>, String>;

    /// RPC-453: the diff of one file in a checkpoint of the named workspace.
    async fn checkpoint_file_diff_in_workspace(
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>, String>;

    /// RPC-453: restore one file from a checkpoint of the named workspace.
    async fn restore_checkpoint_file_in_workspace(
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<(), String>;

    /// RPC-453: restore every file of a checkpoint of the named workspace.
    async fn restore_checkpoint_all_in_workspace(
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String>;

    /// RPC-453: delete one checkpoint of the named workspace.
    async fn delete_checkpoint_in_workspace(
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String>;

    /// RPC-453: delete every checkpoint of the named workspace.
    async fn delete_all_checkpoints_in_workspace(workspace: String) -> Result<(), String>;

    /// RPC-454: the named workspace's directory and branch for the footer
    /// (`None` branch when it is not a git repository).
    async fn get_workspace_info_in_workspace(workspace: String) -> Result<WorkspaceInfo, String>;

    /// RPC-454: the named workspace's current git state (live updates follow
    /// on the workspace git-state push).
    async fn git_state_in_workspace(workspace: String) -> Result<GitState, String>;

    /// RPC-455: `search_files` in the named workspace.
    async fn search_files_in_workspace(
        workspace: String,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>, String>;

    /// RPC-455: `persistence_add_history` recorded under the named workspace.
    async fn persistence_add_history_in_workspace(
        workspace: String,
        session: SessionId,
        text: String,
    ) -> Result<(), String>;

    /// RPC-455: `persistence_get_history` of the named workspace.
    async fn persistence_get_history_in_workspace(
        workspace: String,
        session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>, String>;

    /// RPC-455: `persistence_search_history` within the named workspace.
    async fn persistence_search_history_in_workspace(
        workspace: String,
        query: String,
    ) -> Result<Vec<HistoryMatch>, String>;

    /// RPC-457: add the fspec project at `path` to the workspace registry.
    async fn add_workspace(path: String) -> Result<WorkspaceEntry, String>;

    /// RPC-457: forget the workspace at `path` and drop the daemon's cached
    /// scope; the start workspace cannot be removed.
    async fn remove_workspace(path: String) -> Result<(), String>;

    /// MODEL-011: the thinking levels the session's current model offers,
    /// or that it has no reasoning control (the thinking dialog and
    /// `/thinking` read it). Appended: variant indices are frozen (RPC-449).
    async fn get_thinking_options(
        session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions, String>;

    /// BUG-541: `search_files` in the session's effective working directory
    /// (an isolated session's worktree). Appended (RPC-449).
    async fn search_files_in_session(
        session_id: SessionId,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>, String>;

    /// BUG-601: the blocklist rule ids turned off for the live session
    /// (sorted); they are off for that session only. Appended (RPC-449).
    async fn blocklist_disabled_rules(session_id: SessionId) -> Result<Vec<String>, String>;

    /// BUG-601: turn the blocklist rule `rule_id` off (`enabled` false) or
    /// on for the live session, saved with the session; answers its
    /// turned-off rule ids (sorted). Appended (RPC-449).
    async fn blocklist_set_rule_enabled(
        session_id: SessionId,
        rule_id: String,
        enabled: bool,
    ) -> Result<Vec<String>, String>;
    /// WT-018: the cards an isolated session created, moved or edited on
    /// the shared board (its board writes carry its session id), read
    /// from the session's main checkout.
    async fn session_card_changes(
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>, String>;

    /// WT-018: after a discard, move every card the session moved back to
    /// its status before the session, from the main checkout (a card
    /// someone else changed since is not moved; a refused move is reported
    /// with the gate's error and the other cards still move).
    async fn move_session_cards_back(
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>, String>;

    /// AMGR-023: do (Done) or cancel (Cancel) the human action named
    /// `request_id`; the one reply to a human action.
    async fn pause_human_action(
        session_id: SessionId,
        request_id: String,
        reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<(), String>;
        }
    };
}
