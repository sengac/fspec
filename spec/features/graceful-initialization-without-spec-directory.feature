@done
@scaffolding
@critical
@cli
@initialization
@file-watcher
@resilience
@INIT-016
Feature: Graceful Initialization Without Spec Directory

  """
  Extends existing file watcher mechanism to handle missing files and directories gracefully. Uses fs.watch or chokidar with error handling for ENOENT errors. Implements polling fallback when directory doesn't exist yet. File watcher continues running until explicit termination (Ctrl+C).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When running 'fspec' with no subcommand and spec directory does not exist, system must not crash
  #   2. System displays normal fspec output with empty/no work units when spec directory does not exist
  #   3. System must watch for spec directory and work-units.json creation and auto-reload when they appear
  #   4. System must handle temporary removal of work-units.json during upgrades without crashing
  #   5. Existing file watch mechanism for work-units.json changes must be extended to handle missing file/directory cases
  #   6. Corrupted or invalid work-units.json must be treated same as missing file - show empty board and continue watching
  #   7. No visual distinction between waiting-for-files state and legitimately-empty work units state
  #   8. File watcher must clean up and exit immediately when user terminates fspec command
  #
  # EXAMPLES:
  #   1. User runs 'fspec' before initialization, sees empty Kanban board with no work units
  #   2. User runs 'fspec', sees empty board, then runs 'fspec init' in another terminal, original fspec command auto-refreshes to show initialized work units
  #   3. User has 'fspec' running, upgrade process temporarily deletes work-units.json, fspec shows empty board, upgrade completes and recreates work-units.json, fspec auto-reloads and shows work units again
  #   4. User runs 'fspec', work-units.json contains invalid JSON, system shows empty board and watches, file gets fixed, system auto-reloads with valid work units
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to run fspec commands before spec directory exists
    So that the system gracefully waits and watches instead of crashing

  Scenario: Display empty board when spec directory does not exist
    Given the spec directory does not exist
    When I run 'fspec' with no subcommand
    Then the system should display the normal Kanban board
    And the board should show no work units
    And the system should not crash

  Scenario: Auto-reload when spec directory is created
    Given I have 'fspec' running with no spec directory
    And the system is displaying an empty Kanban board
    When the spec directory is created
    And the work-units.json file is created
    Then the system should automatically reload
    And the system should display the work units from work-units.json

  Scenario: Handle temporary file removal during upgrade
    Given I have 'fspec' running with existing work units
    When the work-units.json file is temporarily deleted during an upgrade
    Then the system should display an empty Kanban board
    And the system should continue watching for the file
    When the work-units.json file is recreated
    Then the system should automatically reload
    And the system should display the work units again

  Scenario: Handle corrupted work-units.json gracefully
    Given the spec directory exists
    But the work-units.json file contains invalid JSON
    When I run 'fspec' with no subcommand
    Then the system should display an empty Kanban board
    And the system should continue watching the file
    When the work-units.json file is fixed with valid JSON
    Then the system should automatically reload
    And the system should display the work units

  Scenario: Clean exit on user termination
    Given I have 'fspec' running and watching for changes
    When I terminate the process with Ctrl+C
    Then the file watcher should clean up immediately
    And the system should exit with code 0
