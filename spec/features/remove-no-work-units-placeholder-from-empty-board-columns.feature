@board-visualization
@board
@tui
@phase-1
@BOARD-017
Feature: Remove 'No work units' placeholder from empty board columns
  """
  Modify WorkUnitColumn component in src/tui/components/WorkUnitColumn.tsx to remove 'No work units' placeholder text. When workUnits array is empty, render only the column header with no body content. Update related tests in src/tui/components/__tests__/ to verify empty columns display correctly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Empty columns should display nothing, not 'No work units'
  #   2. Only the column header should be visible for empty columns
  #
  # EXAMPLES:
  #   1. Backlog column has 5 work units, Testing column is empty → Testing column shows only header
  #   2. All columns are empty → All columns show only headers with no placeholder text
  #   3. Done column has work units, all other columns empty → Only Done shows work units, others show just headers
  #
  # ========================================
  Background: User Story
    As a developer using fspec board
    I want to see clean empty columns without placeholder text
    So that the board looks cleaner and less cluttered

  Scenario: Empty column shows only header without placeholder text
    Given the Testing column has no work units
    When I view the Kanban board
    Then the Testing column should show only its header
    And no 'No work units' text should be displayed

  Scenario: All empty columns show clean headers
    Given all Kanban columns are empty
    When I view the Kanban board
    Then all columns should show only their headers
    And no placeholder text should be visible in any column

  Scenario: Mixed columns show work units only where they exist
    Given the Done column has 3 work units
    And all other columns are empty
    When I view the Kanban board
    Then the Done column should show all 3 work units
    And empty columns should show only headers with no placeholder text
