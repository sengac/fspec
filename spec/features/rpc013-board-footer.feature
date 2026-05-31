@done
@RPC-013
@rust
@tui
@ui
@rpc
@ui-enhancement
@board-view
Feature: RPC-013 BoardView footer — literal port of TS UnifiedBoardLayout footer string
  """
  RPC-013 (slice 1 of 3) — BoardView paints its own 1-row footer at the
  bottom of its render area. The string is the literal port of
  src/tui/components/UnifiedBoardLayout.tsx:504-511.

  Pair: tests live in codelet/fspec-tui/tests/view_board_unit_rpc013.rs.
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want BoardView to render the canonical UnifiedBoardLayout footer string at the bottom of its area
    So that the Rust BoardView matches the TS Ink BoardView's footer 1:1 and the legacy generic hint no longer leaks into the board

  Scenario: BoardView renders the literal TS UnifiedBoardLayout footer string
    Given an App with bootstrap complete and Navigator.active_view = ViewMode::Board
    And BoardStore seeded with [AUTH-001 backlog]
    When the App renders against a 120x24 TestBackend
    Then the rendered buffer contains the substring "← → Columns"
    And the rendered buffer contains the substring "↑↓ Work Units"
    And the rendered buffer contains the substring "[ Priority Up"
    And the rendered buffer contains the substring "] Priority Down"
    And the rendered buffer contains the substring "↵ Work Agent"
    And the rendered buffer contains the substring "ESC Back"

  Scenario: BoardView footer omits the legacy `? help q quit Tab switch pane` hint
    Given an App with bootstrap complete and Navigator.active_view = ViewMode::Board
    When the App renders against a 120x24 TestBackend
    Then the rendered buffer does NOT contain the substring "? help"
    And the rendered buffer does NOT contain the substring "switch pane"
    And the rendered buffer does NOT contain the substring "Tab "

  Scenario: BoardView paints headers above the footer in its own area
    Given a BoardView rendered against a 120x24 TestBackend with [AUTH-001 backlog]
    When a developer scans the rendered buffer row by row
    Then row 22 (the last in-bounds row of the box) contains the footer string substring "← → Columns"
    And at least one row above row 22 contains "BACKLOG"
    And the work-unit id "AUTH-001" appears on a row strictly above the footer row
