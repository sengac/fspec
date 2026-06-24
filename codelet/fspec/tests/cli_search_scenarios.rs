//! CLI surface for the `search-scenarios` subcommand on the standalone
//! fspec Rust binary — RPC-297.
//!
//! Features:
//!   - spec/features/search-scenarios-rust-port.feature
//!   - spec/features/search-scenarios-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired and the fspec_core port
//! is implemented (Phase C), these tests exercise the binary/dispatcher
//! and expect missing-subcommand / NotYetPorted failures. Once Phase C
//! lands the green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── Helpers ─────────

fn run_ss(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("search-scenarios");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec search-scenarios");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "search-scenarios".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_feature(cwd: &Path, name: &str, body: &str) {
    let path = cwd.join("spec/features").join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write feature");
}

fn login_feature(cwd: &Path) {
    write_feature(
        cwd,
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );
}

fn validation_feature(cwd: &Path) {
    write_feature(
        cwd,
        "validation.feature",
        "@VAL-001\nFeature: Validation\n\n  Scenario: Validate user\n    Given a user\n\n  Scenario: valid email\n    Given an email\n",
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Clap exposes search-scenarios as a subcommand and prints flag help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn clap_exposes_search_scenarios_as_a_subcommand_and_prints_flag_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec search-scenarios --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("search-scenarios")
        .arg("--help")
        .output()
        .expect("spawn fspec search-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'search-scenarios'
    assert!(
        stdout.contains("search-scenarios") || stdout.contains("SEARCH-SCENARIOS"),
        "help must mention subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--query'
    assert!(
        stdout.contains("--query"),
        "help must mention --query; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI without --query flag exits non-zero
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_without_query_flag_exits_non_zero() {
    // @step Given an empty directory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec search-scenarios` from that directory
    let (code, _stdout, _stderr) = run_ss(ws.path(), &[]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero when --query is missing");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI default output prints the green summary line
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_output_prints_the_green_summary_line() {
    // @step Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    login_feature(ws.path());

    // @step When I run `./codelet/target/release/fspec search-scenarios --query Login` from that workspace
    let (code, stdout, stderr) = run_ss(ws.path(), &["--query", "Login"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Found 1 scenarios matching "Login"'
    assert!(
        stdout.contains("Found 1 scenarios matching \"Login\""),
        "stdout must contain summary line; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI --json prints the 2-space JSON envelope to stdout
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_json_prints_the_2_space_json_envelope_to_stdout() {
    // @step Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    login_feature(ws.path());

    // @step When I run `./codelet/target/release/fspec search-scenarios --query Login --json` from that workspace
    let (code, stdout, stderr) = run_ss(ws.path(), &["--query", "Login", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as JSON with searchedFiles, scenarios, format, and searchMode fields
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    for field in ["searchedFiles", "scenarios", "format", "searchMode"] {
        assert!(
            parsed.get(field).is_some(),
            "missing field `{field}` in:\n{stdout}"
        );
    }

    // @step And the JSON.searchMode field equals 'literal'
    assert_eq!(parsed["searchMode"].as_str(), Some("literal"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI --regex sets searchMode to regex
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_regex_sets_search_mode_to_regex() {
    // @step Given a temp workspace contains spec/features with scenarios named "Validate user" and "valid email"
    let ws = tempfile::tempdir().expect("tempdir");
    validation_feature(ws.path());

    // @step When I run `./codelet/target/release/fspec search-scenarios --query valid.* --regex --json` from that workspace
    let (code, stdout, stderr) = run_ss(ws.path(), &["--query", "valid.*", "--regex", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");

    // @step And the JSON.searchMode field equals 'regex'
    assert_eq!(parsed["searchMode"].as_str(), Some("regex"));

    // @step And the JSON.scenarios array has 2 elements
    assert_eq!(
        parsed["scenarios"]
            .as_array()
            .expect("scenarios array")
            .len(),
        2
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI exits non-zero on an invalid regex pattern
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_exits_non_zero_on_an_invalid_regex_pattern() {
    // @step Given a temp workspace contains spec/features with at least one feature file
    let ws = tempfile::tempdir().expect("tempdir");
    login_feature(ws.path());

    // @step When I run `./codelet/target/release/fspec search-scenarios --query [ --regex` from that workspace
    let (code, _stdout, stderr) = run_ss(ws.path(), &["--query", "[", "--regex"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero on invalid regex");

    // @step And stderr contains the substring '✗ Search failed:'
    assert!(
        stderr.contains("✗ Search failed:"),
        "stderr must contain '✗ Search failed:'; got:\n{stderr}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: search-scenarios --help is byte-for-byte identical to the TS reference
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_SS: &str = include_str!("fixtures/help/search-scenarios.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec search-scenarios --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("search-scenarios")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn search-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/search-scenarios.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SS);

    // @step And stdout starts with a blank line followed by 'SEARCH-SCENARIOS'
    assert!(stdout.starts_with("\nSEARCH-SCENARIOS\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn default_combined_tui_mode_is_preserved_when_no_subcommand_is_provided() {
    // @step Given the fspec Rust binary has search-scenarios registered as a clap subcommand alongside other ported subcommands

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists search-scenarios as an available subcommand
    assert!(
        help.contains("search-scenarios"),
        "fspec --help must list search-scenarios; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    login_feature(ws.path());

    // @step When I dispatch search-scenarios through fspec_core::dispatch::dispatch_command with query='Login' against that workspace
    let result = dispatch(ws.path(), r#"{"query":"Login"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let disp_len = parsed["scenarios"]
        .as_array()
        .expect("scenarios array")
        .len();

    // @step And I run `./codelet/target/release/fspec search-scenarios --query Login --json` against the same workspace
    let (code, stdout, _stderr) = run_ss(ws.path(), &["--query", "Login", "--json"]);
    assert_eq!(code, 0, "CLI must exit 0");
    let cli_parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let cli_len = cli_parsed["scenarios"]
        .as_array()
        .expect("scenarios array")
        .len();

    // @step Then both invocations produce a JSON envelope with the same scenarios array length
    assert_eq!(
        disp_len, cli_len,
        "dispatcher and CLI scenario counts must match"
    );

    // @step And the CLI bridge module codelet/fspec/src/search_scenarios.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/search_scenarios.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/search_scenarios.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Found ",
        "parse_feature_lenient",
        "glob_feature_files",
        "searchMode",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
