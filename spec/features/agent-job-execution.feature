@done
@schedule-management @scheduler @SCHED-004
Feature: Agent Job Execution

  """
  Add default_model: RwLock<Option<String>> to SessionManager, set from NAPI on app init so scheduler can resolve model at fire time
  trigger_agent_job needs SessionManager access to call create_session_with_id — use the lazy_static SESSION_MANAGER global from crate::lib
  Replace trigger_agent_job stub with real implementation in new file codelet/napi/src/scheduler/agent_job.rs
  Add schedule_triggered: AtomicBool and schedule_name: RwLock<Option<String>> to BackgroundSession for TUI identification
  Add spawn_scheduled_session method on SessionManager that wraps create_session_with_id + set role + mark schedule flags + send initial prompt
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a schedule with job_type 'agent' fires, spawn a new session via SessionManager.create_session_with_id
  #   2. The session's model is resolved at fire time from the user's default model string (stored in SessionManager)
  #   3. The session role is set from schedule.agent.role (if present)
  #   4. The initial prompt from schedule.agent.prompt is sent as the first user message after session creation
  #   5. Schedule-triggered sessions are marked with schedule_triggered=true and schedule_name on BackgroundSession
  #   6. Session name follows pattern: '[scheduled] {schedule-name} — {ISO timestamp}'
  #   7. Scheduled sessions count toward MAX_SESSIONS (10); if full, the job returns an error
  #   8. After session creation, update lastRunAt and lastRunStatus in schedules.json (already handled by engine.rs trigger_and_update)
  #   9. Missing or empty agent.prompt fails the job gracefully with error status
  #   10. NAPI bindings expose session_is_scheduled(session_id) -> bool and session_schedule_name(session_id) -> Option<String> for TUI
  #
  # EXAMPLES:
  #   1. Schedule 'nightly-review' has agent config with role='Code reviewer' and prompt='Review recent changes'. When it fires, a session named '[scheduled] nightly-review — 2026-03-18T02:00:00Z' is created with the role set and prompt sent.
  #   2. Schedule 'daily-check' has agent config with prompt only (no role). Session is created and prompt is sent, but no role overlay is applied.
  #   3. Schedule 'bad-schedule' has agent config with empty prompt. trigger_agent_job returns Err and schedules.json shows lastRunStatus='error'.
  #   4. 10 sessions already exist (MAX_SESSIONS). Schedule fires but create_session_with_id returns session limit error. Job records error status.
  #   5. Schedule fires with agent config that has no agent.prompt field at all. Job fails with error status 'Missing agent prompt'.
  #   6. TUI calls session_is_scheduled(id) and gets true for a scheduled session, and session_schedule_name(id) returns 'nightly-review'.
  #   7. Session completes naturally (agent loop reaches stop point). The scheduler does NOT forcefully terminate it — it runs to completion.
  #   8. Schedule has no agent config block at all (agent: null/missing). Job fails gracefully with 'Missing agent configuration'.
  #
  # ========================================

  Background: User Story
    As a system operator
    I want to have scheduled agent sessions automatically spawn when a schedule fires
    So that I can run automated tasks like nightly reviews, periodic code analysis, and routine maintenance without manual intervention

  Scenario: Spawn agent session with role and prompt
    Given a schedule "nightly-review" with job_type "agent"
    And the schedule has agent config with role "Code reviewer" and prompt "Review recent changes"
    And a default model "anthropic/claude-sonnet-4" is configured
    When the scheduler triggers the agent job for "nightly-review"
    Then a new session is created via SessionManager
    And the session name matches "[scheduled] nightly-review — {timestamp}"
    And the session role is set to "Code reviewer"
    And the initial prompt "Review recent changes" is sent as the first user message

  Scenario: Spawn agent session with prompt only (no role)
    Given a schedule "daily-check" with job_type "agent"
    And the schedule has agent config with prompt "Check for issues" and no role
    And a default model "anthropic/claude-sonnet-4" is configured
    When the scheduler triggers the agent job for "daily-check"
    Then a new session is created via SessionManager
    And the session has no role overlay applied
    And the initial prompt "Check for issues" is sent as the first user message

  Scenario: Agent job fails when prompt is empty
    Given a schedule "bad-schedule" with job_type "agent"
    And the schedule has agent config with an empty prompt ""
    When the scheduler triggers the agent job for "bad-schedule"
    Then the job returns an error with message containing "Missing agent prompt"
    And schedules.json shows lastRunStatus "error" for "bad-schedule"

  Scenario: Agent job fails when agent config is missing entirely
    Given a schedule "no-config" with job_type "agent"
    And the schedule has no agent config block
    When the scheduler triggers the agent job for "no-config"
    Then the job returns an error with message containing "Missing agent configuration"
    And schedules.json shows lastRunStatus "error" for "no-config"

  Scenario: Agent job fails when prompt field is absent
    Given a schedule "missing-prompt" with job_type "agent"
    And the schedule has agent config with role "reviewer" but no prompt field
    When the scheduler triggers the agent job for "missing-prompt"
    Then the job returns an error with message containing "Missing agent prompt"
    And schedules.json shows lastRunStatus "error" for "missing-prompt"

  Scenario: Agent job fails when session limit is reached
    Given 10 sessions already exist at MAX_SESSIONS capacity
    And a schedule "overflow-job" with job_type "agent"
    And the schedule has agent config with prompt "Run analysis"
    When the scheduler triggers the agent job for "overflow-job"
    Then the job returns an error with message containing "session limit"
    And schedules.json shows lastRunStatus "error" for "overflow-job"

  Scenario: Schedule-triggered session is marked with schedule metadata
    Given a schedule "nightly-review" with job_type "agent"
    And the schedule has agent config with prompt "Review code"
    And a default model "anthropic/claude-sonnet-4" is configured
    When the scheduler triggers the agent job for "nightly-review"
    Then the created session has schedule_triggered set to true
    And the created session has schedule_name set to "nightly-review"

  Scenario: NAPI binding exposes schedule metadata for TUI
    Given a scheduled session exists with schedule_name "nightly-review"
    When the TUI calls session_is_scheduled with the session ID
    Then it returns true
    When the TUI calls session_schedule_name with the session ID
    Then it returns "nightly-review"

  Scenario: Non-scheduled session returns false for schedule queries
    Given a regular (non-scheduled) session exists
    When the TUI calls session_is_scheduled with the session ID
    Then it returns false
    When the TUI calls session_schedule_name with the session ID
    Then it returns None

  Scenario: Agent session runs to natural completion
    Given a schedule "quick-task" with job_type "agent"
    And the schedule has agent config with prompt "Say hello"
    And a default model "anthropic/claude-sonnet-4" is configured
    When the scheduler triggers the agent job for "quick-task"
    Then the session is created and prompt is sent
    And the agent loop runs to its natural stop point without forced termination

  Scenario: Default model is resolved at fire time from SessionManager
    Given a default model "anthropic/claude-sonnet-4" is configured on SessionManager
    And a schedule "model-test" with job_type "agent"
    And the schedule has agent config with prompt "Test model"
    When the scheduler triggers the agent job for "model-test"
    Then the session is created with model "anthropic/claude-sonnet-4"

  Scenario: Agent job fails when no default model is configured
    Given no default model is configured on SessionManager
    And a schedule "no-model" with job_type "agent"
    And the schedule has agent config with prompt "Run task"
    When the scheduler triggers the agent job for "no-model"
    Then the job returns an error with message containing "No default model configured"
    And schedules.json shows lastRunStatus "error" for "no-model"
