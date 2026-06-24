//! CLI surface for the `create-prefix` subcommand on the standalone fspec
//! Rust binary — RPC-213.
//!
//! Feature: spec/features/create-prefix-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_create_prefix(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("create-prefix");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec create-prefix");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_prefixes(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("prefixes.json"), raw).expect("write prefixes.json");
}

fn read_prefixes_raw(cwd: &Path) -> String {
    fs::read_to_string(cwd.join("spec/prefixes.json")).expect("read prefixes.json")
}

const AUTH_ONLY_JSON: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "Auth features",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes create-prefix as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_create_prefix_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec create-prefix --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("create-prefix")
        .arg("--help")
        .output()
        .expect("spawn fspec create-prefix --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec create-prefix --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'CREATE-PREFIX'
    assert!(
        stdout.contains("CREATE-PREFIX"),
        "help must mention CREATE-PREFIX header; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '<prefix>'
    assert!(
        stdout.contains("<prefix>"),
        "help must document positional <prefix>; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '<description>'
    assert!(
        stdout.contains("<description>"),
        "help must document positional <description>; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "create-prefix --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--json'
    assert!(
        !stdout.contains("--json"),
        "create-prefix --help must NOT advertise --json; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "create-prefix --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory creates the prefixes file and prints success
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_prefixes_file_and_prints_success() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec create-prefix AUTH "Auth features"` from that directory
    let (code, stdout, stderr) = run_create_prefix(ws.path(), &["AUTH", "Auth features"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0 on valid create; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring '✓ Prefix AUTH created successfully'
    assert!(
        stdout.contains("✓ Prefix AUTH created successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step Then spec/prefixes.json now exists in the directory
    assert!(
        ws.path().join("spec/prefixes.json").exists(),
        "spec/prefixes.json must be created"
    );

    // @step Then spec/prefixes.json contains a top-level prefixes object with an AUTH key whose description is 'Auth features'
    let on_disk: serde_json::Value =
        serde_json::from_str(&read_prefixes_raw(ws.path())).expect("parse spec/prefixes.json");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["prefix"].as_str(), Some("AUTH"));
    assert_eq!(auth["description"].as_str(), Some("Auth features"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a lowercase prefix with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_lowercase_prefix() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec create-prefix auth "bad case"` from that directory
    let (code, _stdout, stderr) = run_create_prefix(ws.path(), &["auth", "bad case"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on validation failure; stderr={stderr}"
    );

    // @step Then stderr contains the substring '✗ Failed to create prefix:'
    // Parity with TS `src/commands/create-prefix.ts:83`:
    // `output.error('✗ Failed to create prefix:', err.message)`.
    assert!(
        stderr.contains("✗ Failed to create prefix:"),
        "stderr must contain TS-parity '✗ Failed to create prefix:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Prefix must be 2-6 uppercase letters'
    assert!(
        stderr.contains("Prefix must be 2-6 uppercase letters"),
        "stderr must mention the validation error; got:\n{stderr}"
    );

    // @step Then spec/prefixes.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/prefixes.json").exists(),
        "spec/prefixes.json must NOT be created on validation failure"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a duplicate prefix without touching the file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_duplicate_prefix() {
    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), AUTH_ONLY_JSON);
    let before = read_prefixes_raw(ws.path());

    // @step When I run `./codelet/target/release/fspec create-prefix AUTH "Different desc"` from that directory
    let (code, _stdout, stderr) = run_create_prefix(ws.path(), &["AUTH", "Different desc"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1 on duplicate; stderr={stderr}");

    // @step Then stderr contains the substring '✗ Failed to create prefix:'
    assert!(
        stderr.contains("✗ Failed to create prefix:"),
        "stderr must contain TS-parity '✗ Failed to create prefix:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Prefix AUTH already exists'
    assert!(
        stderr.contains("Prefix AUTH already exists"),
        "stderr must mention duplicate; got:\n{stderr}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(ws.path());
    assert_eq!(
        before, after,
        "spec/prefixes.json must be untouched after duplicate error"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces a malformed prefixes.json parse error to stderr
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_prefixes_json_exits_1_with_stderr() {
    // @step Given spec/prefixes.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_prefixes(ws.path(), "{ this is not valid json");

    // @step When I run `./codelet/target/release/fspec create-prefix AUTH "Auth features"` from that directory
    let (code, _stdout, stderr) = run_create_prefix(ws.path(), &["AUTH", "Auth features"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on malformed prefixes.json; stderr={stderr}"
    );

    // @step Then stderr contains the substring '✗ Failed to create prefix:'
    assert!(
        stderr.contains("✗ Failed to create prefix:"),
        "stderr must contain TS-parity '✗ Failed to create prefix:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse prefixes.json'
    assert!(
        stderr.contains("Failed to parse prefixes.json"),
        "stderr must mention parse error; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI missing positional argument fails with a clap usage error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_missing_positional_fails() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec create-prefix` from that directory
    let (code, _stdout, stderr) = run_create_prefix(ws.path(), &[]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "missing positional must NOT succeed");

    // @step Then stderr contains the substring 'prefix' or 'description'
    assert!(
        stderr.contains("prefix")
            || stderr.contains("description")
            || stderr.contains("PREFIX")
            || stderr.contains("DESCRIPTION"),
        "stderr must complain about the missing argument; got:\n{stderr}"
    );

    // @step Then spec/prefixes.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/prefixes.json").exists(),
        "spec/prefixes.json must NOT be created on usage error"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_create_prefix() {
    // @step Given the fspec Rust binary has create-prefix registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and create-prefix as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "create-prefix",
    ] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I dispatch create-prefix through fspec_core::dispatch::dispatch_command with prefix='AUTH' and description='Auth features'
    let req = codelet_fspec_core::DispatchRequest {
        command: "create-prefix".to_string(),
        args_json: r#"{"prefix":"AUTH","description":"Auth features"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.success is true and spec/prefixes.json now contains AUTH
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let on_disk: serde_json::Value =
        serde_json::from_str(&read_prefixes_raw(ws.path())).expect("parse on-disk");
    assert_eq!(on_disk["prefixes"]["AUTH"]["prefix"].as_str(), Some("AUTH"));

    // @step Then the CLI bridge module codelet/fspec/src/create_prefix.rs contains NO inline validation, file IO, or rendering logic — its only computation is JSON arg marshalling and stdout/stderr printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/create_prefix.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/create_prefix.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "PREFIX_REGEX",
        "[A-Z]{2,6}",
        "Prefix must be 2-6",
        "Prefix AUTH already exists",
        "ensure_prefixes_file",
        "write_json_atomic",
        "PrefixesData",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create-prefix --help is byte-for-byte identical to TS (RPC-213)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_CP: &str = include_str!("fixtures/help/create-prefix.txt");

#[test]
fn scenario_create_prefix_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec create-prefix --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("create-prefix")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn create-prefix --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "create-prefix --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-prefix.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CP);

    // @step And stdout starts with a blank line followed by 'CREATE-PREFIX'
    assert!(stdout.starts_with("\nCREATE-PREFIX\n"));
}
