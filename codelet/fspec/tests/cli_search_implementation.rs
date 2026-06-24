//! CLI surface for the `search-implementation` subcommand on the standalone
//! fspec Rust binary — RPC-296.
//!
//! Features:
//!   - spec/features/search-implementation-rust-port.feature
//!   - spec/features/search-implementation-cli-subcommand.feature
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

fn run_si(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("search-implementation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec search-implementation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "search-implementation".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Workspace with an on-disk impl file containing "loadConfig" referenced by
/// a single coverage sidecar (featureName user-login).
fn workspace_with_loadconfig(cwd: &Path) {
    write_file(cwd, "src/config.ts", "export function loadConfig() {}\n");
    let body = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        {
          "file": "test/login.test.ts",
          "lines": "1-10",
          "implMappings": [
            { "file": "src/config.ts", "lines": [1, 2, 3] }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["test/login.test.ts"],
    "implFiles": ["src/config.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(cwd, "spec/features/user-login.feature.coverage", body);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Clap exposes search-implementation as a subcommand and prints flag help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn clap_exposes_search_implementation_as_a_subcommand_and_prints_flag_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec search-implementation --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("search-implementation")
        .arg("--help")
        .output()
        .expect("spawn fspec search-implementation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'search-implementation'
    assert!(
        stdout.contains("search-implementation") || stdout.contains("SEARCH-IMPLEMENTATION"),
        "help must mention subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--function'
    assert!(
        stdout.contains("--function"),
        "help must mention --function; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI without --function flag exits non-zero
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_without_function_flag_exits_non_zero() {
    // @step Given an empty directory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec search-implementation` from that directory
    let (code, _stdout, _stderr) = run_si(ws.path(), &[]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero when --function is missing");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI default output prints the green summary line
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_output_prints_the_green_summary_line() {
    // @step Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    let ws = tempfile::tempdir().expect("tempdir");
    workspace_with_loadconfig(ws.path());

    // @step When I run `./codelet/target/release/fspec search-implementation --function loadConfig` from that workspace
    let (code, stdout, stderr) = run_si(ws.path(), &["--function", "loadConfig"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Found "loadConfig" in 1 file(s)'
    assert!(
        stdout.contains("Found \"loadConfig\" in 1 file(s)"),
        "stdout must contain summary line; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI --json prints the 2-space JSON envelope to stdout
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_json_prints_the_2_space_json_envelope_to_stdout() {
    // @step Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    let ws = tempfile::tempdir().expect("tempdir");
    workspace_with_loadconfig(ws.path());

    // @step When I run `./codelet/target/release/fspec search-implementation --function loadConfig --json` from that workspace
    let (code, stdout, stderr) = run_si(ws.path(), &["--function", "loadConfig", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as JSON with searchedFiles and files fields
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    for field in ["searchedFiles", "files"] {
        assert!(
            parsed.get(field).is_some(),
            "missing field `{field}` in:\n{stdout}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: search-implementation --help is byte-for-byte identical to the TS reference
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_SI: &str = include_str!("fixtures/help/search-implementation.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec search-implementation --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("search-implementation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn search-implementation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/search-implementation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SI);

    // @step And stdout starts with a blank line followed by 'SEARCH-IMPLEMENTATION'
    assert!(stdout.starts_with("\nSEARCH-IMPLEMENTATION\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn default_combined_tui_mode_is_preserved_when_no_subcommand_is_provided() {
    // @step Given the fspec Rust binary has search-implementation registered as a clap subcommand alongside other ported subcommands

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists search-implementation as an available subcommand
    assert!(
        help.contains("search-implementation"),
        "fspec --help must list search-implementation; got:\n{help}"
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
    // @step Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    let ws = tempfile::tempdir().expect("tempdir");
    workspace_with_loadconfig(ws.path());

    // @step When I dispatch search-implementation through fspec_core::dispatch::dispatch_command with function='loadConfig' against that workspace
    let result = dispatch(ws.path(), r#"{"function":"loadConfig"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let disp_len = parsed["files"].as_array().expect("files array").len();

    // @step And I run `./codelet/target/release/fspec search-implementation --function loadConfig --json` against the same workspace
    let (code, stdout, _stderr) = run_si(ws.path(), &["--function", "loadConfig", "--json"]);
    assert_eq!(code, 0, "CLI must exit 0");
    let cli_parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let cli_len = cli_parsed["files"].as_array().expect("files array").len();

    // @step Then both invocations produce a JSON envelope with the same files array length
    assert_eq!(
        disp_len, cli_len,
        "dispatcher and CLI files counts must match"
    );

    // @step And the CLI bridge module codelet/fspec/src/search_implementation.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/search_implementation.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/search_implementation.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Found \"",
        "extractImplementationFiles",
        "read_all_coverage_files",
        "implMappings",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
