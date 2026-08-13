//! CLI surface for the `list-scenario-tags` subcommand on the standalone
//! fspec Rust binary — RPC-249.
//!
//! Feature: spec/features/list-scenario-tags-cli-subcommand.feature
//!
//! Red phase: scenarios that depend on the clap subcommand being wired
//! into `rust/fspec/src/main.rs` are `#[ignore]`d until the
//! orchestrator lands the `Mode::ListScenarioTags` variant. The
//! dispatcher-bridge delegation scenario is NOT ignored because it
//! depends only on fspec_core and the bridge module (both landed in
//! Phase 2B).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_list_scenario_tags(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-scenario-tags");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-scenario-tags");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(abs, body).expect("write feature file");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-scenario-tags as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_scenario_tags_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec list-scenario-tags --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-scenario-tags")
        .arg("--help")
        .output()
        .expect("spawn fspec list-scenario-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-scenario-tags --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the list-scenario-tags subcommand
    assert!(
        stdout.contains("list-scenario-tags") || stdout.contains("List all tags"),
        "help must describe the list-scenario-tags subcommand; got:\n{stdout}"
    );

    // @step Then stdout contains the positional placeholder "<FILE>"
    // RPC-249: intercept_ts_help now emits TS-style help (lowercase <file>) BEFORE clap.
    assert!(
        stdout.contains("<file>"),
        "help must show the required positional placeholder '<file>'; got:\n{stdout}"
    );

    // @step Then stdout contains the positional placeholder "<SCENARIO>"
    assert!(
        stdout.contains("<scenario>"),
        "help must show the required positional placeholder '<scenario>'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '--show-categories'
    assert!(
        stdout.contains("--show-categories"),
        "list-scenario-tags --help must advertise --show-categories; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-scenario-tags --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 2 when required positional arguments are missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_missing_positionals_exits_2() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec list-scenario-tags` (no positionals) from that directory
    let (code, _stdout, stderr) = run_list_scenario_tags(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-scenario-tags (no positionals) must exit 1 (Commander usage error parity); got {code}, stderr={stderr}"
    );

    // @step Then stderr names the missing required argument
    assert!(
        stderr.contains("FILE") || stderr.contains("file") || stderr.contains("required"),
        "stderr must name the missing required argument; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints tag list and exits 0 when scenario has tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_tag_list_and_exits_0() {
    // @step Given the working directory contains spec/features/user-login.feature with a Scenario 'Login with valid credentials' tagged '@smoke @critical'
    let ws = tempfile::tempdir().expect("tempdir");
    let body = "Feature: User Login\n\n  @smoke @critical\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(ws.path(), "spec/features/user-login.feature", body);

    // @step When I run `./rust/target/release/fspec list-scenario-tags spec/features/user-login.feature "Login with valid credentials"`
    let (code, stdout, stderr) = run_list_scenario_tags(
        ws.path(),
        &[
            "spec/features/user-login.feature",
            "Login with valid credentials",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-scenario-tags must exit 0 on present scenario; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring "Tags on scenario 'Login with valid credentials':"
    assert!(
        stdout.contains("Tags on scenario 'Login with valid credentials':"),
        "stdout must contain header; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "  @smoke"
    assert!(
        stdout.lines().any(|l| l == "  @smoke"),
        "stdout must contain '  @smoke' line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "  @critical"
    assert!(
        stdout.lines().any(|l| l == "  @critical"),
        "stdout must contain '  @critical' line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/features/user-login.feature has a Scenario 'Login with valid credentials' tagged '@smoke'
    let ws = tempfile::tempdir().expect("tempdir");
    let body =
        "Feature: User Login\n\n  @smoke\n  Scenario: Login with valid credentials\n    Given x\n";
    write_feature(ws.path(), "spec/features/user-login.feature", body);

    // @step When I dispatch list-scenario-tags through fspec_core::dispatch::dispatch_command with file='spec/features/user-login.feature', scenario='Login with valid credentials', and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-scenario-tags".to_string(),
        args_json: r#"{"file":"spec/features/user-login.feature","scenario":"Login with valid credentials","format":"json"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with tags array of length 1
    let arr = dispatcher_data["tags"]
        .as_array()
        .expect("tags array on root");
    assert_eq!(arr.len(), 1, "tags array length 1; got {arr:?}");
    assert_eq!(arr[0].as_str(), Some("@smoke"));

    // @step Then the CLI bridge module rust/fspec/src/list_scenario_tags.rs contains NO inline Gherkin parsing, tag accumulation, or category lookup logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_scenario_tags.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/list_scenario_tags.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Tags on scenario",
        "No tags found on this scenario",
        "Scenario Outline:",
        "Background:",
        "tags.json",
        "fs::metadata",
        "fs::read_to_string",
        "std::fs::metadata",
        "categorized_tags",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-scenario-tags --help is byte-for-byte identical to TS
//           minimal formatCommandHelp reference output (RPC-249)
// ─────────────────────────────────────────────────────────────────────────

/// Captured byte-exact TS reference output of
/// `node dist/index.js list-scenario-tags --help` piped to non-TTY.
const TS_HELP_FIXTURE_LST: &str = include_str!("fixtures/help/list-scenario-tags.txt");

#[test]
fn scenario_list_scenario_tags_help_matches_ts_minimal_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec list-scenario-tags --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-scenario-tags")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-scenario-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-scenario-tags --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the TS reference fixture at rust/fspec/tests/fixtures/help/list-scenario-tags.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE_LST,
        "list-scenario-tags --help output must be byte-for-byte identical to TS reference"
    );

    // @step And stdout starts with a blank line followed by 'LIST-SCENARIO-TAGS'
    assert!(
        stdout.starts_with("\nLIST-SCENARIO-TAGS\n"),
        "help must start with blank line then LIST-SCENARIO-TAGS header"
    );

    // @step And stdout contains '<file> (required)' and '<scenario> (required)' lines
    assert!(
        stdout.contains("<file> (required)"),
        "help must document <file> (required) argument"
    );
    assert!(
        stdout.contains("<scenario> (required)"),
        "help must document <scenario> (required) argument"
    );

    // @step And stdout does NOT contain 'WHEN TO USE' or 'NOTES' section headers
    assert!(
        !stdout.contains("WHEN TO USE"),
        "list-scenario-tags --help must NOT include WHEN TO USE section (TS config omits it)"
    );
    assert!(
        !stdout.contains("NOTES\n"),
        "list-scenario-tags --help must NOT include NOTES section (TS config omits it)"
    );
}
