#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/continue-command-surface.feature
//!
//! CONT-002: TUI side of the `/continue` command surface — grammar parser
//! (mirrors codelet_cli::interactive::auto_continue per the loop_parser
//! precedent), palette registry entry, slash_parser routing, and the
//! status-bar indicator helper.

use codelet_fspec_tui::app::continue_parser::{
    continue_status_indicator, parse_continue_command, ContinueSubcommand,
};
use codelet_fspec_tui::app::slash_parser::{parse_slash_command, SlashCommandParse};
use codelet_fspec_tui::views::agent::slash_commands::SLASH_COMMANDS;

// =============================================================================
// Scenario: Bare /continue toggles auto-continue with the default budget
// (TUI parser side — apply semantics are covered by the shared pure apply
// function in rust/cli/tests/continue_command_surface_test.rs)
// =============================================================================

#[test]
fn tui_parser_bare_continue_toggles_auto_continue_with_the_default_budget() {
    // @step Given auto-continue is off
    // (parser-level: state is applied by the dispatcher via the backend setter)

    // @step When the user enters "/continue"
    let parsed = parse_continue_command("/continue");

    // @step Then auto-continue turns on with budget 10 and the new state is printed
    assert_eq!(
        parsed,
        ContinueSubcommand::Toggle,
        "bare /continue must parse as Toggle (default budget applied downstream)"
    );

    // @step And entering "/continue" again turns auto-continue off and prints the new state
    assert_eq!(
        parse_continue_command("/continue  "),
        ContinueSubcommand::Toggle,
        "trailing whitespace still parses as Toggle"
    );
}

// =============================================================================
// Scenario: /continue with a numeric budget arms or updates the budget
// =============================================================================

#[test]
fn tui_parser_continue_with_a_numeric_budget_arms_or_updates_the_budget() {
    // @step Given auto-continue is off
    // (parser-level)

    // @step When the user enters "/continue 50"
    let parsed = parse_continue_command("/continue 50");

    // @step Then auto-continue turns on with budget 50
    assert_eq!(parsed, ContinueSubcommand::SetBudget(50));

    // @step And entering "/continue 25" while auto-continue is on keeps it on and only updates the budget to 25
    assert_eq!(
        parse_continue_command("/continue 25"),
        ContinueSubcommand::SetBudget(25)
    );
}

// =============================================================================
// Scenario: /continue on and /continue off set the state explicitly
// =============================================================================

#[test]
fn tui_parser_continue_on_and_continue_off_set_the_state_explicitly() {
    // @step Given any auto-continue state
    // (parser-level)

    // @step When the user enters "/continue on"
    let parsed_on = parse_continue_command("/continue on");

    // @step Then auto-continue is on with the default budget
    assert_eq!(parsed_on, ContinueSubcommand::On);
    assert_eq!(
        parse_continue_command("/continue ON"),
        ContinueSubcommand::On,
        "on/off matching is case-insensitive like /thinking args"
    );

    // @step And entering "/continue off" turns auto-continue off
    assert_eq!(
        parse_continue_command("/continue off"),
        ContinueSubcommand::Off
    );
}

// =============================================================================
// Scenario: /continue 0 is rejected with a hint
// =============================================================================

#[test]
fn tui_parser_continue_zero_is_rejected_with_a_hint() {
    // @step Given auto-continue is on with budget 10
    // (parser-level)

    // @step When the user enters "/continue 0"
    let parsed = parse_continue_command("/continue 0");

    // @step Then the command is rejected with the hint "use /continue off"
    assert_eq!(
        parsed,
        ContinueSubcommand::RejectZero,
        "/continue 0 must parse as RejectZero so the dispatcher emits the hint"
    );

    // @step And the auto-continue state is unchanged
    // (RejectZero never reaches the backend setter — dispatcher contract)
}

// =============================================================================
// Scenario: An invalid /continue argument leaves state unchanged
// =============================================================================

#[test]
fn tui_parser_an_invalid_continue_argument_leaves_state_unchanged() {
    // @step Given auto-continue is off
    // (parser-level)

    // @step When the user enters "/continue banana"
    let parsed = parse_continue_command("/continue banana");

    // @step Then an error message is printed
    assert_eq!(
        parsed,
        ContinueSubcommand::Invalid("banana".to_string()),
        "invalid arg must parse as Invalid carrying the trimmed arg for the error notice"
    );

    // @step And the auto-continue state is unchanged
    // (Invalid never reaches the backend setter — dispatcher contract)
}

// =============================================================================
// Scenario: The TUI exposes /continue via the palette and typed input
// =============================================================================

#[test]
fn the_tui_exposes_continue_via_the_palette_and_typed_input() {
    // @step Given the TUI slash command registry
    let registry = SLASH_COMMANDS;

    // @step When the user opens the palette or types a continue command
    let palette_entry = registry.iter().find(|c| c.name() == "continue");
    let typed = parse_slash_command("/continue 50");

    // @step Then the palette lists a continue entry
    assert!(
        palette_entry.is_some(),
        "SLASH_COMMANDS must contain a 'continue' entry"
    );

    // @step And typing "/continue 50" is parsed as a continue subcommand rather than a provider switch or plain prompt
    assert_ne!(
        typed,
        SlashCommandParse::NotASlashCommand,
        "/continue 50 must be routed as a slash command, not forwarded to send_input"
    );
}

// =============================================================================
// Scenario: The status bar shows an auto-continue indicator while armed
// =============================================================================

#[test]
fn the_status_bar_shows_an_auto_continue_indicator_while_armed() {
    // @step Given auto-continue is armed with budget 10 and 3 nudges used
    let (enabled, used, budget) = (true, 3u32, 10u32);

    // @step When the status bar renders
    let indicator = continue_status_indicator(enabled, used, budget);

    // @step Then the status indicator renders "⏩ auto-continue (3/10)"
    assert_eq!(
        indicator.as_deref(),
        Some("⏩ auto-continue (3/10)"),
        "armed indicator must render nudges_used/budget"
    );

    // @step And no indicator is rendered while auto-continue is off
    assert_eq!(
        continue_status_indicator(false, 0, 10),
        None,
        "no indicator while off"
    );
}

// =============================================================================
// CLI-side grammar + apply semantics (moved from rust/cli/tests — the spec
// workflow enforces one test file per feature file, and codelet-fspec-tui
// depends on codelet-cli, so the shared pure parse/apply API is testable here).
// =============================================================================

use codelet_cli::interactive::auto_continue::{
    apply_continue_command, parse_continue_command as cli_parse_continue_command, ContinueCommand,
    DEFAULT_CONTINUE_BUDGET,
};

// =============================================================================
// Scenario: Bare /continue toggles auto-continue with the default budget
// =============================================================================

#[test]
fn bare_continue_toggles_auto_continue_with_the_default_budget() {
    // @step Given auto-continue is off
    let (enabled, budget) = (false, DEFAULT_CONTINUE_BUDGET);

    // @step When the user enters "/continue"
    let cmd = cli_parse_continue_command("/continue");
    assert_eq!(
        cmd,
        ContinueCommand::Toggle,
        "bare /continue parses as Toggle"
    );
    let on = apply_continue_command(enabled, budget, false, &cmd);

    // @step Then auto-continue turns on with budget 10 and the new state is printed
    assert!(on.enabled, "toggle from off must turn on");
    assert_eq!(on.budget, 10, "toggle-on uses the default budget 10");
    assert!(on.changed);
    assert!(
        on.message.contains("on") && on.message.contains("10"),
        "state message must print on + budget; got: {:?}",
        on.message
    );

    // @step And entering "/continue" again turns auto-continue off and prints the new state
    let off = apply_continue_command(
        on.enabled,
        on.budget,
        false,
        &cli_parse_continue_command("/continue"),
    );
    assert!(!off.enabled, "toggle from on must turn off");
    assert!(off.changed);
    assert!(
        off.message.contains("off"),
        "state message must print off; got: {:?}",
        off.message
    );
}

// =============================================================================
// Scenario: /continue with a numeric budget arms or updates the budget
// =============================================================================

#[test]
fn continue_with_a_numeric_budget_arms_or_updates_the_budget() {
    // @step Given auto-continue is off
    let (enabled, budget) = (false, DEFAULT_CONTINUE_BUDGET);

    // @step When the user enters "/continue 50"
    let cmd = cli_parse_continue_command("/continue 50");
    assert_eq!(
        cmd,
        ContinueCommand::SetBudget(50),
        "/continue 50 parses as SetBudget(50)"
    );
    let armed = apply_continue_command(enabled, budget, false, &cmd);

    // @step Then auto-continue turns on with budget 50
    assert!(armed.enabled, "/continue 50 from off must arm");
    assert_eq!(armed.budget, 50);
    assert!(armed.changed);

    // @step And entering "/continue 25" while auto-continue is on keeps it on and only updates the budget to 25
    let updated = apply_continue_command(
        armed.enabled,
        armed.budget,
        false,
        &cli_parse_continue_command("/continue 25"),
    );
    assert!(updated.enabled, "budget update while on must stay on");
    assert_eq!(updated.budget, 25, "budget must update to 25");
    assert!(updated.changed);
}

// =============================================================================
// Scenario: /continue on and /continue off set the state explicitly
// =============================================================================

#[test]
fn continue_on_and_continue_off_set_the_state_explicitly() {
    // @step Given any auto-continue state
    for (enabled, budget) in [(false, 10u32), (true, 50u32)] {
        // @step When the user enters "/continue on"
        let cmd_on = cli_parse_continue_command("/continue on");
        assert_eq!(cmd_on, ContinueCommand::On, "/continue on parses as On");
        let on = apply_continue_command(enabled, budget, false, &cmd_on);

        // @step Then auto-continue is on with the default budget
        assert!(
            on.enabled,
            "explicit on must enable (from enabled={enabled})"
        );
        assert_eq!(
            on.budget, DEFAULT_CONTINUE_BUDGET,
            "explicit on uses the default budget"
        );

        // @step And entering "/continue off" turns auto-continue off
        let cmd_off = cli_parse_continue_command("/continue off");
        assert_eq!(cmd_off, ContinueCommand::Off, "/continue off parses as Off");
        let off = apply_continue_command(on.enabled, on.budget, false, &cmd_off);
        assert!(!off.enabled, "explicit off must disable");
    }
}

// =============================================================================
// Scenario: /continue 0 is rejected with a hint
// =============================================================================

#[test]
fn continue_zero_is_rejected_with_a_hint() {
    // @step Given auto-continue is on with budget 10
    let (enabled, budget) = (true, 10u32);

    // @step When the user enters "/continue 0"
    let cmd = cli_parse_continue_command("/continue 0");
    assert_eq!(
        cmd,
        ContinueCommand::RejectZero,
        "/continue 0 parses as RejectZero"
    );
    let result = apply_continue_command(enabled, budget, false, &cmd);

    // @step Then the command is rejected with the hint "use /continue off"
    assert!(
        result.message.contains("use /continue off"),
        "rejection must hint 'use /continue off'; got: {:?}",
        result.message
    );

    // @step And the auto-continue state is unchanged
    assert!(!result.changed);
    assert!(result.enabled, "state must remain on");
    assert_eq!(result.budget, 10, "budget must remain 10");
}

// =============================================================================
// Scenario: An invalid /continue argument leaves state unchanged
// =============================================================================

#[test]
fn an_invalid_continue_argument_leaves_state_unchanged() {
    // @step Given auto-continue is off
    let (enabled, budget) = (false, DEFAULT_CONTINUE_BUDGET);

    // @step When the user enters "/continue banana"
    let cmd = cli_parse_continue_command("/continue banana");
    assert_eq!(
        cmd,
        ContinueCommand::Invalid("banana".to_string()),
        "/continue banana parses as Invalid carrying the trimmed arg"
    );
    let result = apply_continue_command(enabled, budget, false, &cmd);

    // @step Then an error message is printed
    assert!(
        !result.message.is_empty(),
        "invalid arg must produce an error message"
    );

    // @step And the auto-continue state is unchanged
    assert!(!result.changed);
    assert!(!result.enabled);
    assert_eq!(result.budget, DEFAULT_CONTINUE_BUDGET);
}

// =============================================================================
// Scenario: The CLI repl handles /continue before the provider-switch catch-all
// =============================================================================

#[test]
fn the_cli_repl_handles_continue_before_the_provider_switch_catch_all() {
    // @step Given the CLI repl input handling
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cli/src/interactive/repl_loop.rs"
    ))
    .expect("repl_loop.rs must be readable");

    // @step When the user enters a continue command at the repl prompt
    // (source-shape: the typed input path is repl_loop's line dispatch)

    // @step Then "/continue" input is handled by the continue handler
    let handler_pos = source.find("/continue").unwrap_or_else(|| {
        panic!("repl_loop.rs must contain a /continue handler");
    });

    // @step And it is handled before the provider-switch catch-all for "/" prefixed input
    let catch_all_pos = source
        .find("input.starts_with('/')")
        .expect("provider-switch catch-all must exist");
    assert!(
        handler_pos < catch_all_pos,
        "/continue handler (byte {handler_pos}) must precede the provider-switch \
         catch-all (byte {catch_all_pos})"
    );
}
