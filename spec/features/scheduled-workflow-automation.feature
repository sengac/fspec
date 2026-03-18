@SCHED-001
Feature: Scheduled Workflow Automation
  """
  Scheduler is a tokio task spawned alongside SessionManager::instance(), using the same BackgroundSession/ChainOfCommand/agent_loop patterns as subordinate sessions
  Cron evaluation via a lightweight Rust crate (e.g., cron or croner) — no custom cron parser
  Timer loop: single tokio::spawn with tokio::time::interval(Duration::from_secs(30)) evaluating all schedules — same pattern as the reaper in unified_exec
  Bridge notifications use existing StreamChunk broadcast path — schedule events emit a StreamChunk::ScheduleEvent variant
  spec/schedules.json stores schedule definitions + last-run timestamps together — no separate state file
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Schedules are per-project, persisted in spec/schedules.json
  #   2. Schedules use cron syntax with mandatory timezone (e.g., '0 9 * * *' with 'Australia/Brisbane')
  #   3. Each schedule entry specifies: name, cron expression, timezone, job type (shell command OR agent session with role + initial prompt), and overlap policy (skip or queue)
  #   4. The scheduler is a tokio service inside SessionManager — starts when fspec starts, stops when fspec exits, not a separate daemon
  #   5. Scheduled agent jobs spawn subordinate sessions via AgentManager with full tool access — they get roles, messaging, broadcast observation, and session history like any other subordinate
  #   6. Scheduled agent sessions use the user's default model/provider at execution time — no per-schedule model override
  #   7. No execution timeout — the next trigger waits until the current run's agent loop reaches a stop point; overlap policy controls what happens if the next trigger fires while still running
  #   8. Scheduled sessions count toward MAX_SESSIONS (10) — scheduler queues or skips when full
  #   9. Job results are searchable via SessionSearch — no separate logging system
  #   10. Last-run timestamps are persisted in spec/schedules.json so missed jobs can be detected on restart
  #   11. Catch-up on restart runs at most once per schedule (most recent missed trigger only) — no replay of all missed triggers
  #   12. If a blocklist-blocked tool is hit during a scheduled job, the job fails immediately — SessionSearch is sufficient for post-mortem
  #   13. Schedules are manageable via /schedule slash commands in the TUI and via an AI-callable Schedule tool
  #   14. Bridge notifications (e.g., Telegram) are sent on job failure or completion if a bridge is connected
  #   15. Schedules can be paused and resumed — a paused schedule stays in spec/schedules.json but won't trigger
  #
  # EXAMPLES:
  #   1. User runs '/schedule add nightly-review --cron "0 2 * * *" --tz Australia/Brisbane --role "Code reviewer" --prompt "Review all open PRs and summarize findings"' — schedule is saved to spec/schedules.json and confirmed in TUI
  #   2. User runs '/schedule add daily-sync --cron "0 9 * * 1-5" --tz UTC --command "npm run sync"' — at 9 AM UTC on weekdays, fspec runs the shell command in the project directory
  #   3. User runs '/schedule list' — sees table with name, cron, timezone, type (shell/agent), last run, next run, status (active/paused)
  #   4. At 2:00 AM Brisbane time, the scheduler spawns a subordinate session with 'Code reviewer' role, sends it the prompt, and it appears in TUI session list with a clock icon
  #   5. nightly-review triggers at 2 AM but previous run is still active (overlap: skip) — scheduler skips this trigger and waits for the next one
  #   6. nightly-review triggers at 2 AM but previous run is still active (overlap: queue) — scheduler queues the job and runs it as soon as the current one completes
  #   7. All 10 session slots are full when a scheduled job triggers — scheduler defers the job and retries when a slot frees up
  #   8. fspec restarts after being closed overnight — detects nightly-review missed its 2 AM run, spawns the session once immediately (most recent miss only)
  #   9. fspec restarts and nightly-review missed 3 triggers — only one catch-up run fires, not three
  #   10. A scheduled agent session hits a blocklisted tool (e.g., Write is blocked) — the job fails immediately and the failure is visible via SessionSearch
  #   11. User runs '/schedule pause nightly-review' — job stays in schedules.json but won't trigger; '/schedule resume nightly-review' reactivates it
  #   12. User runs '/schedule remove daily-sync' — schedule is deleted from spec/schedules.json, no future triggers
  #   13. An AI agent uses the Schedule tool to add a schedule programmatically: 'Add a daily test run at 6 AM Sydney time that runs npm test'
  #   14. A scheduled shell command fails with exit code 1 — bridge notification sent via Telegram (if connected) with job name, exit code, and stderr
  #   15. User searches SessionSearch for 'nightly-review' — finds the full session history of past scheduled runs including tool calls and outputs
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should schedules be per-project (spec/schedules.json) or global (~/.config/fspec/schedules.json)? A global scheduler can manage jobs across multiple projects, but per-project is more portable. Or both?
  #   A: Per-project only. Schedules are stored in spec/schedules.json within each project. No global scheduler config.
  #
  #   Q: Should scheduled agent sessions have access to all tools (including Write, Edit, Bash) or be restricted to read-only tools for safety — especially for unattended overnight runs?
  #   A: Full tool access by default. The user configured the schedule, so trust their intent.
  #
  #   Q: What is the maximum execution time for a scheduled job before it's forcibly terminated? Should this be per-schedule configurable or a global default?
  #   A: No timeout. A schedule's next trigger waits until the current run's agent loop reaches a stop point. No forced termination.
  #
  #   Q: How should catch-up work? If fspec was closed for 8 hours and 3 triggers were missed, does it run once (most recent) or replay all 3? Should this be configurable per schedule?
  #   A: Run once on catch-up — only the most recent missed trigger fires, not all missed triggers. Avoids flooding.
  #
  #   Q: Should the blocklist system (BLOCK-002) apply to scheduled sessions? If so, what happens when a scheduled job hits a blocked tool — queue for human approval (impractical at 2 AM) or fail the job?
  #   A: Fail the job if a blocked tool is hit. No special notification — session history via SessionSearch is sufficient for post-mortem.
  #
  #   Q: Should scheduled jobs be able to target a specific work unit context? e.g., 'Every morning, run the test suite for AUTH-001 and report failures' — setting WorkUnitContext on the spawned session
  #   A: No work unit targeting. The prompt can reference a work unit if needed — no special WorkUnitContext wiring required.
  #
  #   Q: What model/provider should scheduled agent sessions use? The user's default? A per-schedule override? What if the configured provider's API key has expired by the time the job runs?
  #   A: Use the user's default model/provider at execution time. No per-schedule model override.
  #
  # ========================================
  Background: User Story
    As a fspec user
    I want to define cron-based schedules that automatically trigger agent sessions or shell commands
    So that routine tasks run unattended on a repeating cadence within my project

  # --- Schedule Management (CRUD) ---
  Scenario: Add an agent schedule via slash command
    Given fspec is running with an active TUI session
    When the user runs "/schedule add nightly-review --cron '0 2 * * *' --tz Australia/Brisbane --role 'Code reviewer' --prompt 'Review all open PRs and summarize findings'"
    Then the schedule "nightly-review" is persisted in spec/schedules.json
    And the schedule entry contains the cron expression "0 2 * * *"
    And the schedule entry contains the timezone "Australia/Brisbane"
    And the schedule entry has job type "agent" with role "Code reviewer"
    And the schedule entry has overlap policy "skip" by default
    And the TUI displays a confirmation that the schedule was added

  Scenario: Add a shell command schedule via slash command
    Given fspec is running with an active TUI session
    When the user runs "/schedule add daily-sync --cron '0 9 * * 1-5' --tz UTC --command 'npm run sync'"
    Then the schedule "daily-sync" is persisted in spec/schedules.json
    And the schedule entry contains the cron expression "0 9 * * 1-5"
    And the schedule entry contains the timezone "UTC"
    And the schedule entry has job type "shell" with command "npm run sync"
    And the TUI displays a confirmation that the schedule was added

  Scenario: List all schedules
    Given fspec is running with schedules "nightly-review" and "daily-sync" configured
    When the user runs "/schedule list"
    Then the TUI displays a table with columns: name, cron, timezone, type, last run, next run, status
    And the table includes a row for "nightly-review" with type "agent" and status "active"
    And the table includes a row for "daily-sync" with type "shell" and status "active"

  Scenario: Pause a schedule
    Given fspec is running with an active schedule "nightly-review"
    When the user runs "/schedule pause nightly-review"
    Then the schedule "nightly-review" has status "paused" in spec/schedules.json
    And the TUI displays a confirmation that the schedule was paused
    And the scheduler does not trigger "nightly-review" at its next cron time

  Scenario: Resume a paused schedule
    Given fspec is running with a paused schedule "nightly-review"
    When the user runs "/schedule resume nightly-review"
    Then the schedule "nightly-review" has status "active" in spec/schedules.json
    And the TUI displays a confirmation that the schedule was resumed

  Scenario: Remove a schedule
    Given fspec is running with a schedule "daily-sync" configured
    When the user runs "/schedule remove daily-sync"
    Then the schedule "daily-sync" is removed from spec/schedules.json
    And the TUI displays a confirmation that the schedule was removed
    And no future triggers fire for "daily-sync"

  Scenario: Add a schedule via the AI-callable Schedule tool
    Given an AI agent session is running
    When the agent calls the Schedule tool to add a schedule named "daily-tests" with cron "0 6 * * *" timezone "Australia/Sydney" and command "npm test"
    Then the schedule "daily-tests" is persisted in spec/schedules.json
    And the tool returns a confirmation with the schedule details

  # --- Job Execution ---
  Scenario: Trigger an agent schedule at cron time
    Given fspec is running with an active agent schedule "nightly-review" set to "0 2 * * *" in "Australia/Brisbane"
    When the current time in Australia/Brisbane reaches 02:00
    Then the scheduler spawns a subordinate session via AgentManager
    And the subordinate session has the role "Code reviewer"
    And the subordinate session receives the configured prompt as its initial message
    And the subordinate session has full tool access
    And the subordinate session uses the user's default model and provider
    And the session appears in the TUI session list with a clock icon indicating it was schedule-triggered

  Scenario: Trigger a shell schedule at cron time
    Given fspec is running with an active shell schedule "daily-sync" set to "0 9 * * 1-5" in "UTC"
    When the current time in UTC reaches 09:00 on a weekday
    Then the scheduler executes "npm run sync" via the Bash tool in the project directory
    And the last-run timestamp for "daily-sync" is updated in spec/schedules.json

  Scenario: Scheduled job results are searchable via SessionSearch
    Given a scheduled agent job "nightly-review" has completed a run
    When the user searches SessionSearch for "nightly-review"
    Then the search results include the full session history of the scheduled run
    And the results include tool calls and their outputs

  # --- Overlap Policies ---
  Scenario: Overlap policy skip - previous run still active
    Given fspec is running with an active agent schedule "nightly-review" with overlap policy "skip"
    And a previous run of "nightly-review" is still active
    When the next cron trigger fires for "nightly-review"
    Then the scheduler skips this trigger
    And the skip is recorded in the session history

  Scenario: Overlap policy queue - previous run still active
    Given fspec is running with an active agent schedule "nightly-review" with overlap policy "queue"
    And a previous run of "nightly-review" is still active
    When the next cron trigger fires for "nightly-review"
    Then the scheduler queues the job
    And the queued job runs as soon as the current run completes

  # --- Session Limits ---
  Scenario: Scheduled job deferred when session limit reached
    Given fspec is running with 10 active sessions (MAX_SESSIONS reached)
    And an active schedule "nightly-review" is configured
    When the cron trigger fires for "nightly-review"
    Then the scheduler defers the job
    And a deferral is recorded in the session history
    And the job runs when a session slot frees up

  # --- Catch-Up on Restart ---
  Scenario: Catch-up fires once for a missed schedule on restart
    Given fspec was closed overnight while schedule "nightly-review" was active with cron "0 2 * * *"
    And the last-run timestamp for "nightly-review" indicates its 02:00 trigger was missed
    When fspec restarts and loads spec/schedules.json
    Then the scheduler detects the missed trigger for "nightly-review"
    And the scheduler spawns the session once immediately as a catch-up run

  Scenario: Catch-up does not replay multiple missed triggers
    Given fspec was closed for 3 days while schedule "nightly-review" was active with cron "0 2 * * *"
    And 3 triggers were missed during the downtime
    When fspec restarts and loads spec/schedules.json
    Then only one catch-up run fires for "nightly-review"
    And the last-run timestamp is updated to the current time

  # --- Error Handling ---
  Scenario: Scheduled job fails when blocked tool is hit
    Given fspec is running with an active agent schedule "nightly-review"
    And the tool "Write" is on the blocklist for the session
    When the scheduled agent session attempts to call the "Write" tool
    Then the job fails immediately
    And the failure is recorded in the session history
    And the failure is discoverable via SessionSearch

  Scenario: Shell command failure sends bridge notification
    Given fspec is running with an active shell schedule "daily-sync"
    And a Telegram bridge is connected
    When the scheduled shell command fails with exit code 1
    Then a notification is sent via the Telegram bridge
    And the notification contains the job name "daily-sync"
    And the notification contains the exit code and stderr output

  Scenario: Agent job completion sends bridge notification
    Given fspec is running with an active agent schedule "nightly-review"
    And a Telegram bridge is connected
    When the scheduled agent session completes successfully
    Then a notification is sent via the Telegram bridge
    And the notification contains the job name "nightly-review" and completion status
