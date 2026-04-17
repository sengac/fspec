@done
@json-schema
@persistence
@schedule-management
@scheduler
@SCHED-002
Feature: Schedule Persistence & Schema
  """
  TypeScript types in src/types/schedule.ts, JSON Schema in src/schemas/schedule.schema.json
  LockedFileManager.transaction() for atomic writes; Rust reads file directly via tokio::fs (no shared lock needed)
  Commands in src/commands/schedule/ directory: add, remove, pause, resume, list
  Cron validation via cron-validate npm package; timezone validation via Intl.supportedValuesOf('timeZone')
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Schedules are per-project, persisted in spec/schedules.json
  #   2. Each schedule entry has: name, cron, timezone, jobType (agent|shell), overlapPolicy (skip|queue), status (active|paused), lastRunAt, lastRunStatus, createdAt
  #   3. Agent jobs additionally store role and prompt; shell jobs store command
  #   4. Cron expressions must be valid 5-field standard cron syntax, validated at write time
  #   5. Timezone must be a valid IANA timezone string (validated via Intl.supportedValuesOf('timeZone'))
  #   6. Schedule names must be unique, lowercase, hyphenated slugs (no spaces)
  #   7. All writes use LockedFileManager.transaction() for concurrent access safety
  #   8. File includes a version field for future migrations (initial: 1.0.0)
  #   9. JSON Schema validation via Ajv runs on every write to prevent invalid data
  #
  # EXAMPLES:
  #   1. User adds agent schedule 'nightly-review' with cron '0 2 * * *', timezone 'Australia/Brisbane', role 'Code reviewer', prompt 'Review PRs' — schedule is persisted to spec/schedules.json with all fields
  #   2. User adds shell schedule 'daily-sync' with cron '0 9 * * 1-5', timezone 'UTC', command 'npm run sync' — schedule is persisted with jobType 'shell'
  #   3. User tries to add schedule with invalid cron '0 99 * * *' — validation fails with error message, no file write occurs
  #   4. User tries to add schedule with invalid timezone 'Fake/City' — validation fails with error message listing valid timezones
  #   5. User tries to add schedule with name 'My Schedule!' (spaces and special chars) — validation fails requiring slug format
  #   6. User tries to add schedule with name 'nightly-review' when it already exists — validation fails with 'schedule already exists' error
  #   7. User pauses schedule 'nightly-review' — status field updated to 'paused', schedule remains in file but won't trigger
  #   8. User resumes paused schedule 'nightly-review' — status field updated back to 'active'
  #   9. User removes schedule 'daily-sync' — entry deleted from schedules.json, no trace remains
  #   10. User lists schedules — sees table with name, cron, timezone, type, status, last run, next run
  #   11. spec/schedules.json doesn't exist — ensureSchedulesFile creates it with version: '1.0.0' and empty schedules object
  #   12. Scheduler completes a run — lastRunAt and lastRunStatus fields are updated in the schedule entry
  #
  # ========================================
  Background: User Story
    As a system administrator
    I want to define and persist schedule configurations
    So that the scheduler engine can reliably read and execute scheduled jobs

  # ========================================
  # SCENARIOS
  # ========================================
  @happy-path
  Scenario: Add an agent schedule with all required fields
    Given the project has no schedules configured
    When I add an agent schedule "nightly-review" with:
      | cron     | 0 2 * * *          |
      | timezone | Australia/Brisbane |
      | role     | Code reviewer      |
      | prompt   | Review PRs         |
    Then the schedule should be persisted to spec/schedules.json
    And the schedule entry should have jobType "agent"
    And the schedule entry should have status "active"
    And the schedule entry should have a createdAt timestamp

  @happy-path
  Scenario: Add a shell command schedule
    Given the project has no schedules configured
    When I add a shell schedule "daily-sync" with:
      | cron     | 0 9 * * 1-5  |
      | timezone | UTC          |
      | command  | npm run sync |
    Then the schedule should be persisted to spec/schedules.json
    And the schedule entry should have jobType "shell"
    And the schedule entry should have status "active"

  @validation
  @error-handling
  Scenario: Reject schedule with invalid cron expression
    Given the project has no schedules configured
    When I try to add a schedule "bad-cron" with cron "0 99 * * *"
    Then the validation should fail with an error about invalid cron syntax
    And spec/schedules.json should not be modified

  @validation
  @error-handling
  Scenario: Reject schedule with invalid timezone
    Given the project has no schedules configured
    When I try to add a schedule "bad-tz" with timezone "Fake/City"
    Then the validation should fail with an error about invalid timezone
    And the error message should suggest valid timezone values
    And spec/schedules.json should not be modified

  @validation
  @error-handling
  Scenario: Reject schedule with invalid name format
    Given the project has no schedules configured
    When I try to add a schedule "My Schedule!" with valid cron and timezone
    Then the validation should fail requiring slug format
    And spec/schedules.json should not be modified

  @validation
  @error-handling
  Scenario: Reject duplicate schedule name
    Given a schedule "nightly-review" already exists
    When I try to add another schedule named "nightly-review"
    Then the validation should fail with "schedule already exists" error
    And the existing schedule should remain unchanged

  Scenario: Pause an active schedule
    Given an active schedule "nightly-review" exists
    When I pause the schedule "nightly-review"
    Then the schedule status should be updated to "paused"
    And the schedule should remain in spec/schedules.json
    And all other schedule fields should be unchanged

  Scenario: Resume a paused schedule
    Given a paused schedule "nightly-review" exists
    When I resume the schedule "nightly-review"
    Then the schedule status should be updated to "active"

  Scenario: Remove a schedule
    Given a schedule "daily-sync" exists
    When I remove the schedule "daily-sync"
    Then the schedule should be deleted from spec/schedules.json
    And no trace of "daily-sync" should remain in the file

  Scenario: List all configured schedules
    Given the following schedules exist:
      | name           | cron        | timezone           | type  | status |
      | nightly-review | 0 2 * * *   | Australia/Brisbane | agent | active |
      | daily-sync     | 0 9 * * 1-5 | UTC                | shell | paused |
    When I list all schedules
    Then I should see a table with columns: name, cron, timezone, type, status, last run, next run
    And the table should contain both schedules

  @initialization
  Scenario: Auto-create schedules file when missing
    Given spec/schedules.json does not exist
    When I run a schedule command that requires the file
    Then spec/schedules.json should be created
    And the file should have version "1.0.0"
    And the schedules object should be empty

  Scenario: Update last run timestamp after execution
    Given a schedule "nightly-review" exists with no previous runs
    When the scheduler engine completes a run of "nightly-review"
    Then the lastRunAt field should be updated to the current timestamp
    And the lastRunStatus field should be set to "completed"
