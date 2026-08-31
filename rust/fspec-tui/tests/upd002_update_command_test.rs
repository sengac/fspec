//! UPD-002 — `/update` TUI slash command.
//!
//! Feature: spec/features/in-place-self-update-tui-command.feature
//!
//! Mirrors the CONT-002 `/continue` test shape: parser grammar, slash_parser
//! routing, palette registry entry, message formatting, and source-shape
//! invariants (no stdin prompt, no self-restart). The dispatch handler calls
//! the shared `codelet_fspec_core::update` engine (rule [0]).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_fspec_core::update::UpdateOutcome;
use codelet_fspec_tui::app::slash_parser::{parse_slash_command, SlashCommandParse};
use codelet_fspec_tui::app::update_parser::{
    format_update_message, parse_update_command, UpdateSubcommand,
};
use codelet_fspec_tui::views::agent::slash_commands::SLASH_COMMANDS;

fn dispatch_source() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("src/app/dispatch_slash_update.rs");
    std::fs::read_to_string(&path).expect("read dispatch_slash_update.rs")
}

// =============================================================================
// Scenario: /update reports up-to-date when already on the latest release
// =============================================================================

#[test]
fn tui_update_reports_up_to_date_when_already_on_the_latest_release() {
    // @step Given fspec is running at the latest released version
    let current = "0.10.0";

    // @step When the user enters "/update"
    let parsed = parse_update_command("/update");
    assert_eq!(parsed, UpdateSubcommand::CheckAndUpdate);

    // @step Then the TUI shows a message that fspec is up to date
    let msg = format_update_message(
        current,
        &UpdateOutcome::UpToDate {
            version: "0.10.0".into(),
        },
    );
    assert!(
        msg.contains("up to date"),
        "up-to-date message must say 'up to date', got: {msg}"
    );

    // @step And the installed binary is unchanged
    // (UpToDate outcome is a no-op in the engine — verified in
    //  rust/fspec-core/tests/upd002_update_engine.rs)
}

// =============================================================================
// Scenario: /update installs the latest release in place
// =============================================================================

#[test]
fn tui_update_installs_the_latest_release_in_place() {
    // @step Given fspec is running at an older version
    let current = "0.9.3";

    // @step And a newer release exists on GitHub with an asset for the current platform
    // (mock-server precondition — verified in the engine test)

    // @step When the user enters "/update"
    let parsed = parse_update_command("/update");
    assert_eq!(parsed, UpdateSubcommand::CheckAndUpdate);

    // @step Then the TUI shows a checking line while the release is looked up
    // (the dispatch handler pushes a checking line before spawning the task)
    let src = dispatch_source();
    assert!(
        src.contains("checking for latest release"),
        "dispatch must emit a checking line before spawning the update task"
    );

    // @step And the TUI shows a success line naming the new version and instructing the user to restart fspec
    let msg = format_update_message(
        current,
        &UpdateOutcome::Updated {
            version: "0.10.0".into(),
            restart_required: true,
        },
    );
    assert!(
        msg.contains("0.10.0"),
        "success line must name the new version, got: {msg}"
    );
    assert!(
        msg.to_lowercase().contains("restart"),
        "success line must instruct the user to restart, got: {msg}"
    );
}

// =============================================================================
// Scenario: /update fails safely with no network
// =============================================================================

#[test]
fn tui_update_fails_safely_with_no_network() {
    // @step Given fspec is running at an older version
    let current = "0.9.3";

    // @step And the network is unreachable
    // (closed-port precondition — verified in the engine test)

    // @step When the user enters "/update"
    let parsed = parse_update_command("/update");
    assert_eq!(parsed, UpdateSubcommand::CheckAndUpdate);

    // @step Then the TUI shows an error line describing the failure
    let msg = format_update_message(
        current,
        &UpdateOutcome::Failed {
            message: "no network / GitHub API unreachable".into(),
        },
    );
    assert!(
        msg.to_lowercase().contains("error"),
        "error line must be prefixed with 'error', got: {msg}"
    );
    assert!(
        msg.contains("unreachable"),
        "error line must describe the failure, got: {msg}"
    );

    // @step And the installed binary is unchanged
    // (Failed outcome never replaces the binary — engine contract)

    // @step And fspec keeps working at its current version
    // (the TUI session is unaffected — the update runs as a spawned task)
}

// =============================================================================
// Scenario: /update never prompts for confirmation
// =============================================================================

#[test]
fn tui_update_never_prompts_for_confirmation() {
    // @step Given fspec is running in the TUI with the terminal in raw mode
    // (TUI precondition)

    // @step And a newer release exists on GitHub
    // (mock-server precondition — verified in the engine test)

    // @step When the user enters "/update"
    let src = dispatch_source();

    // @step Then the update proceeds without blocking on a stdin yes/no prompt
    assert!(
        !src.contains("std::io::stdin") && !src.contains("read_line"),
        "dispatch_slash_update.rs must not read stdin (no interactive prompt)"
    );
    assert!(
        !src.contains("confirm") && !src.contains("yes/no"),
        "dispatch_slash_update.rs must not prompt for confirmation"
    );
}

// =============================================================================
// Scenario: /update does not restart the running TUI
// =============================================================================

#[test]
fn tui_update_does_not_restart_the_running_tui() {
    // @step Given fspec is running in the TUI
    // (TUI precondition)

    // @step And a newer release exists on GitHub
    // (mock-server precondition — verified in the engine test)

    // @step When the user enters "/update" and the update succeeds
    let src = dispatch_source();

    // @step Then the running TUI session continues without interruption
    assert!(
        !src.contains("std::process::exit") && !src.contains("std::process::Command"),
        "dispatch_slash_update.rs must not re-exec or exit the TUI process"
    );

    // @step And the new version activates on the next fspec launch
    // (the success message instructs a manual restart — asserted in the
    //  installs-the-latest-release test above)
}

// =============================================================================
// Wiring: /update is routed and listed in the palette
// =============================================================================

#[test]
fn tui_exposes_update_via_the_palette_and_typed_input() {
    // @step Given the TUI slash command registry
    let registry = SLASH_COMMANDS;

    // @step When the user opens the palette or types an update command
    let palette_entry = registry.iter().find(|c| c.name() == "update");
    let typed = parse_slash_command("/update");

    // @step Then the palette lists an update entry
    assert!(
        palette_entry.is_some(),
        "SLASH_COMMANDS must contain an 'update' entry"
    );

    // @step And typing "/update" is parsed as an update subcommand rather than a plain prompt
    assert!(
        matches!(typed, SlashCommandParse::UpdateSubcommand(_)),
        "/update must be routed as UpdateSubcommand, got: {typed:?}"
    );
}
