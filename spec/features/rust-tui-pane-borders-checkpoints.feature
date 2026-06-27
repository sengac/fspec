@done
@RPC-367
@tui
@diff-viewer
Feature: Restore pane divider parity in the Rust TUI Checkpoints view

  """
  Add a shared vertical-divider helper to codelet/fspec-tui/src/views/diff_common/ (paints '│' in a reserved 1-col gutter using the default colour) and consume it from checkpoints/render.rs between the Checkpoints list pane and the Files list pane. The layout must reserve a 1-col gutter so content rects are not overdrawn, and the cached last_*_rect values used for mouse-wheel hit-testing must reflect the reduced content area. Tests use ratatui TestBackend: render the view into a fixed-size buffer, join cells to a string, and assert '│' appears at the column between the two top panes.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Checkpoints view draws a vertical divider between the Checkpoints list pane and the Files list pane
  #   4. Pane dividers use the default terminal colour and no borderColor is set, matching the TypeScript reference
  #   5. Existing behaviour is preserved: focus highlight, scrolling, scrollbar gutter, and mouse-wheel hit-testing still work after adding dividers
  #   6. Divider rendering lives in the shared diff_common module so both views reuse identical logic (DRY)
  #
  # EXAMPLES:
  #   1. Rendering the Checkpoints view shows a vertical divider glyph between the Checkpoints column and the Files column in the top row
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to see a visible divider between the Checkpoints and Files panes in the Rust Checkpoints view, just like the old TypeScript board
    So that I can clearly distinguish the panes and the UI matches the original reference

  @integration
  Scenario: Checkpoints view shows a vertical divider between the Checkpoints and Files panes
    Given the Checkpoints view has at least one checkpoint to display
    When the view is rendered to the terminal buffer
    Then a vertical divider glyph is drawn in the column between the Checkpoints pane and the Files pane
    And the divider uses the default terminal colour with no explicit colour set
