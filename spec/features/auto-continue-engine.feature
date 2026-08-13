@done
@codelet
@cli
@completion
@streaming
@session
@CONT-002
Feature: Auto-continue engine: done() tool and /continue toggle with nudge budget
  """
  VERIFIED integration points (2026-07-09): Session struct rust/cli/src/session/mod.rs:31-62 (precedent field thinking_exhaustion_cross_turn_count at :57; constructors Session::new :72-88 and from_provider_manager :99-109). Settle point stream_loop.rs run_agent_stream_internal (:269), FinalResponse arm ends with emit_done_with_stop_reason at :1429 (doc said 1428-1430 — exact). Non-nudging emit sites confirmed: :763 (interrupt), :813/:843/:864 (stall_timeout), :1903 (error). PROV-041 re-prompt recipe at :1301-1358 (fresh TokenState+CompactionHook, prompt_streaming_with_history_and_hook, push Message::User, reset per-stream locals, continue).
  DECISION (doc §3 'verify' resolved): agents are rebuilt PER USER MESSAGE — CLI: agent_runner.rs:42 create_rig_agent inside run_agent_with_interruption, with a FRESH session_id per message (agent_runner.rs:32 Uuid::new_v4); NAPI/TUI: agent_loop.rs:982 per dispatched input with stable session.id. So conditional (armed-only) registration IS feasible and takes effect from the next dispatched message. Constraint: create_rig_agent(session_id, preamble, thinking) 3-arg signature is enforced by shape tests rpc082/rpc083/rpc085 — must NOT add a parameter. Chosen mechanism: per-session armed registry in codelet_tools (new rust/tools/src/done.rs, modeled on INJECT_SUMMARY_HANDLERS RwLock registry, inject_summary.rs:90-98); each of the 7 builder chains conditionally adds DoneTool::new(session_id) when codelet_tools::done::is_continue_armed(session_id). Session.continue_enabled remains the single source of truth; the registry is synced immediately before create_rig_agent at the two dispatch sites (agent_runner.rs ~:33 after generating the fresh uuid; agent_loop.rs ~:960 before the provider match). Fallback (always-register + inert) NOT needed.
  Pure decision function: new module rust/cli/src/interactive/auto_continue.rs (<300 lines) exporting ContinueDecision {Finish, Nudge, FinishWithWarning, FinishWithSummary} + decide_continuation(armed, done_summary: Option<&str>, stop_reason: Option<&str>, nudges_used, budget, is_interrupted) implementing the doc §5 table; called in the FinalResponse arm just before stream_loop.rs:1429. Only stop_reason stop/end_turn/None may nudge — max_tokens/length returns Finish (defers to PROV-040/041 which run earlier in the arm). Test pattern mirrors PROV-041: pure fns re-exported via codelet_cli::interactive, tested from rust/cli/tests/ (see thinking_exhaustion_recovery_test.rs:10-15); wiring asserted via source-shape tests (rpc082/083 precedent) rather than a full scripted stream harness — rust's stream_loop has no ScriptedModel-style harness (the CONT-001 harness drives the rig-core loop, not run_agent_stream_internal), so shape tests + pure-fn tests are the feasible red-first surface.
  Command surface wiring (verified): TUI registry rust/fspec-tui/src/views/agent/slash_commands.rs — SlashCommandAction enum :21, name() :45, SLASH_COMMANDS :85. Typed args rust/fspec-tui/src/app/slash_parser.rs::parse_slash_command (:74; /loop routed :123-125) — add '/continue' branch producing SlashCommandParse::ContinueSubcommand(ContinueSubcommand) with a dedicated continue_parser.rs modeled on loop_parser.rs (LoopSubcommand :19). Dispatch rust/fspec-tui/src/app/dispatch_slash_commands.rs — handle_slash_command :27 (palette pick), handle_input_submitted :175 (typed form); backend round-trip per /compact arm :65-97 → new session-state setter in rust/napi/src/session_bindings.rs. CLI repl rust/cli/src/interactive/repl_loop.rs::repl_loop (:16): /continue handler inserted after /compact (:68-134) and BEFORE the provider-switch catch-all at :137; grammar parsing shared via codelet_cli auto_continue module (small duplicate of TUI parser is acceptable — crates don't share a parser crate; ~20 lines, mirrors existing repl ad-hoc pattern).
  """

  Background: User Story
    As a codelet agent-loop user (TUI or CLI repl)
    I want to arm an auto-continue mode with a /continue toggle and budget so the agent is nudged to keep working until it explicitly calls done(summary)
    So that long tasks finish without me babysitting every premature stop, while I keep control via budget, toggle, and interrupt

  Scenario: Session auto-continue state defaults and per-user-turn reset
    Given a newly constructed Session
    Then auto-continue is disabled by default
    And the continue budget defaults to 10
    And the zero-progress nudge count defaults to 0
    When a session that has used 5 nudges begins a new real user turn
    Then the zero-progress nudge count is reset to 0

  Scenario: Off mode finishes exactly as today when the model stops
    Given auto-continue is off
    When the model stops with stop_reason "stop" and no accepted done() call
    Then the continuation decision is Finish
    And no nudge and no warning are produced

  Scenario: Armed stop without done() produces a counted nudge
    Given auto-continue is armed with budget 10 and 0 nudges used
    When the model stops with stop_reason "stop", "end_turn", or no stop_reason and no accepted done() call
    Then the continuation decision is Nudge
    And the nudge text tells the model to call done(summary) if complete or otherwise continue working

  Scenario: Accepted done() finishes the turn and surfaces the summary
    Given auto-continue is armed
    And the model called done with summary "Refactored parser; all tests green"
    When the model stops
    Then the continuation decision is Finish with that summary surfaced
    And no nudge is produced

  Scenario: Budget exhaustion finishes with a visible warning
    Given auto-continue is armed with budget 2 and 2 nudges used
    When the model stops with stop_reason "stop" and no accepted done() call
    Then the continuation decision is Finish with a warning
    And the warning line reports the model never called done() after 2 retries

  Scenario: User interrupt always wins over nudging
    Given auto-continue is armed with remaining budget
    And the user has interrupted the stream
    When the model stops without an accepted done() call
    Then the continuation decision is Finish
    And no nudge is produced even though budget remains

  Scenario: Truncation recovery takes precedence over auto-continue
    Given auto-continue is armed with remaining budget
    When the model stops with stop_reason "max_tokens" or "length" without an accepted done() call
    Then the continuation decision is Finish
    And the existing truncation recovery remains responsible for that stop

  Scenario: A nudge followed by tool activity is refunded
    Given auto-continue is armed and one nudge was just consumed
    When the segment following the nudge produces at least one tool call
    Then that nudge is refunded and does not consume budget
    And a following segment with no tool calls keeps the nudge counted

  Scenario: done() tool records a session-scoped Tier-0 acceptance
    Given a done tool bound to a session
    When the model calls done with summary "task complete"
    Then the acceptance and summary are readable for that session and cleared once taken
    And calling done with an empty summary is rejected without recording acceptance
    And a stale done call arriving while auto-continue is off is accepted inertly without error

  Scenario: done() is registered only while auto-continue is armed
    Given a provider agent is built for a session marked armed
    Then the agent tool set includes the done tool
    And an agent built for an unarmed session does not include the done tool
    And all 7 provider builder chains register the done tool conditionally on the armed state
    And the DeepSearch sub-agent toolset never includes the done tool

  Scenario: Stream loop nudges only at the clean FinalResponse settle point
    Given the stream loop settle points
    Then the continuation decision is consulted only before the FinalResponse emit_done_with_stop_reason
    And the interruption, stall-timeout, and error emit sites never consult the continuation decision
    And a Nudge decision reuses the PROV-041 re-prompt recipe and counts the nudge on the session
