@done
@RPC-452
@rpc
@multi-workspace
@rust
Feature: Changed Files view and file diffs follow the selected workspace
  """
  The Changed Files view (f) and the mux Files pane always showed the
  daemon's start workspace: changed_files() / file_diff(path) take no
  workspace and use the service cwd, and a failed listing showed an empty
  list. New tarpc methods changed_files_in_workspace(workspace) and
  file_diff_in_workspace(workspace, path), appended to FspecService
  (RPC-449), open the named workspace on the daemon and read its git state;
  errors propagate (the old file_diff swallowed them). The TUI passes the
  board's selected workspace and shows a failed listing as an error.
  Tests: rust/fspec-tui/tests/rpc452_changed_files_follow_the_selected_workspace.rs.
  """

  Background: User Story
    As a user switching the board between workspaces
    I want to see the selected workspace's changed files and diffs
    So that the Changed Files view never shows another repository's changes

  Scenario: Changed files over RPC come from the named workspace
    Given a daemon started in git repository "alpha" and a second git repository "beta" with a modified file "beta.txt"
    When the changed files of workspace "beta" are requested over RPC
    Then the listing contains "beta.txt" and no file of "alpha"

  Scenario: A file diff over RPC comes from the named workspace
    Given a daemon started in git repository "alpha" and a second git repository "beta" with a modified file "beta.txt"
    When the diff of "beta.txt" in workspace "beta" is requested over RPC
    Then the diff shows the change made in "beta"

  Scenario: The Changed Files view requests the selected workspace
    Given the board has workspace "beta" selected
    When the Changed Files view opens
    Then the backend is asked for the changed files of workspace "beta"

  Scenario: A failed changed-files listing is shown
    Given a backend whose changed-files listing fails with "not a git repository"
    When the Changed Files view opens
    Then an error dialog shows "not a git repository"
