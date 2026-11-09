@BUG-582
@RPC-448
@tui
@multi-workspace
Feature: TUI daemon switcher
  """
  codelet-fspec-tui: a DaemonDirectory (list() -> Vec<DaemonChoice>, connect(choice) ->
  Arc<dyn FspecBackend>) is installed on the App by the binary (fspec/src/client.rs lists
  common::list_live_daemons + codelet_common::saved_daemons and connects with
  WebSocketFspecBackend::connect_with_bearer). 's' on the board opens the switcher dropdown
  (views/board/daemon_switcher.rs); choosing an entry connects in a pending task and dispatches
  Action::DaemonSwitched, whose handler (app/dispatch_daemons.rs) aborts the subscriber tasks,
  replaces App.backend and re-runs the bootstrap fetches. The old backend is dropped, never asked
  to destroy or interrupt sessions. Header row 1 shows "Daemon: <name>" beside the workspace.
  TUI-123 (daemon-management-on-par-with-workspaces): clicking that label or the Settings menu's
  "Daemons" entry also opens the switcher, and connect failures are named by kind.
  """

  Background: User Story
    As a developer connected to one of several fspec daemons
    I want to switch daemons from the board
    So that I can move between machines without restarting the client

  Scenario: The switcher lists local and saved daemons
    Given a daemon directory with local daemon "alpha" (connected) and saved daemon "work"
    When the user presses "s" on the board
    Then a dropdown lists "alpha" and "work" with "alpha" marked as connected

  Scenario: Choosing a daemon re-scopes the board to it
    Given the board is connected to "alpha" showing "ALPHA-001" and "work" has "WORK-001"
    When the user switches to "work"
    Then the board shows "WORK-001"
    And a later work-unit push from "alpha" does not reach the board

  Scenario: Switching leaves the previous daemon's sessions running
    Given the board is connected to "alpha" with an open session
    When the user switches to "work"
    Then no session on "alpha" was destroyed or interrupted

  Scenario: The header names the connected daemon
    Given the board is connected to "alpha"
    When the user switches to "work"
    Then header row 1 shows "Daemon: work"
