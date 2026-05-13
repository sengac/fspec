@done
@RPC-014
@rust
@tui
@board-view
@kanban
@layout
@responsive
@unit
Feature: RPC-014 BoardView grid pure-function helpers

  """
  RPC-014 (slice 2 of 3) — Pure-function helpers shared by the BoardView
  orchestrator and the source-shape regressions.

  The functions live in codelet/fspec-tui/src/views/board/grid.rs and are
  exercised directly without any rendering — they have no `ratatui::Buffer`
  dependency. Their behaviour is a literal port of the
  `calculateColumnWidths` / `getColumnWidth` / `buildBorderRow` helpers from
  `src/tui/components/UnifiedBoardLayout.tsx`.

  Pair: tests live in codelet/fspec-tui/tests/grid_unit_rpc014.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want grid helpers (calculate_column_widths, column_width_at, build_border_row) that match the TS UnifiedBoardLayout math 1:1
    So that the box-drawing characters line up across separator rows and column content rows for any terminal width

  Scenario: 120-wide terminal divides into seven equal 16-wide columns
    Given terminal_width = 120
    When the test calls calculate_column_widths(120)
    Then the returned base_width equals 16
    And the returned remainder equals 0
    And column_width_at(idx, ...) returns 16 for every idx in 0..7
    And the seven column widths plus the six separator chars sum to 118 (= 120 - 2 outer borders)

  Scenario: 125-wide terminal spreads a 5-wide remainder across the leading columns
    Given terminal_width = 125
    When the test calls calculate_column_widths(125)
    Then the returned base_width equals 16
    And the returned remainder equals 5
    And column_width_at(0, ...) through column_width_at(4, ...) each return 17
    And column_width_at(5, ...) and column_width_at(6, ...) each return 16
    And the seven column widths plus the six separator chars sum to 123 (= 125 - 2 outer borders)

  Scenario: Narrow terminals clamp to the 8-column minimum and report zero remainder
    Given terminal_width = 60
    When the test calls calculate_column_widths(60)
    Then the returned base_width is at least 8
    And the returned remainder equals 0

  Scenario: build_border_row produces the four canonical separator strings for a 120-wide layout
    Given the widths produced by calculate_column_widths(120)
    When the test calls build_border_row(widths, "├", "┤", Plain)
    Then the returned string starts with "├" and ends with "┤" and has length 120
    When the test calls build_border_row(widths, "├", "┤", Top)
    Then the returned string starts with "├" and ends with "┤" and contains exactly six "┬" glyphs
    When the test calls build_border_row(widths, "├", "┤", Cross)
    Then the returned string starts with "├" and ends with "┤" and contains exactly six "┼" glyphs
    When the test calls build_border_row(widths, "├", "┤", Bottom)
    Then the returned string starts with "├" and ends with "┤" and contains exactly six "┴" glyphs
