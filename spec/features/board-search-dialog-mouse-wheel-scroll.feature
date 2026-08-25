@done
@BUG-162
@tui
@board
@dialog
@search
@bug-162
Feature: Board search dialog match list scrolls with the mouse wheel
  """
  BUG-162: The BOARD-022 WorkUnitSearchDialog handle_event only matched
  Event::Key; Event::Mouse (wheel) events fell through as Ignored and the
  BoardView behind the modal scrolled its board column instead of the
  dialog's match list. The dialog now handles Event::Mouse: wheel events
  inside its last-rendered rect move the highlighted match via the shared
  WheelVelocity (1x-5x ramp) and ensure_visible, wheel events outside the
  rect are Ignored so they bubble to the board, and a proportional
  scrollbar gutter (render_list_scrollbar + ScrollbarDrag) is painted and
  hit-tested when the matches overflow the visible rows.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ScrollDown/ScrollUp mouse events INSIDE the dialog's
  #      last-rendered rect move the highlighted match by the WheelVelocity
  #      step (ramping 1x-5x within 150 ms) and update scroll_offset via
  #      ensure_visible; the event is Consumed
  #   2. Mouse events OUTSIDE the dialog's last-rendered rect are Ignored
  #      so they bubble to the BoardView behind the modal
  #   3. Left-button press/drag/release on the scrollbar gutter is routed
  #      through the shared ScrollbarDrag state machine with
  #      ScrollbarGeometry (absolute row converted to body-local row); the
  #      returned offset is applied to scroll_offset
  #   4. When matches exceed the visible rows, a proportional scrollbar
  #      (render_list_scrollbar) is painted in the rightmost body column
  #      and its gutter rect is cached for hit-testing; wheel/scrollbar-
  #      drag state is reset when the query or mode changes (re_filter)
  #
  # EXAMPLES:
  #   - Scrolling the mouse wheel down inside the search dialog moves the
  #     highlighted match down and the scroll offset follows it
  #   - A wheel event outside the dialog rect is ignored so it bubbles to
  #     the board behind the modal
  #
  # QUESTIONS:
  #   (none — the design is locked by the research note)
  # ========================================
  # SCENARIOS
  # ========================================
  # @BUG-162
  Scenario: Scrolling the mouse wheel down inside the dialog moves the selection down
    Given the work-unit search dialog is open with more matches than visible rows
    When I scroll the mouse wheel down inside the dialog
    Then the dialog consumes the event
    And the highlighted match moves down by the wheel step
    And the scroll offset follows the highlighted match

  Scenario: Scrolling the mouse wheel up inside the dialog moves the selection up
    Given the work-unit search dialog is open with the selection past the first visible row
    When I scroll the mouse wheel up inside the dialog
    Then the dialog consumes the event
    And the highlighted match moves up by the wheel step

  Scenario: Rapid wheel notches ramp the wheel velocity
    Given the work-unit search dialog is open with a long match list
    When I scroll the mouse wheel down five times in rapid succession
    Then the fifth step moves the selection by five rows

  Scenario: A wheel event outside the dialog rect is ignored so it bubbles to the board
    Given the work-unit search dialog is open with a match list
    When I scroll the mouse wheel down outside the dialog rect
    Then the dialog ignores the event
    And the selection and scroll offset are unchanged

  Scenario: Repeated wheel-down reaches the last match and keeps it on screen
    Given the work-unit search dialog is open with more matches than visible rows
    When I scroll the mouse wheel down until the last match is highlighted
    Then the last match is highlighted
    And the scroll offset keeps the last match inside the visible window

  Scenario: A proportional scrollbar gutter is painted when matches overflow the visible rows
    Given the work-unit search dialog is open with more matches than visible rows
    When the dialog is rendered
    Then a scrollbar gutter rect is cached for hit-testing
    And the gutter is painted in the rightmost body column
