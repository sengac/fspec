@done
@bug-fix
@phase2
@cli
@workflow
@BUG-006
Feature: Parent work units shouldn't require scenarios

  Background: User Story
    As a developer working on parent work units
    I want to move parent work units through workflow without requiring scenarios
    So that parent work units are containers that don't need their own feature files

  Scenario: Parent work unit moves through workflow without scenario validation
    Given I have a parent work unit with children array
    And all children are marked done
    When I run `fspec update-work-unit-status PARENT-001 testing`
    Then the command should succeed without checking for scenarios
    And I can move through implementing, validating, and done states

  Scenario: Leaf work unit requires scenarios to move to testing
    Given I have a leaf work unit with no children
    And no feature file has @LEAF-001 tag
    When I run `fspec update-work-unit-status LEAF-001 testing`
    Then the command should fail with "No Gherkin scenarios found"

  Scenario: Parent work unit blocked when children incomplete
    Given I have a parent work unit with children array
    And one child is still in implementing status
    When I run `fspec update-work-unit-status PARENT-001 done`
    Then the command should fail with "Cannot mark parent as done while children are incomplete"
