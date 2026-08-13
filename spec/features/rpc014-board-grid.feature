@done
@RPC-014
@rust
@tui
@ui
@rpc
@ui-enhancement
@board-view
@kanban
Feature: RPC-014 BoardView rich box-drawing grid + work-unit details strip
  """
  RPC-014 (slice 1 of 3) — BoardView is upgraded from the placeholder single-Block
  skeleton to the full UnifiedBoardLayout grid topology:

  - box-drawing column separators (├ ┼ ┤ ┬ ┴) computed once per frame from
  the terminal width;
  - a 5-row work-unit details strip mirroring the TS components
  WorkUnitTitle / WorkUnitDescription / WorkUnitAttachments /
  WorkUnitMetadata;
  - focused-column highlighting and work-type color coding inside the
  column cells.

  All RPC-012 actions and the RPC-013 literal footer remain intact. No
  TypeScript code is modified.

  Pair: render tests live in rust/fspec-tui/tests/view_board_unit_rpc014.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want to see a rich box-drawing Kanban grid with a work-unit details strip in the Rust BoardView
    So that the Rust ratatui port matches the visual fidelity of the TypeScript Ink UnifiedBoardLayout 1:1

  Scenario: No work unit selected paints the centered placeholder
    Given an empty BoardStore (no work units, no selection)
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "No work unit selected"

  Scenario: Selected work unit paints title and metadata rows of the details strip
    Given a BoardStore containing AUTH-001 (story, backlog, title "User Login", description "Sign in with email/password", estimate 5, epic "authentication", no attachments)
    And the focused column is "backlog" and the selected index is 0
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "AUTH-001: User Login"
    And the rendered buffer contains the substring "Epic: authentication"
    And the rendered buffer contains the substring "Estimate: 5pts"
    And the rendered buffer contains the substring "Status: backlog"

  Scenario: Attachments row renders comma-joined basenames with the "A" key hint
    Given a BoardStore containing DOC-014 with attachments ["spec/attachments/RPC-014/notes.md", "spec/attachments/RPC-014/ref.md"]
    And the focused column matches DOC-014's column and DOC-014 is selected
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring 'Attachments (use the "A" key to view): notes.md, ref.md'

  Scenario: Box-drawing borders and inner junctions are painted
    Given a BoardStore containing AUTH-001 in backlog
    When the App renders BoardView against a 120x24 TestBackend
    Then row 0 of the rendered buffer starts with "┌" and ends with "┐"
    And the last in-bounds row of the box starts with "└" and ends with "┘"
    And at least one inner row contains the glyph "├" and the glyph "┬" and the glyph "┤"
    And at least one inner row contains the glyph "├" and the glyph "┼" and the glyph "┤"
    And at least one inner row contains the glyph "├" and the glyph "┴" and the glyph "┤"

  Scenario: Focused column header is cyan+bold and other columns are dim
    Given a BoardStore containing AUTH-001 in backlog with the BACKLOG column focused
    When the App renders BoardView against a 120x24 TestBackend
    Then the column header row contains the substring "BACKLOG"
    And the cell holding "BACKLOG" is styled with foreground Cyan and the bold modifier
    And the column header row contains the substring "SPECIFYING"
    And the cell holding "SPECIFYING" is styled with the theme.dim foreground (DarkGray)

  Scenario: Bug cells render red and the focused selected cell flips to bg=green fg=black bold
    Given a BoardStore containing BUG-001 (bug) and BUG-002 (bug) in the backlog column
    And the focused column is "backlog" and BUG-001 is selected
    When the App renders BoardView against a 120x24 TestBackend
    Then the cell containing "BUG-001" is styled with background Green, foreground Black and the bold modifier
    And the cell containing "BUG-002" is styled with foreground Red

  Scenario: Task cells render blue with the [estimate] suffix
    Given a BoardStore containing TASK-001 (task, estimate 3) in the implementing column
    And the focused column is "backlog" (so TASK-001 is NOT the selected cell)
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "TASK-001 [3]"
    And the cell containing "TASK-001 [3]" is styled with foreground Blue

  Scenario: Footer string and footer separator are still painted at the bottom
    Given a BoardStore containing AUTH-001 in backlog
    When the App renders BoardView against a 120x24 TestBackend
    Then the last in-bounds inner row contains the substring "← → Columns"
    And the same row contains the substring "↵ Work Agent"
    And the row immediately above the footer contains the glyph "┴"
