@high
@tui
@board-visualization
@layout
@responsive
@ink
@BOARD-014
Feature: Stretch board content to fill available viewport height
  """
  Architecture notes:
  - Use Ink's Box component with flexGrow=1 for work unit columns area to automatically fill available vertical space
  - Separate header panels (Git Stashes, Changed Files, Work Unit Details) from work unit table to minimize fixed height overhead
  - Work unit table renders ONLY column headers, data rows, and minimal borders
  - Ink's flexbox layout automatically calculates height - no manual row calculations needed
  - Respond to useStdout dimensions changes for automatic resize handling via React re-renders
  - Similar to CAGE's responsive layout patterns with flex-based stretching

  Implementation approach:
  - BoardView structure: <FullScreenWrapper><Box flexDirection="column"><HeaderPanels /><Box flexGrow={1}><WorkUnitTable /></Box><Footer /></Box></FullScreenWrapper>
  - WorkUnitTable uses Box with flexGrow to expand and fill remaining space
  - No hardcoded VIEWPORT_HEIGHT constant - let Ink handle layout
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work unit table must expand to fill all available vertical space within FullScreenWrapper
  #   2. Work unit columns area must use Ink Box component with flexGrow=1 to automatically fill available vertical space
  #   3. Header panels (Git Stashes, Changed Files, Work Unit Details) must render separately from work unit table to minimize fixed height overhead
  #   4. No empty space should appear between the work unit rows and the bottom border
  #   5. Table must respond to terminal resize by recalculating available height via React re-renders
  #
  # EXAMPLES:
  #   1. In 80x24 terminal with 3-row header and 1-row footer, columns should have 20 rows for work units
  #   2. In 120x40 terminal with same headers, columns should have 36 rows for work units
  #   3. When terminal resizes from 24 to 40 rows, column height increases from 20 to 36 rows
  #
  # ========================================
  Background: User Story
    As a developer using fspec board
    I want to see the work unit table fill the entire terminal vertically
    So that I can view more work units at once without wasted screen space

  Scenario: Columns fill vertical space in standard 80x24 terminal
    Given I have a terminal with dimensions 80x24
    And the board has a 3-row header section
    And the board has a 1-row footer section
    When I render the BoardView
    Then the work unit columns should have 20 rows available for displaying items
    And there should be no empty space between the last work unit row and the bottom border

  Scenario: Columns fill vertical space in larger 120x40 terminal
    Given I have a terminal with dimensions 120x40
    And the board has a 3-row header section
    And the board has a 1-row footer section
    When I render the BoardView
    Then the work unit columns should have 36 rows available for displaying items
    And there should be no empty space between the last work unit row and the bottom border

  Scenario: Column height adjusts when terminal is resized
    Given I have a terminal with dimensions 80x24
    And the BoardView is rendered with 20 rows for work units
    When the terminal is resized to 120x40
    Then the work unit columns should automatically resize to 36 rows
    And the additional rows should be filled with work unit content or empty space
    And there should be no empty space between the last row and the bottom border
