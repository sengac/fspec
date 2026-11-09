@done
@TUI-122
@tui
@multi-workspace
@rust
Feature: Add and remove workspaces from the board workspace dropdown
  """
  The board workspace dropdown was pick-only. While it is open, `a` opens
  an add-workspace dialog (components/add_workspace_dialog.rs, a single-line
  text input) and `d` removes the highlighted workspace; the dropdown's
  bottom border shows the keys. Both go through the RPC-457 daemon methods
  (FspecBackend::add_workspace / remove_workspace). A refused path reopens
  the dialog with the path and the daemon's reason; the start workspace and
  a workspace with open sessions are refused before asking the daemon.
  Tests: rust/fspec-tui/tests/tui122_add_and_remove_workspaces_from_the_board_dropdown.rs.
  """

  Background: User Story
    As a user switching between fspec projects on the board
    I want to add a workspace by path and remove one from the workspace dropdown
    So that I manage the workspace list where I use it, without the CLI

  Scenario: Adding a workspace from the dropdown lists and shows it
    Given the board lists workspace "alpha" and its dropdown is open
    When the user presses "a", types "/work/beta" and presses Enter
    Then the daemon is asked to add "/work/beta"
    And the dropdown lists "beta" and the board shows it

  Scenario: A refused path reopens the dialog with the reason
    Given the daemon refuses "/tmp/plain" as not an fspec project
    When the user adds "/tmp/plain" from the dropdown
    Then the add-workspace dialog is open again with "/tmp/plain"
    And it shows the daemon's reason

  Scenario: Removing the highlighted workspace from the dropdown
    Given the board shows workspace "beta" and the dropdown highlights it
    When the user presses "d"
    Then the daemon is asked to remove "/work/beta"
    And the board shows the start workspace "alpha"

  Scenario: The start workspace and a workspace with open sessions are not removed
    Given the dropdown highlights the start workspace, or a workspace with an open session
    When the user presses "d"
    Then the daemon is not asked to remove it
    And an error explains why
