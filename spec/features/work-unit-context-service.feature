@done
@TUI-059
Feature: Work Unit Context Service Layer

  """
  Architecture notes:
  - Service layer: workUnitContextService.ts (pure functions), useWorkUnitContext.ts (React hook), workUnitStatusHook.ts (command integration)
  - Pure functions for change detection and reminder formatting are testable without mocks
  - NAPI wrapper functions delegate to Rust backend
  """

  Background: User Story
    As an AI agent
    I want the service layer to detect work unit context changes and format notifications
    So that I can maintain awareness of what I'm working on

  @environment-info
  Scenario: Work unit context stored when entering AgentView
    Given I am on the kanban board
    And work unit "AUTH-001" exists in the backlog
    When I select work unit "AUTH-001" and press Enter
    Then I should be in the AgentView
    And the environment information should contain "Current work unit: AUTH-001"
    And the environment information should not contain the work unit title
    And the environment information should not contain the work unit status

  @status-change @system-reminder
  Scenario: LLM receives notification when updating a different work unit
    Given I am in the AgentView
    And the session is attached to work unit "AUTH-001"
    When I run "update-work-unit-status BUG-002 implementing"
    Then I should receive a system reminder about work unit change
    And the system reminder should mention "AUTH-001" as the previous work unit
    And the system reminder should mention "BUG-002" as the current work unit
    And the session work unit context should be updated to "BUG-002"

  @reminder-formatting
  Scenario: System reminder formatted with previous and current work unit
    Given a work unit context change is detected
    When the system reminder is formatted
    Then the reminder should mention both work units

  @status-change
  Scenario: No notification when updating the same work unit
    Given I am in the AgentView
    And the session is attached to work unit "AUTH-001"
    When I run "update-work-unit-status AUTH-001 testing"
    Then the status should be updated to "testing"
    And I should not receive a work unit change notification
    And the session work unit context should remain "AUTH-001"

  @cli-mode
  Scenario: No notification when no active session exists
    Given I am running fspec commands from the CLI
    And there is no active TUI session
    When I run "update-work-unit-status AUTH-001 testing"
    Then the status should be updated to "testing"
    And I should not receive a work unit change notification
