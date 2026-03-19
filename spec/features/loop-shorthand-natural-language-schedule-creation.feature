@done
@SCHED-011
Feature: Loop Shorthand — Natural Language Schedule Creation

  """
  Deterministic parser (no LLM round-trip) — regex-based interval extraction with /^\d+[smhd]$/ for leading tokens and /every\s+(\d+)\s*(s|sec|seconds?|m|min|minutes?|h|hrs?|hours?|d|days?)$/i for trailing clauses
  Session-scoped schedules are held in-memory in the scheduler (e.g., a HashMap<String, SessionSchedule>) with a sessionScoped: true flag — they bypass spec/schedules.json entirely and include an expiresAt timestamp for 3-day TTL
  Registers as a slash command in slashCommands.ts alongside the /schedule family — handler lives in a dedicated loop-command.ts that calls into the scheduler API
  """

  Background:
    Given the fspec TUI is running

  Scenario: Create loop with leading interval token
    When I type "/loop 5m check deployment status"
    Then the interval should be parsed as 5 minutes
    And the cron expression should be "*/5 * * * *"
    And the prompt should be "check deployment status"
    And the TUI should confirm "Scheduled every 5 minutes" with an 8-character job ID

  Scenario: Create loop with default interval when none specified
    When I type "/loop check the build"
    Then the interval should default to 10 minutes
    And the cron expression should be "*/10 * * * *"
    And the prompt should be "check the build"

  Scenario: Create loop with trailing interval clause
    When I type "/loop check build status every 2 hours"
    Then the trailing "every 2 hours" should be parsed
    And the cron expression should be "0 */2 * * *"
    And the prompt should be "check build status"

  Scenario: Round sub-minute intervals up to one minute
    When I type "/loop 30s run health check"
    Then the interval should be rounded up to 1 minute
    And the cron expression should be "*/1 * * * *"
    And the TUI should confirm "Scheduled every 1 minute (rounded from 30s)"

  Scenario: Chain slash commands as loop prompts
    When I type "/loop 20m /review-pr 1234"
    Then the interval should be parsed as 20 minutes
    And the prompt should be "/review-pr 1234"
    And the scheduler should send the prompt to a subordinate agent session

  Scenario: Cancel an active loop by job ID
    Given a loop is running with job ID "a1b2c3d4"
    When I type "/loop cancel a1b2c3d4"
    Then the session-scoped schedule should be removed
    And the TUI should confirm "Cancelled loop a1b2c3d4"

  Scenario: List all active loops
    Given loops are running with IDs "abc12345" and "def67890"
    When I type "/loop list"
    Then the TUI should display a table of active loops
    And the table should include columns for ID, Prompt, Interval, Next Fire, and Expires

  Scenario: Session-scoped schedules are lost on restart
    Given a loop is running with job ID "a1b2c3d4"
    When fspec exits and restarts
    Then the loop should no longer exist
    And persistent schedules created via "/schedule add" should still be present

  Scenario: Show usage help when no arguments provided
    When I type "/loop" with no arguments
    Then the TUI should display usage help for the /loop command

  Scenario: Overlap policy defaults to skip
    Given a loop is running with job ID "a1b2c3d4" and a previous run is still active
    When the next trigger fires
    Then the trigger should be skipped
    And the TUI should not spawn a duplicate session

  Scenario: Loop schedules count toward session limit
    Given 10 sessions are already running at the maximum limit
    When I type "/loop 5m check status"
    Then the TUI should display an error about the session limit being reached
