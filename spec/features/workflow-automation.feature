@cli
@project-management
@workflow-automation
@acdd
Feature: Workflow Automation
  """
  Architecture notes:
  - Workflow automation integrates PM workflow with development lifecycle
  - Auto-track metrics when work is completed
  - Validate state transitions align with ACDD
  - Auto-move work units through workflow states
  - Provides utilities for external tools (e.g., automation hooks, custom scripts)

  Critical implementation requirements:
  - MUST record tokens consumed during work
  - MUST increment iterations on each work cycle
  - MUST validate Gherkin scenarios exist before testing state
  - MUST enforce ACDD workflow progression
  - MUST auto-transition on successful validation
  - MUST update work unit status appropriately

  Integration points for external tools:
  - post-tool-use: Record iteration, check for scenario generation
  - pre-commit: Validate specs aligned with work unit
  - post-commit: Auto-advance work unit state if appropriate
  - post-test: Update metrics, transition state

  References:
  - Project Management Design: project-management.md (section 14)
  """

  Background: User Story
    As a developer using automated workflows
    I want fspec to integrate with my development lifecycle
    So that project management stays synchronized with actual work

  @critical
  @automation
  Scenario: Record iteration after tool use
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec record-iteration AUTH-001"
    Then the iterations count should increment on "AUTH-001"
    And the updatedAt timestamp should be updated

  @automation
  @validation
  Scenario: Validate spec alignment before commit
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    And no scenarios are tagged with "@AUTH-001"
    When I run "fspec validate-spec-alignment AUTH-001"
    Then the command should warn "No scenarios for AUTH-001"

  @automation
  @auto-transition
  Scenario: Auto-advance state after tests pass
    Given I have a project with spec directory
    And a work unit "AUTH-001" has status "testing"
    When I run "fspec auto-advance AUTH-001 --from=testing --event=tests-pass"
    Then the work unit should transition to "implementing"

  @automation
  @completion
  Scenario: Auto-mark done after validation succeeds
    Given I have a project with spec directory
    And a work unit "AUTH-001" has status "validating"
    When I run "fspec auto-advance AUTH-001 --from=validating --event=validation-pass"
    Then the work unit should transition to "done"
    And completion timestamp should be recorded
