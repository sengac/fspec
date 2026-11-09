@RPC-445
@tui
@multi-workspace
Feature: Board header workspace dropdown
  """
  codelet-fspec-tui: FspecBackend gains list_workspaces, list_workspace_work_units,
  create_session_in_workspace and workspace_work_units_rx (defaults keep other backends
  compiling; embedded and WebSocket delegate to the tarpc client / service stream). BoardStore
  holds a WorkspaceSelector (store/board_workspaces.rs): the workspace list, the selected path and
  the dropdown highlight. Header row 1 (views/board/header.rs) paints "Workspace: <name> ▾" and the
  open dropdown paints beneath it (views/board/workspace_dropdown.rs). 'w' or a click on row 1
  opens it; Up/Down/Enter/Esc or a click on an entry drive it. Choosing a workspace fetches
  list_workspace_work_units; WorkUnitsLoaded from the start-workspace stream is dropped while
  another workspace is selected, and WorkspaceWorkUnitsLoaded is applied only for the selected
  path. AgentViewStore records each observed session's project and active_menu_session_ids hides
  sessions from other workspaces. Session creation uses create_session_in_workspace when a
  non-start workspace is selected.
  """

  Background: User Story
    As a developer running one fspec daemon across several projects
    I want a workspace selector in the board header
    So that I can switch the board and its sessions between projects

  Scenario: The header shows the start workspace by default
    Given the daemon lists workspaces "alpha" (start) and "beta"
    When the board renders
    Then header row 1 shows "Workspace: alpha"

  Scenario: Pressing w opens a dropdown of every workspace
    Given the daemon lists workspaces "alpha" (start) and "beta"
    When the user presses "w" on the board
    Then a dropdown under the header lists "alpha" and "beta"

  Scenario: Choosing a workspace re-scopes the board
    Given the board is showing alpha's work unit "ALPHA-001" and beta has work unit "BETA-001"
    When the user opens the workspace dropdown, moves down and presses Enter
    Then the board shows "BETA-001" and header row 1 shows "Workspace: beta"
    And a later work-unit push for alpha does not replace beta's board

  Scenario: Pushes for the selected workspace refresh the board
    Given the user has selected workspace "beta"
    When beta's work units are pushed with "BETA-002"
    Then the board shows "BETA-002"

  Scenario: Clicking the workspace row and an entry selects that workspace
    Given the daemon lists workspaces "alpha" (start) and "beta"
    When the user clicks header row 1 and then clicks the "beta" entry
    Then header row 1 shows "Workspace: beta"

  Scenario: Only the selected workspace's sessions are shown
    Given an open session in alpha and an open session in beta
    When the user selects workspace "beta"
    Then the menu bar shows only beta's session

  Scenario: New agents start in the selected workspace
    Given the user has selected workspace "beta"
    When the user creates a new agent from the board
    Then the session is created in beta's path
