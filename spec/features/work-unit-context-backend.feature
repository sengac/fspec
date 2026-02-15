@wip
@TUI-059
Feature: Work Unit Context Backend (Rust)
  """
  Architecture notes (TUI-059 - Work Unit Context):
  - WorkUnitContext struct in session_manager.rs stores work unit info in session
  - format_for_environment() returns "Current work unit: ID" (ID only, no title/status)
  - NAPI functions: sessionSetWorkUnitContext, sessionGetWorkUnitContext, sessionGetActive

  Architecture notes (TUI-064 - Current Date):
  - Add date field to EnvironmentInfo struct in context_gathering.rs, include in to_reminder_content()
  - Date must be ISO 8601 format (YYYY-MM-DD), using system local time (not UTC)
  - inject_context_reminders() called at: session creation, clear history
  - Compaction preserves existing reminders via partition_for_compaction() - no refresh needed
  - Resume flow: sessionManagerCreateWithId() injects fresh reminders FIRST, then
  sessionRestoreMessages() appends old messages (skipping old reminders) - fresh date at START
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

  @cli-mode
  Scenario: No notification when no active session exists
    Given there is no active TUI session
    Then context should not be set

  @environment-format
  @TUI-064
  Scenario: Current date appears in environment information
    When the CLI starts an interactive session
    Then the environment information should contain "Date:" followed by a date in YYYY-MM-DD format
    And the date should be the system's local date

  @environment-format
  @ai-behavior
  @TUI-064
  Scenario: AI must use environment date not training data
    Given the environment information contains "Date: 2026-02-14"
    When the AI needs to reference today's date
    Then the AI must use the date from the environment information
    And the AI must not guess or use dates from training data

  @environment-format
  @session-resume
  @TUI-064
  Scenario: Resumed session gets fresh environment info with current date
    Given a session was created yesterday with "Date: 2026-02-13" in environment info
    When I resume that session today on 2026-02-14
    Then the environment information should be reinjected with fresh data
    And the environment information should contain "Date: 2026-02-14"
    And the AI should see today's date, not yesterday's

  @struct-behavior
  Scenario: No notification when updating the same work unit
    Given the session is attached to work unit "AUTH-001"
    When I run "update-work-unit-status AUTH-001 implementing"
    Then the session work unit context should remain "AUTH-001"

  @environment-format
  @compaction
  @TUI-064
  Scenario: Compaction preserves environment date during session
    Given a session was created on "2026-02-14" with environment info containing "Date: 2026-02-14"
    When context compaction is triggered due to context window limits
    Then partition_for_compaction should preserve the environment system reminder
    And the environment information should still contain "Date: 2026-02-14"
    And inject_context_reminders should NOT be called during compaction
