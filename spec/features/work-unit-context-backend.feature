@done
@TUI-059
Feature: Work Unit Context Backend (Rust)

  """
  Architecture notes:
  - WorkUnitContext struct in session_manager.rs stores work unit info in session
  - format_for_environment() returns "Current work unit: ID" (ID only, no title/status)
  - NAPI functions: sessionSetWorkUnitContext, sessionGetWorkUnitContext, sessionGetActive
  """

  Background: User Story
    As a system component
    I want the Rust backend to correctly store and format work unit context
    So that environment information displays the current work unit ID

  @environment-format
  Scenario: Work unit ID appears in environment information when entering AgentView
    Given work unit "AUTH-001" exists in the backlog
    When I select work unit "AUTH-001" and press Enter
    Then I should be in the AgentView
    And the environment information should contain "Current work unit: AUTH-001"
    And the environment information should not contain the work unit title
    And the environment information should not contain the work unit status

  @struct-behavior
  Scenario: LLM receives notification when updating a different work unit
    Given the session is attached to work unit "AUTH-001"
    When I run "update-work-unit-status BUG-002 implementing"
    Then the session work unit context should be updated to "BUG-002"

  @struct-behavior
  Scenario: No notification when updating the same work unit
    And the session work unit context should remain "AUTH-001"

  @cli-mode
  Scenario: No notification when no active session exists
    Given there is no active TUI session
    Then context should not be set
