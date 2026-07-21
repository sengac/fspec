@TUI-102
Feature: AgentView scrollback scrollbar click-and-drag integration

  """
  AgentView gains scrollbar_drag: ScrollbarDrag field and last_scrollback_total_rows: usize field for geometry caching
  New Action::ScrollbackJumpToOffset(usize) variant added to components/mod.rs Action enum
  handle_scrollback_mouse in mouse_dispatch.rs detects scrollbar column (rightmost col when gutter reserved) and routes to ScrollbarDrag before text selection
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Scrollbar click is detected by hit-testing the rightmost column of the scrollback area when gutter is reserved
  #   2. Click on scrollbar track jumps to the clicked position using ScrollbarDrag state machine
  #   3. Click on scrollbar thumb scrolls one viewport height in click direction
  #   4. Drag on scrollbar thumb continuously updates scroll offset as mouse moves
  #   5. Scrollbar interaction exits stick_to_bottom mode
  #   6. Scrollbar interaction is ignored when content fits in viewport (no gutter reserved)
  #
  # EXAMPLES:
  #   1. User clicks on scrollbar track at row 5 in a 20-row area with 100 total rows and 10 visible. Scrollback jumps to offset 25 and exits stick mode.
  #   2. User clicks on scrollbar track below the thumb. The scrollback jumps down so the clicked position becomes visible at the top of the viewport.
  #   3. User clicks and drags the scrollbar thumb downward. As they drag, the scrollback content scrolls in real time following their mouse position.
  #   4. User quickly clicks on the scrollbar thumb without dragging. The scrollback scrolls down by one viewport height.
  #
  # ========================================

  Background: User Story
    As a user
    I want to click and drag the scrollback scrollbar
    So that quickly navigate to any part of the conversation history

  Scenario: Click on scrollbar track above thumb jumps to that position
    Given the scrollback has more content than fits in the viewport
    And the scrollbar is visible on the rightmost column
    When I click on the scrollbar track at a position above the thumb
    Then the scrollback jumps so the clicked position becomes the top of the viewport
    And the scrollback exits stick-to-bottom mode

  Scenario: Click on scrollbar track below thumb jumps down
    Given the scrollback has more content than fits in the viewport
    And the scrollbar is visible on the rightmost column
    When I click on the scrollbar track below the thumb
    Then the scrollback jumps down so the clicked position becomes visible at the top of the viewport
    And the scrollback exits stick-to-bottom mode

  Scenario: Drag on scrollbar thumb continuously scrolls content
    Given the scrollback has more content than fits in the viewport
    And the scrollbar is visible on the rightmost column
    When I press and drag the scrollbar thumb downward
    Then the scrollback content scrolls in real time following my mouse position
    And the scrollback exits stick-to-bottom mode

  Scenario: Quick click on thumb scrolls one viewport height
    Given the scrollback has more content than fits in the viewport
    And the scrollbar is visible on the rightmost column
    When I quickly click on the scrollbar thumb without dragging
    Then the scrollback scrolls down by one viewport height
    And the scrollback exits stick-to-bottom mode

  Scenario: No scrollbar interaction when content fits in viewport
    Given the scrollback content fits entirely within the viewport
    And no scrollbar is visible
    When I click on the rightmost column of the scrollback area
    Then the click is handled as normal text selection
    And the scrollback offset does not change
