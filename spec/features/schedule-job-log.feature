@done
@SCHED-012
Feature: Schedule Job Log
  """
  New module rust/napi/src/scheduler/job_log.rs — provides append_log_entry() and check_rotation() functions, called from trigger_and_update() and state overlap/defer paths in engine.rs
  Log entry struct is serialized via serde_json — no custom StreamChunk variant needed, this is file I/O only
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Log file is spec/schedule-log.jsonl — JSONL format, one JSON object per line, append-only
  #   2. spec/schedule-log.jsonl must be in .gitignore — it's local runtime state, not project specification
  #   3. Every scheduler lifecycle event is logged: triggered, completed, failed, skipped, deferred, queued
  #   4. Each log entry includes: timestamp (ISO 8601), event type, schedule name, job type (agent/shell), and optional fields: duration_ms, exit_code (shell), error message, session_id (agent)
  #   5. Log rotation: truncate to most recent 1000 entries when file exceeds 2000 entries — prevents unbounded growth
  #   6. Log writes must be non-blocking — append to file asynchronously, never delay the scheduler tick
  #   7. If the log file cannot be written (permissions, disk full), log a warning via tracing but never crash the scheduler
  #
  # EXAMPLES:
  #   1. Scheduler triggers 'daily-sync' agent job → appends {"timestamp":"2026-03-18T10:00:00Z","event":"triggered","schedule":"daily-sync","jobType":"agent","sessionId":"abc-123"} to spec/schedule-log.jsonl
  #   2. Agent job completes → appends entry with event='completed', duration_ms=3200, sessionId
  #   3. Shell job fails with exit code 1 → appends entry with event='failed', exitCode=1, error='npm ERR! missing script: sync'
  #   4. Overlap policy=skip causes a job to be skipped → appends entry with event='skipped', message='Previous run still active'
  #   5. Session limit reached for agent job → appends entry with event='deferred', message='10/10 sessions active'
  #   6. Log has 2001 entries → rotation kicks in, file is truncated to most recent 1000 entries
  #   7. Log file write fails due to permissions → scheduler continues normally, tracing::warn emitted, no crash
  #   8. Overlap policy=queue causes a job to be queued → appends entry with event='queued'
  #
  # ========================================
  Background: User Story
    As a developer using fspec's scheduler
    I want to see a log of what scheduled jobs have run, when they triggered, and whether they succeeded or failed
    So that I can debug schedule issues and understand what the scheduler has been doing without needing a bridge or staring at the TUI

  Scenario: Log agent job triggered event
    Given a project with a scheduled agent job "daily-sync"
    When the scheduler triggers the "daily-sync" job
    Then a JSONL entry is appended to spec/schedule-log.jsonl with event "triggered"
    And the entry contains schedule "daily-sync", jobType "agent", a timestamp, and a sessionId

  Scenario: Log agent job completed event
    Given a scheduled agent job "daily-sync" has been triggered
    When the agent job completes successfully
    Then a JSONL entry is appended with event "completed"
    And the entry contains duration_ms and sessionId

  Scenario: Log shell job failed event
    Given a project with a scheduled shell job "run-tests"
    When the shell job fails with exit code 1 and stderr "npm ERR! missing script: sync"
    Then a JSONL entry is appended with event "failed"
    And the entry contains exitCode 1 and error "npm ERR! missing script: sync"

  Scenario: Log skipped event on overlap
    Given a scheduled job "daily-sync" with overlap policy "skip"
    And the previous run of "daily-sync" is still active
    When the scheduler evaluates "daily-sync" for triggering
    Then a JSONL entry is appended with event "skipped"
    And the entry contains message "Previous run still active"

  Scenario: Log deferred event on session limit
    Given a scheduled agent job "nightly-review"
    And 10 out of 10 sessions are active
    When the scheduler evaluates "nightly-review" for triggering
    Then a JSONL entry is appended with event "deferred"
    And the entry contains message "10/10 sessions active"

  Scenario: Log queued event on overlap queue policy
    Given a scheduled job "hourly-check" with overlap policy "queue"
    And the previous run of "hourly-check" is still active
    When the scheduler evaluates "hourly-check" for triggering
    Then a JSONL entry is appended with event "queued"

  Scenario: Log rotation truncates to 1000 entries
    Given spec/schedule-log.jsonl contains 2001 entries
    When a new log entry is appended
    Then the file is truncated to the most recent 1000 entries plus the new entry

  Scenario: Graceful handling of log write failure
    Given the spec/schedule-log.jsonl file is not writable
    When the scheduler attempts to append a log entry
    Then the scheduler continues operating normally
    And a warning is emitted via tracing
