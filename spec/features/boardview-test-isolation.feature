@tui
@testing
@test-coverage
@BUG-046
Feature: Fix failing BoardView TUI tests
  """
  BoardView component uses useFspecStore hook which loads data from process.cwd(). Tests create temporary directories but store ignores them. Solution: Add optional cwd prop to BoardView and useFspecStore to support test directory isolation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BoardView tests must use the test directory's work-units.json, not the project's work-units.json
  #   2. Tests create temporary directories with test data but components load from process.cwd()
  #   3. The fspecStore must accept an optional cwd parameter to support test isolation
  #
  # EXAMPLES:
  #   1. Test creates BOARD-001 in temp dir, but component shows TECH-001 from project dir
  #   2. Test expects 'No work unit selected' but component shows TECH-001 details from project data
  #   3. BoardView accepts optional cwd prop, passes to store for test directory isolation
  #
  # ========================================
  Background: User Story
    As a developer running TUI tests
    I want to test BoardView component in isolation
    So that tests use test data not project data

  Scenario: Test shows work unit from test directory not project directory
    Given I have a test that creates BOARD-001 in a temporary directory
    When I render BoardView component
    Then the component should show BOARD-001 from the test directory
    And the component should not show work units from the project directory

  Scenario: Test shows empty state when test data has no work units
    Given I have a test with an empty work-units.json in a temporary directory
    When I render BoardView component
    Then the component should show 'No work unit selected' message
    And the component should not show work units from the project directory
