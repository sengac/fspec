@bug
@trackpad
@zoom
@diagram-viewer
@viewer
@critical
@VIEW-002
Feature: Mermaid diagram zoom not centered on mouse and trackpad causes jittering
  """
  Root cause analysis: fspec implementation uses panzoom.zoomToPoint() method which doesn't correctly implement zoom-to-cursor behavior. Mindstrike manually calculates viewport offset using formula: newX = viewport.x + (mouseX - viewport.x) * (1 - zoomRatio). This keeps the point under the cursor fixed during zoom. Current implementation passes clientX/clientY to panzoom but library's zoomToPoint doesn't work as expected.
  Trackpad jitter cause: Trackpads send many rapid scroll events with tiny deltaY values. Each event triggers immediate zoom state change. Solution: implement debouncing or accumulate deltas over short time window (e.g., 16ms) before applying zoom. Mindstrike may benefit from ReactFlow's internal smoothing.
  Implementation approach: Replace panzoom.zoomToPoint() call with manual viewport calculation matching mindstrike's MindMap.tsx lines 1004-1012. Get panzoom's current pan/zoom state, calculate new state with zoom-to-point formula, then apply with panzoom.zoom() and panzoom.pan() separately.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Zoom must be centered on cursor position (point under cursor stays fixed during zoom)
  #   2. Mouse wheel events must use deltaMode to correctly calculate zoom delta (supports LINE=1, PAGE=2, PIXEL=0 modes)
  #   3. Trackpad scroll events must be smoothed/debounced to prevent jittering from rapid tiny deltas
  #   4. Horizontal scroll (deltaX) must always pan left/right regardless of pan mode
  #   5. Manual zoom-to-point calculation: newX = viewport.x + (mouseX - viewport.x) * (1 - zoomRatio)
  #
  # EXAMPLES:
  #   1. User positions mouse at top-right corner of diagram, scrolls up with mouse wheel, diagram zooms in but shifts position (point under cursor moves)
  #   2. User positions mouse at center of diagram, scrolls up with mouse wheel, diagram zooms in and point under cursor stays fixed (correct behavior like mindstrike)
  #   3. User scrolls with trackpad (vertical two-finger swipe), diagram jitters rapidly instead of smooth zoom (many tiny deltaY events cause rapid state changes)
  #   4. User scrolls with trackpad (horizontal two-finger swipe), diagram pans smoothly left/right (deltaX events work correctly)
  #   5. User scrolls mouse wheel down, diagram zooms out centered on cursor position (same zoom-to-point formula applies)
  #
  # ========================================
  Background: User Story
    As a developer viewing mermaid diagrams in fullscreen mode
    I want to zoom in and out from my cursor position smoothly with both mouse and trackpad
    So that I can examine diagram details precisely where I'm looking without the diagram jumping around

  Scenario: Zoom in centered on cursor position with mouse wheel
    Given I have a mermaid diagram open in fullscreen modal
    And my cursor is positioned at coordinates (400, 300) on the diagram
    And the diagram has a specific element at those coordinates
    When I scroll up with the mouse wheel
    Then the diagram should zoom in
    And the element that was under my cursor should remain under my cursor
    And the viewport position should be calculated using the formula: newX = viewport.x + (mouseX - viewport.x) * (1 - zoomRatio)

  Scenario: Zoom out centered on cursor position with mouse wheel
    Given I have a mermaid diagram open in fullscreen modal
    And my cursor is positioned at the top-right corner of the diagram
    And the diagram is zoomed in to 200%
    When I scroll down with the mouse wheel
    Then the diagram should zoom out
    And the point that was under my cursor should remain fixed
    And the zoom calculation should use the same zoom-to-point formula

  Scenario: Smooth zoom with trackpad vertical scroll
    Given I have a mermaid diagram open in fullscreen modal
    And I am using a trackpad
    When I perform a vertical two-finger swipe upward
    Then the zoom events should be debounced or smoothed
    And the diagram should zoom in smoothly without jittering
    And rapid tiny deltaY events should be accumulated before applying zoom

  Scenario: Horizontal pan with trackpad
    Given I have a mermaid diagram open in fullscreen modal
    And I am using a trackpad
    When I perform a horizontal two-finger swipe to the left
    Then the diagram should pan right smoothly
    And deltaX events should be processed immediately
    And pan mode modifier should not affect horizontal scrolling

  Scenario: Handle different mouse wheel deltaMode values
    Given I have a mermaid diagram open in fullscreen modal
    When I scroll with a mouse wheel that reports deltaMode=0 (PIXEL)
    Then the zoom delta should be calculated as: -deltaY * 0.002
    When I scroll with a mouse wheel that reports deltaMode=1 (LINE)
    Then the zoom delta should be calculated as: -deltaY * 0.05
    When I scroll with a mouse wheel that reports deltaMode=2 (PAGE)
    Then the zoom delta should be calculated as: -deltaY * 1
