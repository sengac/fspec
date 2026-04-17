@done
@schedule-management
@scheduler
@SCHED-003
Feature: Core Scheduler Engine
  """
  Scheduler lives in codelet/napi/src/scheduler/ module with engine.rs and types.rs
  SessionManager stores scheduler_handle: RwLock<Option<JoinHandle<()>>> for graceful shutdown
  Uses croner crate for 5-field cron + timezone-aware scheduling, chrono-tz for IANA timezones
  Follow reaper pattern from codelet/napi/src/unified_exec/reaper.rs for tokio::spawn + interval
  Job execution stubs (trigger_agent_job, trigger_shell_job) return Ok(()) for now — implemented in SCHED-004/005
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Scheduler is a tokio task spawned within SessionManager using a 30-second interval timer
  #   2. Scheduler reads spec/schedules.json each tick to pick up additions/removals without restart
  #   3. Paused schedules (status != 'active') are skipped during evaluation
  #   4. A schedule triggers if last_run_at is None OR last_run_at < previous cron trigger time
  #   5. Cron expressions use schedule's timezone via chrono-tz for IANA timezone conversion
  #   6. Scheduler MUST NOT crash if schedules.json is missing or malformed — log and skip
  #   7. Scheduler MUST NOT block tokio runtime — all operations must be async
  #   8. After job completes, update last_run_at and last_run_status in schedules.json
  #   9. Scheduler starts automatically when first session is created in a project with schedules
  #   10. Scheduler runs for the lifetime of the fspec process — no separate daemon
  #
  # EXAMPLES:
  #   1. Scheduler spawns on first session creation and runs every 30 seconds
  #   2. Schedule with cron '0 * * * *' and last_run_at 1 hour ago triggers on next tick
  #   3. Schedule with cron '0 * * * *' and last_run_at 30 seconds ago does NOT trigger
  #   4. Schedule with last_run_at=null triggers immediately on next tick
  #   5. Paused schedule is skipped even if cron time has passed
  #   6. Schedule with America/New_York timezone triggers at 2am EST not 2am UTC
  #   7. Missing schedules.json file logs warning but scheduler continues running
  #   8. Malformed JSON in schedules.json logs error but scheduler continues running
  #   9. After job triggers, last_run_at and last_run_status are updated in schedules.json
  #   10. New schedule added to schedules.json is picked up on next 30-second tick
  #   11. Agent job type delegates to SCHED-004 trigger_agent_job function
  #   12. Shell job type delegates to SCHED-005 trigger_shell_job function
  #
  # ========================================
  Background: User Story
    As a system operator
    I want to have schedules automatically evaluated on a 30-second interval
    So that scheduled jobs trigger at the configured cron times without manual intervention

  # --- Scheduler Lifecycle ---
  Scenario: Scheduler spawns on first session creation
    Given a project with spec/schedules.json containing active schedules
    When I create a new session in that project
    Then the scheduler task should be spawned
    And the scheduler should run on a 30-second interval

  # --- Cron Trigger Evaluation ---
  Scenario: Schedule triggers when last run is older than previous cron time
    Given a schedule with cron expression "0 * * * *"
    And the schedule's last_run_at is 1 hour ago
    When the scheduler evaluates the schedule
    Then the schedule should trigger

  Scenario: Schedule does not trigger when already run since last cron time
    Given a schedule with cron expression "0 * * * *"
    And the schedule's last_run_at is 30 seconds ago
    When the scheduler evaluates the schedule
    Then the schedule should not trigger

  Scenario: Schedule with no last run triggers immediately
    Given a schedule with cron expression "0 * * * *"
    And the schedule has no last_run_at value
    When the scheduler evaluates the schedule
    Then the schedule should trigger

  # --- Paused Schedules ---
  Scenario: Paused schedule is skipped during evaluation
    Given a schedule with cron expression "0 * * * *"
    And the schedule status is "paused"
    And the cron time has passed since last run
    When the scheduler evaluates the schedule
    Then the schedule should not trigger

  # --- Timezone Handling ---
  Scenario: Schedule respects configured timezone
    Given a schedule with cron expression "0 2 * * *"
    And the schedule timezone is "America/New_York"
    When evaluating if the schedule should trigger
    Then the cron expression should be evaluated in America/New_York time
    And not in UTC time

  # --- Error Resilience ---
  Scenario: Scheduler handles missing schedules file gracefully
    Given a project without spec/schedules.json
    When the scheduler tick runs
    Then a warning should be logged
    And the scheduler should continue running

  Scenario: Scheduler handles malformed JSON gracefully
    Given spec/schedules.json contains invalid JSON
    When the scheduler tick runs
    Then an error should be logged
    And the scheduler should continue running

  # --- Timestamp Updates ---
  Scenario: Job completion updates last run timestamp
    Given a schedule that is ready to trigger
    When the job executes successfully
    Then last_run_at should be updated to the current time
    And last_run_status should be set to "success"

  # --- Dynamic Schedule Discovery ---
  Scenario: New schedule is picked up without restart
    Given the scheduler is already running
    And spec/schedules.json contains no schedules
    When a new schedule is added to spec/schedules.json
    And the scheduler tick runs
    Then the new schedule should be evaluated

  # --- Job Type Delegation ---
  Scenario: Agent job type delegates to agent execution
    Given a schedule with job type "agent"
    When the schedule triggers
    Then trigger_agent_job should be called with the schedule config

  Scenario: Shell job type delegates to shell execution
    Given a schedule with job type "shell"
    When the schedule triggers
    Then trigger_shell_job should be called with the schedule config
