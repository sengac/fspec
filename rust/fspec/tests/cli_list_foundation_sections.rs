//! CLI surface for the `list-foundation-sections` subcommand on the
//! standalone fspec Rust binary — RPC-246.
//!
//! Feature: spec/features/list-foundation-sections-cli-subcommand.feature
//!
//! Red phase: this test MUST fail today because `intercept_ts_help` in
//! `rust/fspec/src/main.rs` does not yet route `list-foundation-sections`,
//! so clap emits its default `--help` block which differs from the TS
//! Commander.js reference output.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-foundation-sections --help is byte-for-byte identical
//           to TS Commander.js reference output
// ─────────────────────────────────────────────────────────────────────────

/// Captured byte-exact TS reference output of
/// `node dist/index.js list-foundation-sections --help` piped to non-TTY.
/// Regenerate via:
///   `node /Users/rquast/projects/fspec/dist/index.js list-foundation-sections --help \
///    > rust/fspec/tests/fixtures/help/list-foundation-sections.txt`
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/list-foundation-sections.txt");

#[test]
fn scenario_list_foundation_sections_help_matches_ts_commander_default() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec list-foundation-sections --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-foundation-sections")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-foundation-sections --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-foundation-sections --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And the TS reference binary `node dist/index.js list-foundation-sections --help` produces a 6-line block: Usage line, blank, description, blank, Options header, --format and -h lines
    // (Fixture captured at rust/fspec/tests/fixtures/help/list-foundation-sections.txt — 7 lines including final newline.)

    // @step And stdout is byte-for-byte identical to the TS reference output
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "list-foundation-sections --help output must be byte-for-byte identical to TS reference"
    );

    // @step And stdout starts with the line `Usage: fspec list-foundation-sections [options]`
    assert!(
        stdout.starts_with("Usage: fspec list-foundation-sections [options]\n"),
        "help must start with TS Commander.js Usage line; got first 60 bytes:\n{:?}",
        &stdout.chars().take(60).collect::<String>()
    );

    // @step And stdout contains the line `  --format <format>  Output format: text (default) or json (default: "text")`
    assert!(
        stdout.contains(
            "  --format <format>  Output format: text (default) or json (default: \"text\")\n"
        ),
        "help must contain TS --format option line"
    );

    // @step And stdout contains the line `  -h, --help         Display help for command`
    assert!(
        stdout.contains("  -h, --help         Display help for command\n"),
        "help must contain TS -h, --help line"
    );
}
