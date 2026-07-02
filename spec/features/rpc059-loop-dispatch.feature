@done
@RPC-059
@rpc
@agent-view
@tui
@slash-command
@rust
@loop-management
Feature: /loop subcommand parser + dispatch
  """
  Phase 7.6 of the RPC-030 roadmap. Reaches TS-parity for the /loop
  slash command by:

  1. Adding THREE new RPC methods through the trait, FspecService,
  FspecBackend, and both transports:
  * loop_add(session_id, interval_seconds, prompt) -> Result<RegisteredLoop>
  * loop_cancel(id) -> Result<bool>
  * loop_list(session_id) -> Vec<RegisteredLoop>
  2. Replacing the `SlashCommandAction::Loop` notice fallback in
  dispatch_slash_commands.rs with a real `handle_slash_loop_help` routed
  through a new app/dispatch_slash_loop.rs file (mirrors the
  dispatch_slash_schedule pattern).
  3. Intercepting `/loop …` (with args) in the submit-line path
  via a new LoopSubcommand variant on SlashCommandParse, then
  fanning the parsed subcommand out to the matching
  handle_loop_* helper.
  4. Routing every subcommand response into the focused session's
  scrollback via Action::EmitSessionNotice so the line lands on
  the right SessionContext even if the user switched tabs mid-RPC.

  TS reference: `src/tui/services/loop-service.ts` —
  `handleLoopCommand(input, sessionId)` and
  `src/tui/utils/loopCommandParser.ts::parseLoopCommand(input)`.

  Out of scope: a dedicated `/loop` view (matches TS — the TUI
  command is purely notice-driven); the loop_store lift itself
  (covered by the loop-store-lift feature file in the same card).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle MUST expose default-impl methods for all three operations so existing handles compile unchanged.
  #   2. StubSessionManagerHandle MUST expose per-call counters and seedable state for cross-transport parity tests.
  #   3. codelet-sessions handle_impl delegates each method to the shared codelet-core::loops::LoopStore singleton, wiring on_fire+idle_check closures that close over the session Arc returned by self.get_session(session_id).
  #   4. parse_loop_command resolves bare /loop to Help, leading-interval Ns/Nm/Nh/Nd to Add, trailing 'every N <unit>' to Add, no-interval body to Add with default 600s, 'list' to List, 'cancel <id>' to Cancel.
  #   5. SlashCommandAction::Loop with no current session is a silent no-op (matches /schedule, /merge-worktree, /clear).
  #   6. SlashCommandAction::Loop (popup pick, no args) emits the Help notice via handle_slash_loop_help.
  #   7. /loop <args> submit-line input is intercepted by parse_slash_command → SlashCommandParse::LoopSubcommand(sub) → Action::LoopSubcommandParsed(sub).
  #   8. Each handle_loop_* spawns a tokio task awaiting the backend round-trip and routes the response via Action::EmitSessionNotice.
  #   9. Notice formats: add → '[loop] scheduled every <intervalStr> [job: ID]'; list → 'Active loops:' + table OR '[loop] No active loops.'; cancel(true) → '[loop] cancelled ID'; cancel(false) → '[error] /loop cancel: Loop "ID" not found'; error → '[error] /loop SUB: {e}'; help → multi-line USAGE_TEXT.
  #   10. With no Tokio runtime (sync unit tests), helpers are graceful no-ops.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with an open AgentView session
    I want to manage session-scoped recurring prompts via /loop [interval] <prompt> | cancel <id> | list from the Rust ratatui frontend
    So that I get full parity with the TS Ink /loop slash command without leaving the TUI

  # ---- Parser scenarios ---------------------------------------------
  Scenario: parse_loop_command resolves bare /loop to Help
    When parse_loop_command("/loop") is invoked
    Then it returns LoopSubcommand::Help

  Scenario: parse_loop_command resolves /loop list
    When parse_loop_command("/loop list") is invoked
    Then it returns LoopSubcommand::List

  Scenario: parse_loop_command resolves /loop cancel <id>
    When parse_loop_command("/loop cancel a1b2c3d4") is invoked
    Then it returns LoopSubcommand::Cancel with id "a1b2c3d4"

  Scenario: parse_loop_command resolves leading-interval seconds
    When parse_loop_command("/loop 30s check the build") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 30 and prompt "check the build"

  Scenario: parse_loop_command resolves leading-interval minutes
    When parse_loop_command("/loop 5m check deployment status") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 300 and prompt "check deployment status"

  Scenario: parse_loop_command resolves leading-interval hours
    When parse_loop_command("/loop 2h check build") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 7200 and prompt "check build"

  Scenario: parse_loop_command resolves leading-interval days
    When parse_loop_command("/loop 1d nightly summary") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 86400 and prompt "nightly summary"

  Scenario: parse_loop_command resolves trailing-every clause
    When parse_loop_command("/loop check status every 2 hours") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 7200 and prompt "check status"

  Scenario: parse_loop_command defaults to 10 minutes when no interval is specified
    When parse_loop_command("/loop check the build") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 600 and prompt "check the build"

  Scenario: parse_loop_command treats minimum interval as 1 second
    When parse_loop_command("/loop 0s prompt") is invoked
    Then it returns LoopSubcommand::Add with interval_seconds 1 and prompt "prompt"

  # ---- slash_parser interception ------------------------------------
  Scenario: parse_slash_command routes a /loop submit-line input to LoopSubcommand
    When parse_slash_command("/loop list") is invoked
    Then it returns SlashCommandParse::LoopSubcommand(LoopSubcommand::List)

  Scenario: parse_slash_command routes bare /loop to LoopSubcommand::Help
    When parse_slash_command("/loop") is invoked
    Then it returns SlashCommandParse::LoopSubcommand(LoopSubcommand::Help)

  # ---- Dispatch scenarios -------------------------------------------
  Scenario: /loop popup pick with no current session is a silent no-op
    Given an App with NO open AgentView session
    When SlashCommandSelected(SlashCommandAction::Loop) is dispatched
    Then no backend method is called
    And no scrollback notice is emitted

  Scenario: /loop popup pick with an open session emits the Help notice
    Given an App with open session s-1
    When SlashCommandSelected(SlashCommandAction::Loop) is dispatched
    Then Action::EmitSessionNotice for s-1 with text starting with "[loop] Usage:" is observed on the action bus
    And no backend method is called

  Scenario: /loop list with two loops emits a multi-line list notice
    Given an App with open session s-1 wired to a MockBackend whose loop_list returns two RegisteredLoop rows
    When Action::LoopSubcommandParsed(LoopSubcommand::List) is dispatched
    Then within 1 second backend.loop_list is called exactly once with session_id s-1
    And within 1 second Action::EmitSessionNotice for s-1 with text containing "Active loops:" is observed on the action bus

  Scenario: /loop list with no loops emits "No active loops."
    Given an App with open session s-1 wired to a MockBackend whose loop_list returns an empty Vec
    When Action::LoopSubcommandParsed(LoopSubcommand::List) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[loop] No active loops." is observed on the action bus

  Scenario: /loop add success emits the "scheduled" notice
    Given an App with open session s-1 wired to a MockBackend whose loop_add returns Ok(RegisteredLoop { id: "ab12cd34", session_id: SessionId::new("s-1"), prompt: "check the build", interval_seconds: 30, created_at: "2026-05-24T00:00:00Z", expires_at: "2026-05-27T00:00:00Z", last_run_at: None })
    When Action::LoopSubcommandParsed(LoopSubcommand::Add { interval_seconds: 30, prompt: "check the build" }) is dispatched
    Then within 1 second backend.loop_add is called exactly once with session_id s-1 and interval_seconds 30 and prompt "check the build"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[loop] scheduled every 30 seconds [job: ab12cd34]" is observed on the action bus

  Scenario: /loop add error emits an error notice
    Given an App with open session s-1 wired to a MockBackend whose loop_add returns Err("Session not found: s-1")
    When Action::LoopSubcommandParsed(LoopSubcommand::Add { interval_seconds: 30, prompt: "p" }) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /loop add: Session not found: s-1" is observed on the action bus

  Scenario: /loop cancel success emits the "cancelled" notice
    Given an App with open session s-1 wired to a MockBackend whose loop_cancel returns Ok(true)
    When Action::LoopSubcommandParsed(LoopSubcommand::Cancel { id: "a1b2c3d4" }) is dispatched
    Then within 1 second backend.loop_cancel is called exactly once with id "a1b2c3d4"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[loop] cancelled a1b2c3d4" is observed on the action bus

  Scenario: /loop cancel unknown id emits a "not found" error notice
    Given an App with open session s-1 wired to a MockBackend whose loop_cancel returns Ok(false)
    When Action::LoopSubcommandParsed(LoopSubcommand::Cancel { id: "does-not-exist" }) is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /loop cancel: Loop \"does-not-exist\" not found" is observed on the action bus

  Scenario: Bare /loop submit-line input emits the Help notice
    Given an App with open session s-1
    When Action::LoopSubcommandParsed(LoopSubcommand::Help) is dispatched
    Then no backend method is called
    And Action::EmitSessionNotice for s-1 with text starting with "[loop] Usage:" is observed on the action bus
