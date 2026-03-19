@done
@SCHED-008
Feature: Schedule TUI Slash Commands

  """
  Create src/tui/utils/scheduleCommandParser.ts — parseScheduleCommand() takes the raw /schedule input, extracts subcommand and named flags (--cron, --tz, --role, --prompt, --command, --overlap), handling quoted strings
  Create src/tui/services/schedule-service.ts — thin service layer over spec/schedules.json using LockedFileManager for file locking (addSchedule, listSchedules, pauseSchedule, resumeSchedule, removeSchedule)
  Modify src/tui/utils/slashCommands.ts — add { name: 'schedule', description: 'Manage scheduled jobs', syntax: 'add|list|pause|resume|remove [options]', requiresSession: false } to SLASH_COMMANDS array
  Modify src/tui/components/AgentView.tsx — add /schedule dispatch branch in handleSubmitWithCommand that delegates to schedule-service, displays results via addSystemMessage (UserNotification pattern)
  """

  Background: User Story
    Given I am using the fspec TUI

  @happy-path
  Scenario: Add an agent-type schedule via slash command
    Given no schedule named "nightly-review" exists
    When I type "/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs""
    Then the TUI should display a success message containing "nightly-review" and "added"
    And the schedule "nightly-review" should be persisted in schedules.json as type "agent"

  @happy-path
  Scenario: Add a shell-type schedule via slash command
    Given no schedule named "daily-sync" exists
    When I type "/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync""
    Then the TUI should display a success message containing "daily-sync" and "added"
    And the schedule "daily-sync" should be persisted in schedules.json as type "shell"

  @happy-path
  Scenario: List all schedules
    Given a schedule named "nightly-review" exists with cron "0 2 * * *"
    And a schedule named "daily-sync" exists with cron "0 9 * * 1-5"
    When I type "/schedule list"
    Then the TUI should display a table containing "nightly-review" and "daily-sync"
    And the table should include columns for Name, Cron, Timezone, Type, and Status

  @happy-path
  Scenario: Pause an active schedule
    Given an active schedule named "nightly-review" exists
    When I type "/schedule pause nightly-review"
    Then the TUI should display a success message containing "nightly-review" and "paused"
    And the schedule "nightly-review" should have status "paused" in schedules.json

  @happy-path
  Scenario: Resume a paused schedule
    Given a paused schedule named "nightly-review" exists
    When I type "/schedule resume nightly-review"
    Then the TUI should display a success message containing "nightly-review" and "resumed"
    And the schedule "nightly-review" should have status "active" in schedules.json

  @happy-path
  Scenario: Remove an existing schedule
    Given a schedule named "daily-sync" exists
    When I type "/schedule remove daily-sync"
    Then the TUI should display a success message containing "daily-sync" and "removed"
    And the schedule "daily-sync" should not exist in schedules.json

  @validation
  Scenario: Reject invalid cron expression
    When I type "/schedule add bad --cron "not-a-cron" --tz UTC --command "echo hi""
    Then the TUI should display an error message containing "Invalid cron expression"

  @validation
  Scenario: Reject duplicate schedule name
    Given a schedule named "nightly-review" exists
    When I type "/schedule add nightly-review --cron "0 2 * * *" --tz UTC --command "echo""
    Then the TUI should display an error message containing "already exists"

  @validation
  Scenario: Reject removal of nonexistent schedule
    When I type "/schedule remove nonexistent"
    Then the TUI should display an error message containing "not found"

  @validation
  Scenario: Show usage help when no subcommand provided
    When I type "/schedule"
    Then the TUI should display usage help containing "add" and "list" and "pause" and "resume" and "remove"

  @validation
  Scenario: Reject agent schedule missing required fields
    When I type "/schedule add agent-job --cron "0 9 * * *" --tz UTC"
    Then the TUI should display an error message containing "require" and "role" and "prompt"

  @validation
  Scenario: Reject invalid timezone
    When I type "/schedule add test --cron "0 9 * * *" --tz Invalid/Zone --command "echo""
    Then the TUI should display an error message containing "Invalid timezone"
