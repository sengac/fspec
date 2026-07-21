@TUI-101
Feature: Scrollbar click-and-drag navigation core module

  """
  Architecture notes:
  - Pure Rust module in codelet/fspec-tui/src/mouse/scrollbar_drag.rs
  - Implements ScrollbarDrag state machine with on_mouse() returning Option<usize> scroll offset
  - Inverts proportional formula: offset = (click_row * total) / area_height
  - No view dependencies — consumer handles hit-testing and state application
  - Unit tested with injected coordinates. File size < 300 LoC per source-shape convention
  """

  Background: User Story
    As a TUI user
    I want to click or drag on a scrollbar
    So that I can quickly navigate to any position in a scrollable list

  Scenario: Click on track above thumb jumps to that position
    Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
    And the current scroll offset is 0 (thumb occupies rows 0-1)
    When I click the left mouse button at row 5 on the scrollbar track
    Then the ScrollbarDrag should return a scroll offset of 25
    And the state should return to idle after the click

  Scenario: Click on track below thumb jumps to that position
    Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
    And the current scroll offset is 0 (thumb occupies rows 0-1)
    When I click the left mouse button at row 15 on the scrollbar track
    Then the ScrollbarDrag should return a scroll offset of 75
    And the state should return to idle after the click

  Scenario: Click and drag thumb continuously updates scroll offset
    Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
    And the current scroll offset is 0
    When I press the left mouse button on the thumb at row 0
    And I drag the mouse down to row 10
    Then the ScrollbarDrag should return a scroll offset of 50 during the drag
    And releasing the mouse button should return the state to idle

  Scenario: Quick click on thumb without drag scrolls one viewport height
    Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
    And the current scroll offset is 0
    When I quickly click and release the left mouse button on the thumb at row 0 without dragging
    Then the ScrollbarDrag should return a scroll offset of 10 (one viewport height)

  Scenario: Drag continues when cursor strays outside scrollbar area
    Given a scrollbar with 100 total items, 10 visible, and an area height of 20 rows
    And I have pressed the left mouse button on the thumb
    When I drag the mouse to row 15 even if it moves outside the scrollbar rect
    Then the ScrollbarDrag should still compute and return the scroll offset for row 15

  Scenario: Non-left-button events are ignored
    Given a ScrollbarDrag in idle state
    When I scroll the mouse wheel up
    Then the ScrollbarDrag should return None (no action)
    And the state should remain idle

  Scenario: Reset clears dragging state
    Given a ScrollbarDrag in the middle of a drag operation
    When reset is called
    Then the state should return to idle
    And no scroll offset should be returned

  Scenario: No scrollbar needed when content fits in viewport
    Given a scrollbar with 5 total items and 10 visible rows
    When I click at any row on the scrollbar
    Then the ScrollbarDrag should return a scroll offset of 0
    And the state should return to idle
