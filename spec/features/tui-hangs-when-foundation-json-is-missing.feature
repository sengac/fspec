@done
@cli
@board-visualization
@bug
@tui
@board-view
@ui-enhancement
@BUG-072
Feature: TUI hangs when foundation.json is missing

  """
  Architecture notes:
  - TUI is launched from src/index.ts when no command arguments provided
  - BoardView component (src/tui/components/BoardView.tsx) handles TUI rendering
  - fspecStore (src/tui/store/fspecStore.ts) loads data via loadData() function
  - ensureWorkUnitsFile() and ensureEpicsFile() create files if they don't exist
  - No foundation.json check exists in TUI launch path (unlike "fspec board" command)

  Critical implementation requirements:
  - TUI must gracefully handle missing foundation.json (don't hang)
  - loadData() errors should be caught and displayed to user
  - Empty board state must render properly (all columns empty)
  - ESC key handler must work regardless of data state
  - Consider adding helpful message when board is empty and no foundation exists
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TUI must not hang or pause when foundation.json is missing
  #   2. TUI should display an empty board when no work units exist
  #   3. TUI must be responsive and allow user to exit with ESC key
  #
  # EXAMPLES:
  #   1. User runs 'fspec' in ~/projects/cagetools (no spec dir), TUI loads showing empty board
  #   2. User runs 'fspec' in project with spec dir but no foundation.json, TUI loads showing empty board
  #   3. User presses ESC key in TUI, app exits gracefully
  #
  # ========================================

  Background: User Story
    As a developer using fspec in a new project
    I want to run the TUI without a foundation.json file
    So that I see either an empty board or a helpful error message instead of the app hanging

  Scenario: TUI loads with no spec directory
    Given I am in a project with no spec directory
    When I run "fspec" with no arguments
    Then the TUI should load within 5 seconds
    And the board should display empty columns
    And the TUI should be responsive to keyboard input

  Scenario: TUI loads with spec directory but no foundation.json
    Given I am in a project with a spec directory
    But no foundation.json file exists
    When I run "fspec" with no arguments
    Then the TUI should load within 5 seconds
    And the board should display empty columns
    And the TUI should be responsive to keyboard input

  Scenario: User can exit TUI with ESC key
    Given the TUI is running without foundation.json
    When I press the ESC key
    Then the TUI should exit gracefully within 1 second