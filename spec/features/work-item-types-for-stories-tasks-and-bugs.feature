@work-unit-management
@cli
@FEAT-006
@work-units
@type-system
Feature: Work unit types for stories, tasks, and bugs

  Background: User Story
    As an AI agent managing project work with fspec
    I want to categorize work units by type (story, task, bug)
    So that I can track meta-work and housekeeping through Kanban without violating ACDD discipline for user-facing features

  Scenario: Create story work unit with explicit type
    Given I am in a project with fspec initialized
    When I run "fspec create-work-unit AUTH 'User Login' --type=story"
    Then a work unit should be created with id matching "AUTH-\d+"
    And the work unit should have type "story"
    And the work unit should have title "User Login"

  Scenario: Create task work unit with explicit type
    Given I am in a project with fspec initialized
    When I run "fspec create-work-unit CLEAN 'Audit coverage files' --type=task"
    Then a work unit should be created with id matching "CLEAN-\d+"
    And the work unit should have type "task"
    And the work unit should have title "Audit coverage files"

  Scenario: Create bug work unit with explicit type
    Given I am in a project with fspec initialized
    When I run "fspec create-work-unit BUG 'Login fails with @ symbol' --type=bug"
    Then a work unit should be created with id matching "BUG-\d+"
    And the work unit should have type "bug"
    And the work unit should have title "Login fails with @ symbol"

  Scenario: Default type is story for backward compatibility
    Given I am in a project with fspec initialized
    When I run "fspec create-work-unit AUTH 'Feature'" without specifying type
    Then a work unit should be created with type "story"
    And the work unit should have title "Feature"

  Scenario: Filter work units by type
    Given I have work units with different types
    And I have a story work unit "AUTH-001"
    And I have a task work unit "CLEAN-001"
    And I have a bug work unit "BUG-001"
    When I run "fspec list-work-units --type=task"
    Then the output should include "CLEAN-001"
    And the output should not include "AUTH-001"
    And the output should not include "BUG-001"

  Scenario: Task workflow skips testing state
    Given I have a task work unit "CLEAN-001" in backlog
    When I move the work unit through states: specifying → implementing → validating → done
    Then each state transition should succeed
    And attempting to move to "testing" state should fail with error
    And the error should explain "Tasks do not have a testing phase"

  Scenario: Story requires feature file before moving to testing
    Given I have a story work unit "AUTH-001" in specifying state
    And the story has no linked feature file
    When I try to move the work unit to testing state
    Then the command should fail with exit code 1
    And the output should contain "Cannot move to testing: no feature file linked"

  Scenario: Task can move through workflow without feature file
    Given I have a task work unit "CLEAN-001" in backlog
    When I move the work unit to implementing state
    Then the command should succeed
    And no feature file validation should occur

  Scenario: Bug must link to existing feature file
    Given I have a bug work unit "BUG-001" in specifying state
    And no feature file is linked to the bug
    When I try to move the work unit to testing state
    Then the command should fail with exit code 1
    And the output should contain "Bugs must link to existing feature file"
    And the output should suggest "If feature has no spec, create a story instead"

  Scenario: Bug links to existing feature file successfully
    Given I have a feature file "spec/features/user-authentication.feature"
    And I have a bug work unit "BUG-001" for that feature
    When I link the bug to the feature file
    And I move the bug to testing state
    Then the command should succeed

  Scenario: Work unit type is immutable after creation
    Given I have a work unit "AUTH-001" with type "story"
    When I try to change the type to "task"
    Then the command should fail
    And the output should explain "Type is immutable. Delete and recreate if incorrect."

  Scenario: Task supports optional Example Mapping in specifying phase
    Given I have a task work unit "CLEAN-001" in specifying state
    When I add rules and examples to the task using Example Mapping commands
    Then the commands should succeed
    And the task should store the rules and examples
    And moving to implementing should not require Example Mapping fields to be filled

  Scenario: Existing work units default to story type
    Given I have an existing work unit "AUTH-001" without a type field
    When I read the work unit
    Then the work unit should have type "story"
    And no migration command should be required

  Scenario: Query work units with type filtering
    Given I have multiple work units of different types
    When I run "fspec query-work-units --type=task --format=json"
    Then the JSON output should only include work units with type "task"

  Scenario: Metrics reporting can break down by type
    Given I have completed work units of different types
    When I run "fspec query-metrics --type=story"
    Then the output should show metrics for story work units only
    When I run "fspec query-metrics" without type filter
    Then the output should show combined metrics for all types
