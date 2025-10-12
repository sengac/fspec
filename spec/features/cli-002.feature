@done
@phase1
@cli
@feature-management
@critical
Feature: List Prefixes Command

  Background: User Story
    As a developer using fspec for project management
    I want to list all registered prefixes with their work unit statistics
    So that I can see which prefixes are in use and their progress

  @CLI-002
  Scenario: List all prefixes with descriptions
    Given I have a project with spec/prefixes.json containing multiple prefixes
    When I run "fspec list-prefixes"
    Then the command should display all prefixes
    And each prefix should show its description
    And the command should exit with code 0

  @CLI-002
  Scenario: List prefixes with work unit statistics
    Given I have prefixes in spec/prefixes.json
    And I have work units with IDs like "SAFE-001", "CLI-002"
    And some work units are in "done" status
    When I run "fspec list-prefixes"
    Then each prefix should show work unit count
    And each prefix should show completion percentage
    And the format should match list-epics output

  @CLI-002
  Scenario: Handle missing prefixes file gracefully
    Given I have a project with no spec/prefixes.json file
    When I run "fspec list-prefixes"
    Then the command should display "No prefixes found" in yellow
    And the command should exit with code 0
