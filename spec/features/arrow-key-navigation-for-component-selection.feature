@done
@high
@tui
@navigation
@TUI-009
Feature: Arrow Key Navigation for Component Selection

  """
  Critical: Arrow navigation must wrap around (circular). Mental model: horizontal panes = horizontal keys, vertical items = vertical keys. Both Tab and arrow keys should coexist (Tab forward = right arrow, Shift-Tab backward = left arrow).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Left/right arrow navigation applies ONLY to CheckpointViewer and ChangedFilesViewer
  #   2. Arrow navigation wraps around: right arrow on rightmost pane goes to leftmost pane, left arrow on leftmost pane goes to rightmost pane
  #   3. Left/right arrow keys always navigate between panes at parent level (horizontal layout matches horizontal keys). No child components capture these keys in current implementation.
  #   4. Arrow keys work alongside Tab navigation: right arrow and Tab both move forward, left arrow and Shift-Tab both move backward. Both methods remain available for user preference.
  #
  # EXAMPLES:
  #   1. In CheckpointViewer, user on 'checkpoints' pane presses right arrow → focus moves to 'files' pane
  #   2. In CheckpointViewer, user on 'files' pane presses left arrow → focus moves to 'checkpoints' pane
  #   3. In CheckpointViewer, user on 'diff' pane (rightmost) presses right arrow → focus wraps around to 'checkpoints' pane (leftmost)
  #   4. In CheckpointViewer, user on 'checkpoints' pane (leftmost) presses left arrow → focus wraps around to 'diff' pane (rightmost)
  #   5. In ChangedFilesViewer, user on 'files' pane presses right arrow → focus moves to 'diff' pane
  #   6. In ChangedFilesViewer, user on 'diff' pane presses right arrow → focus wraps around to 'files' pane
  #   7. In CheckpointViewer, user on 'checkpoints' pane presses Tab → focus moves to 'files' pane (same as right arrow)
  #   8. In CheckpointViewer, user on 'checkpoints' pane presses up/down arrows → navigates between checkpoint items within the pane (existing behavior preserved)
  #
  # ========================================

  Background: User Story
    As a user navigating the TUI
    I want to use arrow keys (left/right) to move between component selections
    So that navigation feels more intuitive and matches common UI patterns

  Scenario: Navigate forward in CheckpointViewer with right arrow
    Given I am viewing the CheckpointViewer
    And the 'checkpoints' pane is focused
    When I press the right arrow key
    Then the 'files' pane should be focused
    And the 'files' pane heading should have a green background

  Scenario: Navigate backward in CheckpointViewer with left arrow
    Given I am viewing the CheckpointViewer
    And the 'files' pane is focused
    When I press the left arrow key
    Then the 'checkpoints' pane should be focused
    And the 'checkpoints' pane heading should have a green background

  Scenario: Right arrow wraps from rightmost to leftmost pane in CheckpointViewer
    Given I am viewing the CheckpointViewer
    And the 'diff' pane is focused
    When I press the right arrow key
    Then the 'checkpoints' pane should be focused
    And the 'checkpoints' pane heading should have a green background

  Scenario: Left arrow wraps from leftmost to rightmost pane in CheckpointViewer
    Given I am viewing the CheckpointViewer
    And the 'checkpoints' pane is focused
    When I press the left arrow key
    Then the 'diff' pane should be focused
    And the 'diff' pane heading should have a green background

  Scenario: Navigate forward in ChangedFilesViewer with right arrow
    Given I am viewing the ChangedFilesViewer
    And the 'files' pane is focused
    When I press the right arrow key
    Then the 'diff' pane should be focused
    And the 'diff' pane heading should have a green background

  Scenario: Right arrow wraps in ChangedFilesViewer
    Given I am viewing the ChangedFilesViewer
    And the 'diff' pane is focused
    When I press the right arrow key
    Then the 'files' pane should be focused
    And the 'files' pane heading should have a green background

  Scenario: Tab key works alongside right arrow (forward navigation)
    Given I am viewing the CheckpointViewer
    And the 'checkpoints' pane is focused
    When I press the Tab key
    Then the 'files' pane should be focused
    And the navigation should be identical to pressing the right arrow key

  Scenario: Up/down arrow keys continue to navigate within panes
    Given I am viewing the CheckpointViewer
    And the 'checkpoints' pane is focused
    And there are multiple checkpoint items in the list
    When I press the down arrow key
    Then the selection should move to the next checkpoint item within the pane
    And the 'checkpoints' pane should remain focused
    When I press the up arrow key
    Then the selection should move to the previous checkpoint item within the pane
    And the 'checkpoints' pane should remain focused
