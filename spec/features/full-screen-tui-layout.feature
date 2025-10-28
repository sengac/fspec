@board-visualization
@high
@tui
@layout
@cross-platform
@ink
@BOARD-013
Feature: Full-Screen TUI Layout
  """
  Create FullScreenWrapper component wrapping BoardView - Uses useStdout hook to detect terminal dimensions (stdout.columns, stdout.rows) - Set Ink Box width to stdout.columns and height to stdout.rows-1 - Component responds to terminal resize events automatically - Clear screen before Ink renders using stdout.write(clearScreen) - Similar to CAGE's FullScreenWrapper at /Users/rquast/projects/cage/packages/cli/src/shared/components/layout/FullScreenWrapper.tsx
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Board must fill entire terminal width and height
  #   2. Board must respond to terminal resize events automatically
  #   3. Screen must be cleared before rendering to start from position (0,0)
  #   4. Layout must use Ink Box with width and height set to terminal dimensions
  #   5. No whitespace or padding should appear outside the board borders
  #
  # EXAMPLES:
  #   1. User opens board in 80x24 terminal, board fills entire 80x24 space
  #   2. User opens board in 120x40 terminal, board fills entire 120x40 space
  #   3. User resizes terminal from 80x24 to 120x40 while board is open, board automatically resizes to 120x40
  #   4. Board renders with no extra whitespace above, below, left, or right of borders
  #   5. Board uses useStdout hook to detect terminal dimensions and updates on resize
  #
  # ========================================
  Background: User Story
    As a developer using fspec board
    I want to view the Kanban board in a full-screen TUI
    So that I can see all work units clearly without wasted screen space

  Scenario: Board fills entire terminal in standard 80x24 terminal
    Given I have a terminal with dimensions 80x24
    When I run the interactive board command
    Then the BoardView should render with width 80 and height 23
    And the board should fill the entire terminal space
    And there should be no whitespace outside the board borders

  Scenario: Board fills entire terminal in larger 120x40 terminal
    Given I have a terminal with dimensions 120x40
    When I run the interactive board command
    Then the BoardView should render with width 120 and height 39
    And the board should fill the entire terminal space
    And there should be no whitespace outside the board borders

  Scenario: Board automatically resizes when terminal dimensions change
    Given I have a terminal with dimensions 80x24
    And the interactive board is running
    When the terminal is resized to 120x40
    Then the BoardView should automatically re-render with width 120 and height 39
    And the board should fill the new terminal space
    And column layouts should adjust to the new width

  Scenario: Screen is cleared before rendering to eliminate artifacts
    Given I have a terminal with previous output displayed
    When I run the interactive board command
    Then the screen should be cleared before rendering
    And the board should render starting from position (0,0)
    And no previous output should be visible

  Scenario: BoardView uses useStdout hook for terminal dimension detection
    Given I have the BoardView component
    When the component initializes
    Then it should call the useStdout hook
    And it should read stdout.columns for terminal width
    And it should read stdout.rows for terminal height
    And it should set Box width to stdout.columns
    And it should set Box height to stdout.rows minus 1
