@board-visualization
@tui
@BOARD-008
Feature: Color-coded work units without shimmer or priority icons

  """
  Architecture notes:
  - Modify UnifiedBoardLayout.tsx work unit display logic (around line 333-340)
  - Remove priority emoticons (🔴, 🟡, 🟢) from display
  - Use chalk colors: white for stories, red for bugs, blue for tasks, bgGreen for selected
  - Story points format: only show when estimate > 0, display as [N]
  - Selected work units: green background (no shimmer), black text
  - Do NOT add shimmer animation to selected items (that's BOARD-009)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Story work units must display in white color
  #   2. Bug work units must display in red color
  #   3. Task work units must display in blue color
  #   4. Selected work unit must display with green background (overrides type color)
  #   5. Story points must be hidden when estimate is 0 or undefined
  #   6. Story points must display as [N] format when estimate > 0
  #   7. No priority emoticons must be displayed
  #   8. No shimmer animation on selected work units
  #
  # EXAMPLES:
  #   1. Story TECH-001 with 0 estimate displays as 'TECH-001' in white (no story points shown)
  #   2. Story FEAT-002 with 5 estimate displays as 'FEAT-002 [5]' in white
  #   3. Bug BUG-001 with 3 estimate displays as 'BUG-001 [3]' in red
  #   4. Task TASK-001 with no estimate displays as 'TASK-001' in blue
  #   5. Selected story displays with green background, no shimmer, no priority icons
  #
  # ========================================

  Background: User Story
    As a developer viewing TUI board
    I want to identify work unit types by color instead of emoji
    So that I can quickly scan the board and distinguish stories, bugs, and tasks at a glance

  Scenario: Story work unit with zero estimate hides story points
    Given UnifiedBoardLayout renders work units
    And story work unit TECH-001 has estimate 0
    When the board is rendered
    Then TECH-001 should display as 'TECH-001' with no story points
    And TECH-001 should display in white color
    And TECH-001 should NOT contain priority emoticons

  Scenario: Story work unit with estimate shows story points in bracket format
    Given UnifiedBoardLayout renders work units
    And story work unit FEAT-002 has estimate 5
    When the board is rendered
    Then FEAT-002 should display as 'FEAT-002 [5]'
    And FEAT-002 should display in white color
    And FEAT-002 should NOT contain priority emoticons

  Scenario: Bug work unit with estimate shows story points in bracket format
    Given UnifiedBoardLayout renders work units
    And bug work unit BUG-001 has estimate 3
    When the board is rendered
    Then BUG-001 should display as 'BUG-001 [3]'
    And BUG-001 should display in red color
    And BUG-001 should NOT contain priority emoticons

  Scenario: Task work unit without estimate hides story points
    Given UnifiedBoardLayout renders work units
    And task work unit TASK-001 has no estimate
    When the board is rendered
    Then TASK-001 should display as 'TASK-001' with no story points
    And TASK-001 should display in blue color
    And TASK-001 should NOT contain priority emoticons

  Scenario: Selected work unit displays with green background without shimmer
    Given UnifiedBoardLayout renders work units
    And story work unit AUTH-001 has estimate 5
    And AUTH-001 is the currently selected work unit
    When the board is rendered
    Then AUTH-001 should display as 'AUTH-001 [5]'
    And AUTH-001 should display with green background color
    And AUTH-001 should NOT have shimmer animation
    And AUTH-001 should NOT contain priority emoticons
