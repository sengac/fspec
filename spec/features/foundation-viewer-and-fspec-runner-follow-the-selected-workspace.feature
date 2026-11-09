@BUG-582
@done
@TUI-120
@tui
@multi-workspace
@rust
Feature: Foundation viewer and agent-view fspec runner follow the selected workspace
  """
  The board D key opened FOUNDATION.md from a viewer bound to the TUI
  process's current directory, which with a remote daemon is not even on
  the daemon's machine, and the agent-view fspec runner answered
  list-work-units and show-work-unit from the start workspace.
  Now OpenFoundation reads spec/FOUNDATION.md of the board's selected
  workspace from the daemon (read_card_attachment) through the BUG-431
  fetch/cache/serve path (App::spawn_workspace_file_open) and shows any
  failure; the fspec runner lists the selected workspace's work units
  (list_workspace_work_units). Supersedes the RPC-373 client-port URL
  scenarios.
  Tests: rust/fspec-tui/tests/tui120_foundation_viewer_and_fspec_runner_follow_the_selected_workspace.rs.
  """

  Background: User Story
    As a user whose board shows a workspace other than the TUI's directory
    I want to open that workspace's FOUNDATION.md and have agent fspec commands read its work units
    So that the foundation and work units always come from the workspace I am looking at, even on a remote daemon

  Scenario: Pressing D opens the selected workspace's foundation from the daemon
    Given the board shows workspace "beta"
    When the user presses D
    Then "spec/FOUNDATION.md" of "beta" is read from the daemon
    And the viewer opens it

  Scenario: Pressing D before the workspace list loads shows why nothing opened
    Given the workspace list has not loaded
    When the user presses D
    Then nothing is read from the daemon
    And an error explains that no workspace is selected

  Scenario: The agent fspec runner reads the selected workspace's work units
    Given the board shows workspace "beta" holding work unit "BETA-001"
    When a session asks the fspec runner to show work unit "BETA-001"
    Then the runner answers with "BETA-001" from "beta"
