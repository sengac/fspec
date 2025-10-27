@high
@tui
@interactive-cli
@BOARD-008
Feature: Color-coded work units with animated shimmer for active item
  """
  Modify UnifiedBoardLayout.tsx work unit display logic (lines 256-270). Remove emoji icons from typeIcon variable. Use chalk colors: white for stories, red for bugs, blue for tasks. For selected work unit (isSelected check), use green color with background shimmer effect. Implement shimmer using setInterval with 5-second delay that toggles background brightness. Use chalk.bgGreen with alternating intensity for shimmer animation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Remove all emoji icons (📖, 🐛, ⚙️) from work unit display
  #   2. Story work units must be displayed in white color
  #   3. Bug work units must be displayed in red color
  #   4. Task work units must be displayed in blue color
  #   5. Active/selected work unit must be displayed in green color (overrides type color)
  #   6. Active/selected work unit must have animated shimmer background effect
  #   7. Shimmer animation must trigger every 5 seconds
  #
  # EXAMPLES:
  #   1. Story TECH-001 displays as white text without emoji icon
  #   2. Bug BUG-001 displays as red text without emoji icon
  #   3. Task TASK-001 displays as blue text without emoji icon
  #   4. Selected story BOARD-008 displays as green text with shimmer background animation
  #
  # ========================================
  Background: User Story
    As a developer viewing TUI board
    I want to identify work unit types by color instead of emoji
    So that I can quickly scan the board and distinguish stories, bugs, and tasks at a glance

  Scenario: Story work units display in white without emoji
    Given UnifiedBoardLayout renders work units
    And a story work unit TECH-001 is in the backlog column
    When the board is rendered
    Then TECH-001 should display in white color
    And TECH-001 should NOT contain emoji icons (📖)
    And the text should show only the ID and estimate

  Scenario: Bug work units display in red without emoji
    Given UnifiedBoardLayout renders work units
    And a bug work unit BUG-001 is in the implementing column
    When the board is rendered
    Then BUG-001 should display in red color
    And BUG-001 should NOT contain emoji icons (🐛)
    And the text should show only the ID and estimate

  Scenario: Task work units display in blue without emoji
    Given UnifiedBoardLayout renders work units
    And a task work unit TASK-001 is in the testing column
    When the board is rendered
    Then TASK-001 should display in blue color
    And TASK-001 should NOT contain emoji icons (⚙️)
    And the text should show only the ID and estimate

  Scenario: Selected work unit displays in green with shimmer animation
    Given UnifiedBoardLayout renders work units
    And story BOARD-008 is in the implementing column
    And BOARD-008 is the currently selected work unit
    When the board is rendered
    Then BOARD-008 should display in green color (not white)
    And BOARD-008 should have an animated shimmer background
    And the shimmer animation should cycle every 5 seconds
    And BOARD-008 should NOT contain emoji icons
