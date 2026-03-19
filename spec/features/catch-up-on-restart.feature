@done
@SCHED-007
Feature: Catch-Up on Restart

  """
  Create codelet/napi/src/scheduler/catch_up.rs with run_catch_up function.
  Wire catch-up into spawn_scheduler: call ONCE before the 30-second tick loop.
  Reuse find_previous_trigger from engine.rs for detection.
  """

  Background: User Story
    As a system operator
    I want to have missed schedule triggers automatically caught up on fspec restart
    So that I don't lose scheduled runs when the application was closed

  Scenario: Catch-up fires once for a missed daily schedule
    Given a schedule "nightly-review" with cron "0 2 * * *" and lastRunAt 3 days ago
    When the scheduler starts and runs catch-up
    Then exactly one catch-up job fires for "nightly-review"
    And lastRunAt is updated to the current time

  Scenario: Catch-up fires for a never-run schedule with a past due trigger
    Given a schedule "new-check" with cron "*/5 * * * *" and no lastRunAt
    When the scheduler starts and runs catch-up
    Then one catch-up job fires for "new-check"

  Scenario: No catch-up when last run is recent enough
    Given a schedule "hourly-task" with cron "0 * * * *" and lastRunAt 30 minutes ago
    When the scheduler starts and runs catch-up
    Then no catch-up job fires for "hourly-task"

  Scenario: Paused schedule is skipped during catch-up
    Given a schedule "paused-job" with status "paused" and a stale lastRunAt
    When the scheduler starts and runs catch-up
    Then no catch-up job fires for "paused-job"

  Scenario: Catch-up does not cause double-fire on first regular tick
    Given a schedule "daily-report" with a missed trigger
    When catch-up fires and updates lastRunAt
    And the first regular 30-second tick evaluates the schedule
    Then the regular tick does not trigger the schedule again

  Scenario: Missing schedules.json on startup is handled gracefully
    Given no schedules.json file exists in the project directory
    When the scheduler starts and runs catch-up
    Then catch-up completes without error and no jobs fire

  Scenario: Catch-up respects session limit
    Given 10 agent sessions are already running
    And a schedule "missed-agent" has a missed trigger
    When the scheduler starts and runs catch-up
    Then the catch-up job is deferred to the deferred queue

  Scenario: Multiple schedules with missed triggers each get one catch-up
    Given schedule "job-a" with lastRunAt 2 days ago
    And schedule "job-b" with lastRunAt 3 days ago
    When the scheduler starts and runs catch-up
    Then one catch-up fires for "job-a" and one for "job-b"
