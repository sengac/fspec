@done
@BUG-465
@session
@session-management
@rust
Feature: RPC lists and session controls report failures
  """
  Without a session manager (or start workspace) the RPC service answered
  loop_list, list_provider_credentials, list_session_worktrees,
  list_sessions, list_live_sessions, get_subordinates, changed_files,
  list_checkpoints, checkpoint_diff_files and search_files with an empty
  Vec, and interrupt, set_active_session and clear_active_session silently
  did nothing. The handle methods behind them hid failures too: loop_list,
  get_subordinates and interrupt treated an unknown session as empty or a
  no-op, and set_active_session ignored an invalid id (a valid id that is
  not loaded stays allowed: active-session tracking works without a session
  row, RPC-042); list_provider_credentials and
  list_sessions turned a read failure into an empty list;
  list_session_worktrees skipped every git error; and changed_files /
  list_checkpoints / checkpoint_diff_files used unwrap_or_default.
  Now every one returns Result (trait, sessions, stub, tarpc in its
  RPC-449 position, MCP and both TUI transports): no session manager or
  workspace is refused, an unknown session is the lookup error and read
  failures propagate; list_session_worktrees skips only project roots that
  are not git repositories.
  Tests: rust/fspec-tui/tests/bug465_rpc_lists_and_controls_report_failures.rs.
  """

  Background: User Story
    As a TUI, RPC or MCP client listing or controlling sessions
    I want to have a missing session manager or workspace, an unknown session or a failed read reported instead of an empty list or a silent no-op
    So that an empty list always means there is nothing to show

  Scenario: Session lists and controls report an unknown session
    Given a session manager with no session "00000000-0000-4000-8000-000000000465"
    When its loops and subordinates are listed and it is interrupted
    Then every call fails naming "00000000-0000-4000-8000-000000000465"

  Scenario: Lists and controls over RPC are refused without a session manager
    Given an RPC service with no session manager attached
    When the sessions, live sessions and provider credentials are listed and a session is interrupted over the embedded transport
    Then every call fails saying no session manager is attached

  Scenario: Workspace lists over RPC are refused without a workspace
    Given an RPC service with no workspace directory
    When the changed files, checkpoints and file search are requested over the embedded transport
    Then every call fails instead of returning an empty list

  Scenario: A failed session listing is reported
    Given a data directory whose sessions path is a file, not a directory
    When the sessions are listed through the session manager
    Then the listing fails instead of returning an empty list
