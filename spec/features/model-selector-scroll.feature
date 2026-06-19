@done
@rust
@scroll
@model-selector
@tui
@RPC-340
Feature: Model selector list does not scroll to follow the cursor

  """
  Scroll state lives in ModelSelectorView (codelet/fspec-tui/src/views/model_selector/mod.rs): scroll_offset + visible_rows fields, windowed by rows::render_body. Fix reuses the existing components::scroll_viewport::ensure_visible(scroll_offset, selected, visible_rows, total) helper (scroll_viewport.rs:46-66), wrapped in a private adjust_scroll() mirroring provider_settings::adjust_scroll. visible_rows = body_area.height - 1 (legend row). adjust_scroll is invoked from move_up/move_down (covers keyboard + mouse-wheel), Home/End, set_providers, handle_filter_key, toggle_expansion, and once at render after visible_rows is known (defensive resize reconcile). No wire-type or Action changes; scrollbar/overflow indicators in rows.rs follow automatically once scroll_offset tracks the cursor.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When the selected row moves below the visible window, the viewport MUST scroll down so the selected row becomes the last visible row (offset = selected + 1 - visible_rows)
  #   2. When the selected row moves above the visible window, the viewport MUST scroll up so the selected row becomes the first visible row (offset = selected)
  #   3. The scroll offset MUST be clamped to a maximum of total_rows - visible_rows so the list never scrolls past the end leaving blank trailing rows
  #   4. When visible_rows is 0 or total_rows is 0 (tiny or empty viewport), the scroll offset MUST reset to 0 and the body MUST render without panic
  #   5. Scroll reconciliation MUST apply to BOTH keyboard navigation (arrows, Home, End) and mouse-wheel navigation, since both move the selection
  #   6. When the row list is rebuilt (provider load/refresh via set_providers, filter change, expand/collapse), the scroll offset MUST be reconciled so the current selected row stays visible
  #   7. The scrollbar thumb position and the up/down overflow indicators MUST reflect the live scroll offset (follow the cursor), not a frozen offset of 0
  #   8. Visible row count used for scroll math MUST be the body height minus the legend row (body_area.height - 1), NOT the outer-chrome CHROME_ROWS value
  #   9. Keep the selection minimally in view via ensure_visible (do NOT hard-reset to 0). This future-proofs RPC-341, which will seed the cursor onto the current model that may be deep in the list. Because set_providers currently resets selection to first_selectable_or_zero (near top), ensure_visible naturally yields ~0 today.
  #   10. Yes, add a defensive render-time reconcile after the true body height is known (right after self.visible_rows is assigned). ensure_visible is idempotent so it is harmless, and it covers window-resize and initial-draw cases where the viewport height was not yet known during navigation.
  #
  # EXAMPLES:
  #   1. Viewport shows 10 rows, list has 30 rows, cursor at row 0; user presses Down 12 times -> cursor on row 12, viewport scrolled so row 12 is the last visible row (offset 3)
  #   2. Cursor is on the last visible row near the bottom of a 30-row list; user presses Up repeatedly back to the top -> viewport scrolls up with the cursor and the scroll offset returns to 0
  #   3. User presses End on a list taller than the viewport -> cursor jumps to the last selectable row and the viewport scrolls so that row sits at the bottom edge with no blank rows after it
  #   4. User scrolls the mouse-wheel down on an overflowing list -> the selection advances (skipping headers) and the viewport scrolls to keep the new selection visible, identical to pressing Down
  #   5. User has scrolled down a filtered/long list, then types into the filter narrowing results to a few rows -> the viewport resets so the (reset) selection is visible and there are no blank trailing rows
  #   6. Viewport is only 3 rows tall (or the list is empty) -> the body renders gracefully with the scroll offset at 0 and no panic
  #   7. User has the cursor near the bottom of a tall list, then resizes the terminal smaller (fewer body rows) -> on the next paint the viewport re-clamps so the selected row is still visible and there are no blank trailing rows
  #
  # QUESTIONS (ANSWERED):
  #   Q: On provider load/refresh (set_providers), should the viewport keep the current selection minimally in view (ensure_visible), or hard-reset the scroll to the top (offset 0)?
  #   A: Keep the selection minimally in view via ensure_visible (do NOT hard-reset to 0). This future-proofs RPC-341, which will seed the cursor onto the current model that may be deep in the list. Because set_providers currently resets selection to first_selectable_or_zero (near top), ensure_visible naturally yields ~0 today.
  #
  #   Q: Should we also add a defensive render-time scroll reconcile (after the real body height is known) so a window-resize that shrinks the viewport re-clamps the offset on the next paint?
  #   A: Yes, add a defensive render-time reconcile after the true body height is known (right after self.visible_rows is assigned). ensure_visible is idempotent so it is harmless, and it covers window-resize and initial-draw cases where the viewport height was not yet known during navigation.
  #
  # ========================================

  Background: User Story
    As a user browsing the full-screen model selector
    I want to have the list scroll to keep my highlighted model visible as I navigate
    So that I can see and reach every model even when the list is longer than the screen

  Scenario: Navigating down past the bottom scrolls the viewport to follow the cursor
    Given the model selector shows a body viewport 10 rows tall
    And the list is much longer than the viewport with the cursor at the top
    When I press Down until the selected row would fall below the visible window
    Then the viewport scrolls down so the selected row becomes the last visible row
    And the selected row stays inside the visible window

  Scenario: Navigating back up scrolls the viewport up with the cursor
    Given the model selector shows a body viewport 10 rows tall
    And the cursor has been moved down so the viewport is scrolled away from the top
    When I press Up until the cursor reaches the first row
    Then the viewport scrolls up with the cursor
    And the scroll offset returns to 0

  Scenario: End jumps to the last row and pins it to the bottom edge
    Given the model selector shows a body viewport 10 rows tall
    And the list is taller than the viewport
    When I press End
    Then the cursor is on the last selectable row
    And the scroll offset equals total rows minus visible rows
    And there are no blank rows rendered after the last row

  Scenario: Mouse-wheel navigation scrolls the viewport like the Down key
    Given the model selector shows a body viewport 10 rows tall
    And the list overflows the viewport with the cursor on the last visible row
    When I scroll the mouse-wheel down
    Then the selection advances to the next selectable row skipping headers
    And the viewport scrolls to keep the new selection visible

  Scenario: Filtering rebuilds the list and reconciles the scroll offset
    Given the model selector has been scrolled down a long list
    When I type a filter that narrows the results to a few rows
    Then the scroll offset is reconciled so the reset selection is visible
    And there are no blank trailing rows rendered

  Scenario: A tiny or empty viewport renders gracefully without panic
    Given the model selector body viewport is only 3 rows tall or the list is empty
    When the body is rendered
    Then the scroll offset is 0
    And the body renders without panic

  Scenario: Shrinking the terminal re-clamps the scroll offset on the next paint
    Given the model selector cursor is near the bottom of a tall list
    When the terminal is resized smaller so the body has fewer rows
    Then on the next paint the scroll offset is re-clamped
    And the selected row is still visible
    And there are no blank trailing rows rendered
