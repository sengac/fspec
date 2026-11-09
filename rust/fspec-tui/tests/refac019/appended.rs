//! Approved additive tails excluded from the REFAC-019 frozen baseline.

/// Members appended after the frozen baseline by approved additive work:
/// BOARD-024 canonical details (one RPC/backend method and its Actions),
/// RPC-443/444/445 workspaces, RPC-448 daemon switching and RPC-440
/// get_default_model, TUI-114 New Bot, RIG-016 submit_input/InputQueued,
/// MCP-003 configure_session_mcp_servers and TOOL-027 control_exec_stdin
/// (approved by BUG-370), the TUI-116 ExecStdinControl Action, and the TUI-117
/// ExecStdinWriteFailed / ExecStdinControlFailed / ExecStdinMuted Actions, and the
/// BUG-364 ExecStdinProbeFailed, BUG-394 PausedReadFailed and BUG-395 PausedWriteFailed Actions,
/// BUG-427 create_isolated_session_in_workspace and BUG-431 read_card_attachment, and MODEL-011
/// get_thinking_options with its ThinkingOptionsLoaded / ThinkingLevelRefused Actions, and the TUI-123
/// ConfirmRemoveSavedDaemon Action, and BUG-541 search_files_in_session, and BUG-601
/// blocklist_disabled_rules / blocklist_set_rule_enabled with the BlocklistDisabledRulesLoaded Action. Only a trailing run of these names is stripped, so any change to the frozen
/// ordered prefix still fails the baseline snapshot
/// (RPC-449: new members are appended, never inserted).
const APPROVED_APPENDED_TAILS: [&str; 85] = [
    "get_work_unit_details",
    "OpenBoardCardChooser",
    "RequestBoardCardDetails",
    "BoardCardDetailsLoaded",
    "BoardCardAttachmentFailed",
    "list_workspaces",
    "list_workspace_work_units",
    "create_session_in_workspace",
    "workspace_work_units_rx",
    "get_default_model",
    "WorkspacesLoaded",
    "OpenWorkspaceDropdown",
    "CloseWorkspaceDropdown",
    "MoveWorkspaceHighlight",
    "ChooseWorkspace",
    "WorkspaceWorkUnitsLoaded",
    "OpenDaemonSwitcher",
    "CloseDaemonSwitcher",
    "MoveDaemonHighlight",
    "ChooseDaemon",
    "DaemonSwitched",
    "OpenNewBotDialog",
    "CreateBotSubmitted",
    "BotSessionCreated",
    "submit_input",
    "InputQueued",
    "configure_session_mcp_servers",
    "control_exec_stdin",
    "ExecStdinControl",
    "ExecStdinWriteFailed",
    "ExecStdinControlFailed",
    "ExecStdinMuted",
    "ExecStdinProbeFailed",
    "PausedReadFailed",
    "PausedWriteFailed",
    "create_isolated_session_in_workspace",
    "read_card_attachment",
    "changed_files_in_workspace",
    "file_diff_in_workspace",
    "checkpoint_counts_in_workspace",
    "list_checkpoints_in_workspace",
    "checkpoint_diff_files_in_workspace",
    "checkpoint_file_diff_in_workspace",
    "restore_checkpoint_file_in_workspace",
    "restore_checkpoint_all_in_workspace",
    "delete_checkpoint_in_workspace",
    "delete_all_checkpoints_in_workspace",
    "get_workspace_info_in_workspace",
    "git_state_in_workspace",
    "WorkspaceGitStateChanged",
    "workspace_git_state_rx",
    "search_files_in_workspace",
    "persistence_add_history_in_workspace",
    "persistence_get_history_in_workspace",
    "persistence_search_history_in_workspace",
    "add_workspace",
    "remove_workspace",
    "OpenAddWorkspaceDialog",
    "AddWorkspace",
    "WorkspaceAddFailed",
    "WorkspaceAdded",
    "RemoveWorkspace",
    "WorkspaceRemoved",
    "OpenAddDaemonForm",
    "OpenEditDaemonForm",
    "SaveDaemon",
    "RemoveSavedDaemon",
    "ConfirmRemoveSavedDaemon",
    "get_thinking_options",
    "ThinkingOptionsLoaded",
    "ThinkingLevelRefused",
    "search_files_in_session",
    "oauth_clear_tokens",
    "oauth_get_tokens",
    "oauth_headless_start",
    "oauth_headless_complete",
    "oauth_device_start",
    "oauth_device_poll",
    "oauth_copilot_device_start",
    "OAuthBrowserReady",
    "blocklist_disabled_rules",
    "blocklist_set_rule_enabled",
    "BlocklistDisabledRulesLoaded",
    // WT-018: a session's card changes and their discard moves.
    "session_card_changes",
    "move_session_cards_back",
];

pub fn frozen_prefix(tokens: &str) -> String {
    let mut lines: Vec<&str> = tokens.lines().collect();
    while lines.last().is_some_and(|line| {
        line.split_once(' ')
            .is_some_and(|(name, _)| APPROVED_APPENDED_TAILS.contains(&name))
    }) {
        lines.pop();
    }
    lines.join("\n")
}
