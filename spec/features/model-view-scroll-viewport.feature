@done
@bug
@ts-parity
@scroll
@model-selector
@tui
@PROV-104
Feature: Model view scroll/viewport parity with TypeScript

  """
  The full-screen ModelSelectorView (codelet/fspec-tui/src/views/model_selector/) renders a windowed flat row list (provider headers + model rows). Scroll state lives in mod.rs (selected_index, scroll_offset, visible_rows); adjust_scroll reuses components::scroll_viewport::ensure_visible. FIX (TS parity with ModelSelectorView.tsx): rows::render_body slices the FULL visible window so every visible slot paints content, and draws the up/down/scrollbar indicator in a dedicated column beside the list rather than overwriting the first/last content row, eliminating the inline-arrow row-stealing that hid the selected row at viewport edges. PageDown/PageUp added in handle_key, moving selection by one viewport height across selectable rows. Tests render to a ratatui TestBackend and assert the selected row text/marker is actually PAINTED at top/bottom/mid edges.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The scroll indicator (up/down/scrollbar) renders in a dedicated column beside the list and never overwrites a content row
  #   2. The body slices the full visible window so every visible row paints content; the selected row is always painted within the viewport
  #   3. Navigating to the bottom edge of a list longer than the viewport keeps the selected row painted on the last visible content row
  #   4. Navigating to the top edge keeps the selected row painted on the first visible content row
  #   5. PageDown and PageUp move the selection by one viewport height and keep the selected row painted within the viewport
  #
  # EXAMPLES:
  #   1. With a 30-model list in a 10-row viewport, pressing Down past the bottom paints the selected model row as the last visible content row
  #   2. After scrolling down then pressing Up back to the first model, the selected model row is painted on the first content row and offset returns to 0
  #   3. Pressing End on a tall list paints the last model row at the bottom edge with no inline arrow overwriting it
  #   4. A mid-list selection is painted within the viewport and not stolen by an overflow indicator
  #   5. Pressing PageDown advances the selection by one viewport height and the new selected row is painted within the viewport
  #   6. When the list overflows the viewport, a scrollbar column is painted beside the list and the rightmost content column still shows model text
  #
  # ========================================

  Background: User Story
    As a model selector user
    I want to scroll through a long model list
    So that the highlighted model row always stays visible and follows my cursor

  Scenario: Down past the bottom paints the selected row on the last content row
    Given a model list of 30 models in a viewport 10 content rows tall
    When I press Down until the selection would fall below the visible window
    Then the selected model row is painted within the visible viewport
    And the selected model row is painted on the last visible content row
    And no overflow indicator overwrites the selected row

  Scenario: Returning to the first model paints it on the first content row
    Given a model list of 30 models in a viewport 10 content rows tall
    And the viewport has been scrolled down away from the top
    When I press Up until the cursor reaches the first model
    Then the selected model row is painted within the visible viewport
    And the leading provider header is painted above the selected model row
    And the scroll offset returns to 0

  Scenario: End jumps to the last model and paints it at the bottom edge
    Given a model list of 30 models in a viewport 10 content rows tall
    When I press End
    Then the selected model row is the last model row
    And the selected model row is painted within the visible viewport
    And no inline arrow overwrites the last content row

  Scenario: A mid-list selection is painted within the viewport
    Given a model list of 30 models in a viewport 10 content rows tall
    When I move the selection to a model in the middle of the list
    Then the selected model row is painted within the visible viewport
    And the selected model row is not stolen by an overflow indicator

  Scenario: PageDown advances by one viewport height and keeps the selection painted
    Given a model list of 30 models in a viewport 10 content rows tall
    When I press PageDown
    Then the selection advances by approximately one viewport height
    And the selected model row is painted within the visible viewport

  Scenario: An overflowing list paints a scrollbar column beside the content
    Given a model list of 30 models in a viewport 10 content rows tall
    When the body is rendered
    Then a scrollbar column is painted beside the list
    And the rightmost content column still shows model text
