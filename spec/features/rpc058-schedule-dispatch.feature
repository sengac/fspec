@done
@RPC-058
@rpc
@agent-view
@tui
@slash-command
@rust
@schedule-management
Feature: /schedule subcommand parser + dispatch
  """
  Phase 7.5 of the RPC-030 roadmap. Reaches TS-parity for the
  /schedule slash command by:

  1. Adding FIVE new RPC methods through the trait, FspecService,
     FspecBackend, and both transports:
       * schedule_add(name, cron, timezone, job_type, role, prompt,
         command, overlap_policy) -> Result<ScheduledJob>
       * schedule_list() -> Vec<ScheduledJob>
       * schedule_pause(name) -> Result<ScheduledJob>
       * schedule_resume(name) -> Result<ScheduledJob>
       * schedule_remove(name) -> Result<()>
  2. Replacing the `SlashCommandAction::Schedule` notice fallback in
     dispatch_rpc020.rs with a real `handle_slash_schedule_help`
     routed through a new app/dispatch_rpc058.rs file (mirrors the
     dispatch_rpc057 pattern).
  3. Intercepting `/schedule …` (with args) in the submit-line path
     via a new ScheduleSubcommand variant on SlashCommandParse, then
     fanning the parsed subcommand out to the matching
     handle_schedule_* helper.
  4. Routing every subcommand response into the focused session's
     scrollback via Action::EmitSessionNotice so the line lands on
     the right SessionContext even if the user switched tabs mid-RPC.

  TS reference: `src/tui/services/schedule-service.ts` —
  `handleScheduleCommand(input, cwd)` and `src/tui/utils/
  scheduleCommandParser.ts::parseScheduleCommand(input)`.

  Out of scope: a dedicated `/schedule` view (matches TS — the TUI
  command is purely notice-driven); the scheduler engine lift itself
  (covered by the engine-lift feature file in the same card).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle MUST expose default-impl methods for all five operations so existing handles compile unchanged.
  #   2. StubSessionManagerHandle MUST expose per-call counters and seedable state for cross-transport parity tests.
  #   3. codelet-sessions handle_impl delegates each method to a shared CRUD helper (codelet-core::scheduler::crud) that wraps the file-lock + atomic write + cron/timezone validation. repo_path comes from std::env::current_dir().
  #   4. parse_schedule_command tokenises quoted strings and returns ScheduleSubcommand::{Add, List, Pause, Resume, Remove, Help}.
  #   5. SlashCommandAction::Schedule with no current session is a silent no-op (matches /merge-worktree, /clear).
  #   6. SlashCommandAction::Schedule (popup pick, no args) emits the Help notice via handle_slash_schedule_help.
  #   7. /schedule <args> submit-line input is intercepted by parse_slash_command → SlashCommandParse::ScheduleSubcommand(sub) → Action::ScheduleSubcommandParsed(sub).
  #   8. Each handle_schedule_* spawns a tokio task awaiting the backend round-trip and routes the response via Action::EmitSessionNotice.
  #   9. Notice formats: add → '[schedule] added "NAME" (TYPE, CRON, TZ)'; list → multi-line table or '[schedule] No schedules configured.'; pause/resume/remove → '[schedule] STATE "NAME"'; error → '[error] /schedule SUB: {e}'; help → multi-line USAGE_TEXT.
  #   10. With no Tokio runtime (sync unit tests), helpers are graceful no-ops.
  #
  # ========================================

  Background: User Story
    As a fspec TUI user with an open AgentView session
    I want to manage scheduled jobs via /schedule add|list|pause|resume|remove from the Rust ratatui frontend
    So that I have full parity with the TS Ink /schedule slash command without leaving the TUI

  # ---- Parser scenarios ---------------------------------------------

  Scenario: parse_schedule_command resolves bare /schedule to Help
    When parse_schedule_command("/schedule") is invoked
    Then it returns ScheduleSubcommand::Help

  Scenario: parse_schedule_command resolves /schedule list
    When parse_schedule_command("/schedule list") is invoked
    Then it returns ScheduleSubcommand::List

  Scenario: parse_schedule_command resolves a full /schedule add agent command
    When parse_schedule_command is invoked on "/schedule add daily --cron \"0 9 * * *\" --tz UTC --role reviewer --prompt \"daily standup\""
    Then it returns ScheduleSubcommand::Add with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None

  Scenario: parse_schedule_command infers shell job_type from --command flag
    When parse_schedule_command is invoked on "/schedule add backup --cron \"0 2 * * *\" --tz UTC --command \"tar -czf /tmp/backup.tar.gz ~/work\""
    Then it returns ScheduleSubcommand::Add with name "backup" and job_type "shell" and command Some("tar -czf /tmp/backup.tar.gz ~/work") and role None and prompt None

  Scenario: parse_schedule_command resolves /schedule pause <name>
    When parse_schedule_command("/schedule pause daily") is invoked
    Then it returns ScheduleSubcommand::Pause with name "daily"

  Scenario: parse_schedule_command resolves /schedule resume <name>
    When parse_schedule_command("/schedule resume daily") is invoked
    Then it returns ScheduleSubcommand::Resume with name "daily"

  Scenario: parse_schedule_command resolves /schedule remove <name>
    When parse_schedule_command("/schedule remove daily") is invoked
    Then it returns ScheduleSubcommand::Remove with name "daily"

  Scenario: parse_schedule_command falls back to Help on an unknown subcommand
    When parse_schedule_command("/schedule frobnicate") is invoked
    Then it returns ScheduleSubcommand::Help

  # ---- slash_parser interception ------------------------------------

  Scenario: parse_slash_command routes a /schedule submit-line input to ScheduleSubcommand
    When parse_slash_command("/schedule list") is invoked
    Then it returns SlashCommandParse::ScheduleSubcommand(ScheduleSubcommand::List)

  # ---- Dispatch scenarios -------------------------------------------

  Scenario: /schedule popup pick with no current session is a silent no-op
    Given an App with NO open AgentView session
    When SlashCommandSelected(SlashCommandAction::Schedule) is dispatched
    Then no backend method is called
    And no scrollback notice is emitted

  Scenario: /schedule popup pick with an open session emits the Help notice
    Given an App with open session s-1
    When SlashCommandSelected(SlashCommandAction::Schedule) is dispatched
    Then Action::EmitSessionNotice for s-1 with text starting with "[schedule] Usage: /schedule" is observed on the action bus
    And no backend method is called

  Scenario: /schedule list with two schedules emits a multi-line list notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_list returns two ScheduledJob rows
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::List) is dispatched
    Then within 1 second backend.schedule_list is called exactly once
    And within 1 second Action::EmitSessionNotice for s-1 with text containing "[schedule] 2 schedule(s)" is observed on the action bus

  Scenario: /schedule list with no schedules emits "No schedules configured."
    Given an App with open session s-1 wired to a MockBackend whose schedule_list returns an empty Vec
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::List) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] No schedules configured." is observed on the action bus

  Scenario: /schedule add success emits the "added" notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_add returns Ok(ScheduledJob { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", status: "active", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip"), created_at: None, last_run_at: None, last_run_status: None })
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip") }) is dispatched
    Then within 1 second backend.schedule_add is called exactly once with the matching arguments
    And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] added \"daily\" (agent, 0 9 * * *, UTC)" is observed on the action bus

  Scenario: /schedule add error emits an error notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_add returns Err("Timezone is required")
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Add { name: "daily", cron: "0 9 * * *", timezone: "", job_type: "agent", role: Some("r"), prompt: Some("p"), command: None, overlap_policy: None }) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /schedule add: Timezone is required" is observed on the action bus

  Scenario: /schedule pause success emits the "paused" notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_pause returns Ok(ScheduledJob with status "paused")
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Pause { name: "daily" }) is dispatched
    Then within 1 second backend.schedule_pause is called exactly once with name "daily"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] paused \"daily\"" is observed on the action bus

  Scenario: /schedule pause unknown schedule emits an error notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_pause returns Err("Schedule not found: unknown-job")
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Pause { name: "unknown-job" }) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /schedule pause: Schedule not found: unknown-job" is observed on the action bus

  Scenario: /schedule resume success emits the "resumed" notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_resume returns Ok(ScheduledJob with status "active")
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Resume { name: "daily" }) is dispatched
    Then within 1 second backend.schedule_resume is called exactly once with name "daily"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] resumed \"daily\"" is observed on the action bus

  Scenario: /schedule remove success emits the "removed" notice
    Given an App with open session s-1 wired to a MockBackend whose schedule_remove returns Ok(())
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Remove { name: "daily" }) is dispatched
    Then within 1 second backend.schedule_remove is called exactly once with name "daily"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[schedule] removed \"daily\"" is observed on the action bus

  Scenario: Bare /schedule submit-line input emits the Help notice
    Given an App with open session s-1
    When Action::ScheduleSubcommandParsed(ScheduleSubcommand::Help) is dispatched
    Then no backend method is called
    And Action::EmitSessionNotice for s-1 with text starting with "[schedule] Usage: /schedule" is observed on the action bus
