@done
@SCHED-006
Feature: Overlap & Session Limit Management
  """
  Create SchedulerState struct with active_runs, queued_jobs, deferred_jobs.
  Insert overlap check in evaluate_and_run BEFORE trigger_and_update.
  After trigger_and_update succeeds for agent jobs, insert session_id into active_runs.
  On each tick, sweep active_runs to detect completed sessions and drain queues.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a schedule triggers and its previous run is still active, the overlap_policy field determines behavior: 'skip' silently skips, 'queue' enqueues for later
  #   2. Default overlap_policy is 'skip' when not specified in the schedule entry
  #   3. Active run detection uses an in-memory HashMap<String, Uuid> mapping schedule_name to session_id, checked before each trigger
  #   4. When MAX_SESSIONS (10) is reached, scheduled jobs are deferred to a FIFO queue and retried each 30-second tick
  #   5. Overlap check happens BEFORE session limit check — skip policy prevents attempting to spawn at all
  #   6. Only ONE deferred or queued job is spawned per tick to prevent burst-spawning
  #   7. Completion detection uses poll-on-tick: each 30s tick checks if active_runs session IDs are still present in SessionManager
  #   8. Queued jobs (overlap=queue) wait for the SAME schedule's previous run to complete; deferred jobs wait for ANY session slot to open
  #   9. Shell jobs don't occupy session slots (they use tokio::process::Command, not BackgroundSession), so session limit only applies to agent jobs
  #
  # EXAMPLES:
  #   1. Schedule 'nightly-review' has overlap_policy='skip'. It triggers at 02:00 but the 01:30 ad-hoc run is still active. The scheduler skips the 02:00 trigger and logs a skip event.
  #   2. Schedule 'health-check' has overlap_policy='queue'. It triggers at 10:00 but the 09:55 run is still active. The job is enqueued. When the 09:55 run completes, the queued job fires on the next tick.
  #   3. Schedule has no overlap_policy field. Default 'skip' behavior applies — previous active run causes skip.
  #   4. 10 agent sessions are running (MAX_SESSIONS). An agent schedule triggers. The job is deferred. 30 seconds later a session finishes, the deferred job fires on the next tick.
  #   5. 10 agent sessions running. A shell schedule triggers. Shell jobs run as processes not sessions, so it executes immediately regardless of session limit.
  #   6. An active agent session spawned by schedule 'daily-check' finishes. On the next tick, the scheduler detects it's gone from SessionManager.sessions, removes it from active_runs, and checks the queue.
  #   7. Three different schedules all trigger on the same tick while 8 sessions exist. Two agent jobs are deferred (only one spawns this tick per the one-per-tick rule). Both deferred jobs run on subsequent ticks.
  #   8. A schedule with overlap_policy='queue' already has a queued job. Another trigger fires while still queued. The queue holds the most recent enqueue (replaces, doesn't duplicate).
  #
  # ========================================
  Background: User Story
    As a system operator
    I want to have overlap policies and session limit handling for scheduled jobs
    So that schedules don't pile up when previous runs are still active or all session slots are full

  Scenario: Skip policy prevents trigger when previous run is active
    Given a schedule "nightly-review" with overlap_policy "skip"
    And the schedule has an active run in the scheduler state
    When the schedule triggers on the next cron match
    Then the trigger is skipped
    And a skip event is logged with the schedule name

  Scenario: Queue policy enqueues trigger when previous run is active
    Given a schedule "health-check" with overlap_policy "queue"
    And the schedule has an active run in the scheduler state
    When the schedule triggers on the next cron match
    Then the job is added to the queued_jobs queue
    And the trigger does not spawn a new session immediately

  Scenario: Default overlap policy is skip when not specified
    Given a schedule "daily-task" with no overlap_policy field
    And the schedule has an active run in the scheduler state
    When the schedule triggers on the next cron match
    Then the trigger is skipped as if overlap_policy were "skip"

  Scenario: Queued job fires when previous run completes
    Given a schedule "health-check" with overlap_policy "queue"
    And the schedule has a queued job waiting
    And the previous run's session is no longer in SessionManager
    When the scheduler tick runs and sweeps active_runs
    Then the queued job is fired
    And the schedule's active_runs entry is updated with the new session ID

  Scenario: Agent job deferred when session limit reached
    Given 10 agent sessions are running at MAX_SESSIONS
    And a schedule "report-gen" with job_type "agent" triggers
    When the scheduler attempts to spawn the agent session
    Then the job is added to the deferred_jobs queue
    And the schedule's lastRunStatus is not updated yet

  Scenario: Shell job executes regardless of session limit
    Given 10 agent sessions are running at MAX_SESSIONS
    And a schedule "lint-check" with job_type "shell" triggers
    When the scheduler attempts to execute the shell job
    Then the shell command runs immediately
    And session count remains at 10

  Scenario: Deferred job fires when a session slot opens
    Given a deferred agent job "report-gen" is in the deferred queue
    And a session slot opens (session count drops below MAX_SESSIONS)
    When the scheduler tick runs and processes deferred jobs
    Then the deferred job is spawned as a new agent session
    And it is removed from the deferred queue

  Scenario: Completion detection removes finished sessions from active_runs
    Given a schedule "daily-check" has an active run with session ID in active_runs
    And that session is no longer present in SessionManager sessions
    When the scheduler tick runs and sweeps active_runs
    Then the session ID is removed from active_runs for "daily-check"

  Scenario: Only one deferred job spawns per tick
    Given three agent schedules all trigger on the same tick
    And only 2 session slots are available
    When the scheduler processes all three triggers
    Then one agent job spawns immediately
    And two are added to the deferred queue
    And on the next tick only one deferred job spawns

  Scenario: Queue replaces duplicate entries for same schedule
    Given a schedule "monitor" with overlap_policy "queue"
    And "monitor" already has a queued job waiting
    When the schedule triggers again while still queued
    Then the queue contains only one entry for "monitor" with the latest trigger time
