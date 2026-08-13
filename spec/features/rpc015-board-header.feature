@done
@RPC-015
@rust
@tui
@ui
@rpc
@ui-enhancement
@board-view
@header
@kanban
Feature: RPC-015 BoardView header — FSPEC Logo + CheckpointStatus + KeybindingShortcuts
  """
  RPC-015 (slice 2 of 3) — BoardView gains a 4-row header strip with separator
  inserted between the top box-drawing border and the existing 5-row
  work-unit details strip.

  - Left of the header (12 chars wide): the FSPEC ASCII art logo, rendered
  glyph-for-glyph identical to src/tui/components/Logo.tsx.
  - Right of the header (row 0): blank.
  - Right of the header (row 1): `Checkpoints: None` when both counts are 0,
  otherwise `Checkpoints: {manual} Manual, {auto} Auto`.
  - Right of the header (row 2): a `─` divider line matching TS borderTop.
  - Right of the header (row 3): `C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ / New Agent`.

  The C / F / D / / keybindings are HINT-ONLY in this card — wiring lands in
  subsequent RPC-002 children. BoardStore gains a `checkpoint_counts` field
  populated by App::bootstrap firing off backend.checkpoint_counts() and
  dispatching Action::CheckpointCountsLoaded.

  No TypeScript code is modified.

  Pair: render tests live in rust/fspec-tui/tests/view_board_unit_rpc015.rs;
  bootstrap-flow tests live in rust/fspec-tui/tests/app_bootstrap_rpc015.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a rich 4-row header strip with the FSPEC logo, live checkpoint counts, and top-level keybinding hints
    So that the Rust ratatui BoardView matches the TS Ink UnifiedBoardLayout header pixel-for-pixel

  Scenario: Empty BoardStore paints the FSPEC logo and Checkpoints: None
    Given an empty BoardStore (no work units, default checkpoint_counts = 0/0)
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "┏┓┏┓┏┓┏┓┏┓"
    And the rendered buffer contains the substring "Checkpoints: None"

  Scenario: Non-zero checkpoint counts paint the Manual/Auto breakdown
    Given a BoardStore whose checkpoint_counts has been set to { manual: 2, auto: 5 }
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "Checkpoints: 2 Manual, 5 Auto"

  Scenario: KeybindingShortcuts chord row is painted in the header
    Given a BoardStore with any selection state
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "C Checkpoints"
    And the rendered buffer contains the substring "F Changed Files"
    And the rendered buffer contains the substring "D FOUNDATION.md"
    And the rendered buffer contains the substring "/ New Agent"

  Scenario: New ├──┤ separator sits between the header strip and the details strip
    Given a BoardStore containing AUTH-001 in backlog
    When the App renders BoardView against a 120x24 TestBackend
    Then one of the inner rows contains the glyph "├" and the glyph "┤" with NO inner "┬" or "┼" or "┴" junctions on that same row
    And the four existing details/columns/footer separator rows (├┬┤ / ├┼┤ / ├┴┤) are still painted exactly as before

  Scenario: RPC-014 details strip and RPC-013 footer are still painted after RPC-015 inserts its header
    Given a BoardStore containing AUTH-001 (story, backlog, title "User Login", description "Sign in with email/password", estimate 5, epic "authentication", no attachments)
    And the focused column is "backlog" and the selected index is 0
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring "AUTH-001: User Login"
    And the rendered buffer contains the substring "Epic: authentication"
    And the rendered buffer contains the substring "Estimate: 5pts"
    And the rendered buffer contains the substring "Status: backlog"
    And the rendered buffer contains the substring "← →"
    And the rendered buffer contains the substring "Work Agent"

  Scenario: KeybindingShortcuts are visible hints only — no Action wiring lands in this card
    Given the App has rendered BoardView with the new header strip painted
    When the user presses the key 'C'
    Then NO new Action variant is emitted that opens a checkpoint viewer
    And NO new Action variant is emitted that opens the FOUNDATION.md viewer
    And NO new Action variant is emitted that opens the changed-files viewer
    And BoardView continues to emit existing Action variants on existing key events (← / → / ↑ / ↓ / Enter / [ / ] / Shift+Right / ESC)
