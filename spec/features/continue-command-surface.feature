@done
@tui
@cli
@codelet
@completion
@slash-commands
@CONT-002
Feature: Continue Command Surface
  """
  Grammar (doc §4, CONT-002): /continue toggles; /continue <n> (n>=1) arms with budget n or updates budget when already on; /continue on|off explicit with default budget; /continue 0 rejected with hint 'use /continue off'; invalid arg errors with state unchanged; new state always printed.
  TUI wiring (verified 2026-07-09): registry rust/fspec-tui/src/views/agent/slash_commands.rs (SlashCommandAction enum :21, name() :45, SLASH_COMMANDS :85); typed args rust/fspec-tui/src/app/slash_parser.rs::parse_slash_command (:74, /loop routed :123-125) gains a '/continue' branch returning SlashCommandParse::ContinueSubcommand backed by a new continue_parser.rs modeled on loop_parser.rs (LoopSubcommand :19); dispatch rust/fspec-tui/src/app/dispatch_slash_commands.rs handle_slash_command :27 (palette) and handle_input_submitted :175 (typed) with the /compact backend round-trip pattern (:65-97) driving a new session-state setter in rust/napi/src/session_bindings.rs; status-bar indicator '⏩ auto-continue (n/N)' while armed via a pure formatting helper.
  CLI repl wiring: rust/cli/src/interactive/repl_loop.rs::repl_loop (:16) — /continue handler inserted after the /compact block (:68-134) and BEFORE the provider-switch '/' catch-all (:137); grammar applied by a shared pure apply function in codelet_cli's auto_continue module so repl and NAPI setter agree; state printed via println per repl conventions.
  Session state mutated: continue_enabled/continue_budget on rust/cli/src/session/mod.rs Session (CONT-002 fields).
  """

  Background: User Story
    As a codelet agent-loop user (TUI or CLI repl)
    I want to control auto-continue with a /continue slash command in both the ratatui TUI and the CLI repl
    So that I can arm, tune, and disarm the completion contract and always see its current state

  Scenario: Bare /continue toggles auto-continue with the default budget
    Given auto-continue is off
    When the user enters "/continue"
    Then auto-continue turns on with budget 10 and the new state is printed
    And entering "/continue" again turns auto-continue off and prints the new state

  Scenario: /continue with a numeric budget arms or updates the budget
    Given auto-continue is off
    When the user enters "/continue 50"
    Then auto-continue turns on with budget 50
    And entering "/continue 25" while auto-continue is on keeps it on and only updates the budget to 25

  Scenario: /continue on and /continue off set the state explicitly
    Given any auto-continue state
    When the user enters "/continue on"
    Then auto-continue is on with the default budget
    And entering "/continue off" turns auto-continue off

  Scenario: /continue 0 is rejected with a hint
    Given auto-continue is on with budget 10
    When the user enters "/continue 0"
    Then the command is rejected with the hint "use /continue off"
    And the auto-continue state is unchanged

  Scenario: An invalid /continue argument leaves state unchanged
    Given auto-continue is off
    When the user enters "/continue banana"
    Then an error message is printed
    And the auto-continue state is unchanged

  Scenario: The TUI exposes /continue via the palette and typed input
    Given the TUI slash command registry
    When the user opens the palette or types a continue command
    Then the palette lists a continue entry
    And typing "/continue 50" is parsed as a continue subcommand rather than a provider switch or plain prompt

  Scenario: The status bar shows an auto-continue indicator while armed
    Given auto-continue is armed with budget 10 and 3 nudges used
    When the status bar renders
    Then the status indicator renders "⏩ auto-continue (3/10)"
    And no indicator is rendered while auto-continue is off
