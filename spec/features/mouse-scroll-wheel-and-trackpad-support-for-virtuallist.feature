@done
@virtuallist
@mouse-events
@interaction
@tui
@high
@TUI-001
Feature: Mouse scroll wheel and trackpad support for VirtualList
  """
  Uses Ink's useInput hook to detect mouse scroll events via key.mouse.button === 'none'. Implements throttling using lodash.throttle with 100ms interval. Integrates with existing VirtualList component without modifying core scrolling logic. Scroll events handled at parent component level and passed to VirtualList via props.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mouse scroll events detected via useInput hook checking key.mouse.button === 'none' for scroll actions
  #   2. Scroll events MUST be throttled to prevent performance degradation from high-frequency events
  #   3. All VirtualList instances in the application MUST support mouse scrolling consistently
  #   4. Terminal must have mouse tracking enabled for scroll events to work
  #   5. Traditional scrolling behavior: scroll wheel down moves viewport down (shows items below), scroll wheel up moves viewport up (shows items above)
  #   6. Use 100ms throttle interval for scroll events
  #   7. Scroll events only handled when pane containing VirtualList has focus
  #   8. Vertical scrolling only - no horizontal scrolling support needed
  #
  # EXAMPLES:
  #   1. User scrolls down with mouse wheel in Kanban board file list → selected item moves down one item
  #   2. User scrolls up with mouse wheel in CheckpointViewer file list → selected item moves up one item
  #   3. User scrolls with trackpad (two-finger gesture) on Mac → behaves identically to mouse wheel scroll
  #   4. User rapidly scrolls mouse wheel → throttled events prevent UI lag and maintain smooth navigation
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should scroll wheel down move selection down (natural scrolling) or scroll the viewport down showing items above (traditional scrolling)?
  #   A: true
  #
  #   Q: What throttle interval should be used for scroll events? (e.g., 50ms, 100ms, 150ms)
  #   A: true
  #
  #   Q: Should scrolling work when ANY pane is active, or only when specific panes (like file list) have focus?
  #   A: true
  #
  #   Q: Should holding Shift+ScrollWheel enable horizontal scrolling, or is vertical-only sufficient?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to scroll through lists using mouse wheel or trackpad
    So that I can navigate more efficiently without relying on keyboard arrow keys

  Scenario: Scroll down with mouse wheel in file list
    Given a VirtualList component is rendered with multiple items
    And the file list pane has focus
    When the user scrolls mouse wheel down
    Then the viewport should scroll down showing items below
    And the traditional scrolling behavior should be used

  Scenario: Scroll up with mouse wheel in file list
    Given a VirtualList component is rendered with multiple items
    And the file list pane has focus
    When the user scrolls mouse wheel up
    Then the viewport should scroll up showing items above
    And the traditional scrolling behavior should be used

  Scenario: Scroll with trackpad two-finger gesture
    Given a VirtualList component is rendered on macOS
    And the file list pane has focus
    When the user performs a two-finger trackpad scroll gesture
    Then the scrolling should behave identically to mouse wheel scroll
    And the viewport should move smoothly

  Scenario: Rapid scroll with throttling
    Given a VirtualList component is rendered with many items
    And the file list pane has focus
    When the user rapidly scrolls the mouse wheel multiple times
    Then scroll events should be throttled at 100ms intervals
    And the UI should remain responsive without lag
    And navigation should feel smooth and performant
