@done
@diff-viewer
@tui
@RPC-367
Feature: Restore pane border/divider parity in Rust TUI Changed Files and Checkpoints views

  """
  Add shared border helpers to codelet/fspec-tui/src/views/diff_common/: a vertical-divider helper (reserves a 1-col gutter and paints '│' using default colour) and a heading-underline helper (paints a 1-row '─' rule below pane_header). Both views (changed_files/render.rs and checkpoints/render.rs) consume these helpers.
  Layout constraints must reserve space for dividers (1 column between horizontally-split panes; 1 row for the heading underline) so content rects are not overdrawn. Update cached last_*_rect values used for mouse-wheel hit-testing (pane_at) and page-step math to reflect the reduced content area.
  Tests use ratatui TestBackend: render each view into a fixed-size buffer, join cells to a string, and assert '│' appears at the column between panes and '─' appears on the row below each heading. Follow the existing buffer-to-string pattern in full_screen_shell.rs tests and changed_files/tests.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Checkpoints view draws a vertical divider between the Checkpoints list pane and the Files list pane
  #   2. The Changed Files view draws a vertical divider between the Files pane and the Diff pane
  #   3. Each pane draws a horizontal underline rule beneath its heading title
  #   4. Pane dividers and heading underlines use the default terminal colour and no borderColor is set, matching the TypeScript reference
  #   5. Existing behaviour is preserved: focus highlight, scrolling, scrollbar gutter, empty-state messages, and mouse-wheel hit-testing still work after adding dividers
  #   6. Divider and heading-rule rendering lives in the shared diff_common module so both views reuse identical logic (DRY)
  #
  # EXAMPLES:
  #   1. Rendering the Checkpoints view shows a vertical divider glyph between the Checkpoints column and the Files column in the top row
  #   2. Rendering the Changed Files view shows a vertical divider glyph between the Files column and the Diff column
  #   3. Rendering either view shows a horizontal underline rule directly below each pane heading row
  #   4. Rendering the Changed Files view with no changes still shows the empty-state message and is unaffected by divider rendering
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to see visible dividers between the panes in the Rust Changed Files and Checkpoints views, just like the old TypeScript board
    So that I can clearly distinguish the panes and the UI matches the original reference

  @integration
  Scenario: Changed Files view shows a vertical divider between the Files and Diff panes
    Given the Changed Files view has at least one changed file to display
    When the view is rendered to the terminal buffer
    Then a vertical divider glyph is drawn in the column between the Files pane and the Diff pane
    And the divider uses the default terminal colour with no explicit colour set

  @integration
  Scenario: Each pane shows a horizontal underline rule beneath its heading
    Given the Changed Files view has at least one changed file to display
    When the view is rendered to the terminal buffer
    Then a horizontal underline rule is drawn on the row directly below each pane heading

  @integration
  Scenario: Empty Changed Files view still shows its empty-state message
    Given the Changed Files view has no changed files
    When the view is rendered to the terminal buffer
    Then the empty-state message that there are no changed files is shown
    And no pane divider is drawn over the empty-state message
