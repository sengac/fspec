@BUG-618
@done
@RPC-455
@rpc
@multi-workspace
@rust
Feature: File search, input history and session lists follow the selected workspace
  """
  The @file search and input history were answered from the daemon's start
  workspace (self.inner.cwd()), the /resume and attach session lists used
  the TUI process's current directory and model sync listed every project's
  sessions. Appended RPCs (RPC-449) search_files_in_workspace and
  persistence_{add,get,search}_history_in_workspace resolve the named
  workspace with workspace_cwd. The TUI sends the board's selected
  workspace (App::selected_workspace_path; the legacy calls only before the
  workspace list loads), lists sessions for that workspace, and model sync
  looks the session up in its own recorded project. BUG-618 (product owner,
  2026-11-07): the TUI's @file search no longer follows the selected
  workspace; it searches the session's own working directory
  (search_files_in_session). The search_files_in_workspace RPC stays.
  Tests: rust/fspec-tui/tests/rpc455_file_search_history_and_session_lists_follow_the_selected_workspace.rs.
  """

  Background: User Story
    As a user working in a workspace other than the daemon's start workspace
    I want to have @file search, input history, the session lists and model sync use that workspace
    So that suggestions, history and sessions never come from another repository

  Scenario: File search over RPC searches the named workspace
    Given a daemon started in workspace "alpha" and a second workspace "beta" holding "src/beta_only.rs"
    When files starting with "src/beta" are searched in "beta" over RPC
    Then "src/beta_only.rs" is found

  Scenario: Input history over RPC is kept per named workspace
    Given a daemon started in workspace "alpha" and a second workspace "beta"
    When the input "beta command" is added to the history of "beta" over RPC
    Then the history of "beta" contains "beta command"
    And the start workspace's history does not

  Scenario: The TUI sends input history and session lists to the selected workspace
    Given the board shows workspace "beta"
    When the user recalls history and opens the session list
    Then each request names workspace "beta"

  Scenario: Model sync looks the session up in its own project
    Given a session recorded in project "/work/beta"
    When the session's model selection is synchronised
    Then the sessions of "/work/beta" are listed
