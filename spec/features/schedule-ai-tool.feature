@done
@SCHED-009
Feature: Schedule AI Tool
  """
  Create rust/tools/src/schedule/mod.rs — ScheduleTool struct with session_id, impl Tool for ScheduleTool with ScheduleArgs (action, name, cron, timezone, job_type, role, prompt, command, overlap_policy)
  Create rust/tools/src/schedule/handler.rs — ScheduleHandler type alias, static SCHEDULE_HANDLERS registry, set_schedule_handler, execute_schedule_command, has_schedule_handler, clear_all_schedule_handlers
  Create rust/napi/src/schedule_handler.rs — create_handler function that returns a closure reading/writing spec/schedules.json, dispatching on action (add/list/pause/resume/remove)
  Modify session_manager.rs — register schedule handler before agent run, clean up (set to None) after agent run, following the existing pattern for SessionSearch/AgentManager/Fspec handlers
  Create facade files for multi-provider support — schedule_facade.rs with ClaudeScheduleFacade, GeminiScheduleFacade, OpenAIScheduleFacade, ZAIScheduleFacade. Register in ProviderToolRegistry
  """

  Background: 
    Given a session with a registered schedule handler
    And an empty schedules.json file

  Scenario: Add an agent-type schedule
    When the Schedule tool is called with action "add", name "nightly-review", cron "0 2 * * *", timezone "Australia/Sydney", job_type "agent", role "Code reviewer", and prompt "Review recent changes"
    Then the response should have success true and action "add"
    And the response schedule should have name "nightly-review", cron "0 2 * * *", timezone "Australia/Sydney", and job_type "agent"
    And schedules.json should contain a schedule named "nightly-review"

  Scenario: Add a shell-type schedule
    When the Schedule tool is called with action "add", name "daily-lint", cron "0 6 * * 1-5", timezone "UTC", job_type "shell", and command "npm run lint"
    Then the response should have success true and action "add"
    And the response schedule should have name "daily-lint" and job_type "shell"
    And schedules.json should contain a schedule named "daily-lint"

  Scenario: List all schedules
    Given a schedule named "nightly-review" exists with cron "0 2 * * *" and type "agent"
    And a schedule named "daily-sync" exists with cron "0 9 * * 1-5" and type "shell"
    When the Schedule tool is called with action "list"
    Then the response should have success true and action "list"
    And the response should contain 2 schedules with names "nightly-review" and "daily-sync"

  Scenario: Pause an active schedule
    Given a schedule named "nightly-review" exists with status "active"
    When the Schedule tool is called with action "pause" and name "nightly-review"
    Then the response should have success true and action "pause"
    And schedules.json should show "nightly-review" with status "paused"

  Scenario: Resume a paused schedule
    Given a schedule named "nightly-review" exists with status "paused"
    When the Schedule tool is called with action "resume" and name "nightly-review"
    Then the response should have success true and action "resume"
    And schedules.json should show "nightly-review" with status "active"

  Scenario: Remove an existing schedule
    Given a schedule named "nightly-review" exists
    When the Schedule tool is called with action "remove" and name "nightly-review"
    Then the response should have success true and action "remove"
    And schedules.json should not contain a schedule named "nightly-review"

  Scenario: Reject invalid cron expression
    When the Schedule tool is called with action "add", name "bad", cron "not-a-cron", timezone "UTC", and job_type "shell"
    Then the response should have success false
    And the error message should contain "Invalid cron expression"

  Scenario: Reject duplicate schedule name
    Given a schedule named "existing-schedule" exists
    When the Schedule tool is called with action "add" and name "existing-schedule" with valid parameters
    Then the response should have success false
    And the error message should contain "Schedule already exists"

  Scenario: Reject removal of nonexistent schedule
    When the Schedule tool is called with action "remove" and name "nonexistent"
    Then the response should have success false
    And the error message should contain "Schedule not found"

  Scenario: Graceful error when no handler is registered
    Given a session with no registered schedule handler
    When execute_schedule_command is called for that session
    Then the response should have success false
    And the error message should contain "No schedule handler registered"

  Scenario: Reject invalid timezone
    When the Schedule tool is called with action "add", name "test", cron "0 9 * * *", timezone "Invalid/Zone", and job_type "shell"
    Then the response should have success false
    And the error message should contain "Invalid timezone"

  Scenario: Reject agent job missing required fields
    When the Schedule tool is called with action "add", name "test", cron "0 9 * * *", timezone "UTC", job_type "agent", without role or prompt
    Then the response should have success false
    And the error message should contain "Agent jobs require"

  Scenario: ScheduleTool registered in all provider agent builders
    Given each provider's create_rig_agent method is called with a session_id
    When the agent is built with the tool chain
    Then the tool definitions should include a Schedule tool for Claude, Gemini, OpenAI, Z.AI, and Codex providers
