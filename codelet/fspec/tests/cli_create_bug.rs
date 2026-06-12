//! CLI surface for the `create-bug` subcommand on the standalone fspec
//! Rust binary — RPC-210.
//!
//! Feature: spec/features/create-bug-cli-subcommand.feature
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

fn run_create_bug(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("create-bug");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec create-bug");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), "{\"version\":\"2.0.0\"}").expect("write foundation");
}

fn write_prefix(cwd: &Path, prefix: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let body = format!(
        "{{\"prefixes\":{{\"{p}\":{{\"prefix\":\"{p}\",\"description\":\"desc\",\"createdAt\":\"2026-06-01T00:00:00.000Z\"}}}}}}",
        p = prefix
    );
    fs::write(spec.join("prefixes.json"), body).expect("write prefixes");
}

fn write_epics(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_wu_value(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes create-bug with positional args and option flags in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_create_bug_with_args_and_option_flags() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec create-bug --help`
    let output = Command::new(fspec_bin())
        .arg("create-bug")
        .arg("--help")
        .output()
        .expect("spawn create-bug --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "create-bug --help must exit 0; got {code}, stderr={stderr}");

    // @step And stdout describes the create-bug subcommand
    assert!(
        stdout.contains("create-bug") || stdout.contains("CREATE-BUG"),
        "help must describe create-bug; got:\n{stdout}"
    );

    // @step And stdout mentions the `<prefix>` argument
    assert!(stdout.contains("prefix"), "help must mention prefix; got:\n{stdout}");

    // @step And stdout mentions the `<title>` argument
    assert!(stdout.contains("title"), "help must mention title; got:\n{stdout}");

    // @step And stdout advertises the `--description` flag (or its `-d` short form)
    assert!(
        stdout.contains("--description") || stdout.contains("-d"),
        "help must advertise --description; got:\n{stdout}"
    );

    // @step And stdout advertises the `--epic` flag (or its `-e` short form)
    assert!(
        stdout.contains("--epic") || stdout.contains("-e"),
        "help must advertise --epic; got:\n{stdout}"
    );

    // @step And stdout advertises the `--parent` flag (or its `-p` short form)
    assert!(
        stdout.contains("--parent") || stdout.contains("-p"),
        "help must advertise --parent; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a minimal bug and prints the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_minimal_bug_and_prints_success_block() {
    // @step Given a working directory with spec/foundation.json present and prefix 'BUG' registered
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefix(ws.path(), "BUG");

    // @step When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    let (code, stdout, stderr) = run_create_bug(ws.path(), &["BUG", "Login crash"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created bug BUG-001'
    assert!(
        stdout.lines().any(|l| l == "✓ Created bug BUG-001"),
        "missing checkmark line; got:\n{stdout}"
    );

    // @step And stdout contains the line '  Title: Login crash'
    assert!(
        stdout.lines().any(|l| l == "  Title: Login crash"),
        "missing title line; got:\n{stdout}"
    );

    // @step And the file spec/work-units.json contains work unit 'BUG-001' with type='bug'
    let data = read_wu_value(ws.path());
    assert_eq!(data["workUnits"]["BUG-001"]["type"].as_str(), Some("bug"));

    // @step And stderr contains the substring 'Bug BUG-001 created successfully.'
    assert!(
        stderr.contains("Bug BUG-001 created successfully."),
        "missing system-reminder on stderr; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a bug with description, epic, and parent printing all detail lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_bug_with_description_epic_parent() {
    // @step Given a working directory with spec/foundation.json present, prefix 'BUG' registered, an existing bug 'BUG-001', and an existing epic 'auth'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefix(ws.path(), "BUG");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "BUG-001": {"id":"BUG-001","title":"a","type":"bug","status":"backlog","createdAt":"x","updatedAt":"x","children":[]}
  },
  "states": {"backlog":["BUG-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"BUG": 1}
}"#,
    );
    write_epics(ws.path(), r#"{"epics":{"auth":{"id":"auth","title":"Authentication","createdAt":"x"}}}"#);

    // @step When I run `./codelet/target/release/fspec create-bug BUG "Login crash" -d "Crashes on submit" -e auth -p BUG-001`
    let (code, stdout, stderr) = run_create_bug(
        ws.path(),
        &["BUG", "Login crash", "-d", "Crashes on submit", "-e", "auth", "-p", "BUG-001"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created bug BUG-002'
    assert!(stdout.lines().any(|l| l == "✓ Created bug BUG-002"), "got:\n{stdout}");

    // @step And stdout contains the line '  Description: Crashes on submit'
    assert!(stdout.lines().any(|l| l == "  Description: Crashes on submit"), "got:\n{stdout}");

    // @step And stdout contains the line '  Epic: auth'
    assert!(stdout.lines().any(|l| l == "  Epic: auth"), "got:\n{stdout}");

    // @step And stdout contains the line '  Parent: BUG-001'
    assert!(stdout.lines().any(|l| l == "  Parent: BUG-001"), "got:\n{stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails when foundation is missing with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_when_foundation_missing() {
    // @step Given a working directory with no spec/foundation.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    let (code, stdout, stderr) = run_create_bug(ws.path(), &["BUG", "Login crash"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain 'Error:'; got:\n{stderr}");

    // @step And stderr contains the substring 'Project foundation not found'
    assert!(stderr.contains("Project foundation not found"), "got:\n{stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unregistered prefix with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unregistered_prefix() {
    // @step Given a working directory with spec/foundation.json present and no registered prefixes
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());

    // @step When I run `./codelet/target/release/fspec create-bug BUG "Login crash"`
    let (code, stdout, stderr) = run_create_bug(ws.path(), &["BUG", "Login crash"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "got:\n{stderr}");

    // @step And stderr contains the substring "Prefix 'BUG' is not registered"
    assert!(stderr.contains("Prefix 'BUG' is not registered"), "got:\n{stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a working directory with spec/foundation.json present and prefix 'BUG' registered
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefix(ws.path(), "BUG");

    // @step When I dispatch create-bug via fspec_core::dispatch::dispatch_command with prefix='BUG' title='First'
    let req = codelet_fspec_core::DispatchRequest {
        command: "create-bug".to_string(),
        args_json: r#"{"prefix":"BUG","title":"First"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then the dispatcher writes spec/work-units.json with 'BUG-001'
    let data = read_wu_value(ws.path());
    assert!(data["workUnits"].get("BUG-001").is_some(), "BUG-001 must be present");

    // @step And running `./codelet/target/release/fspec create-bug BUG "Second"` afterwards exits 0
    let (code, stdout, stderr) = run_create_bug(ws.path(), &["BUG", "Second"]);
    assert_eq!(code, 0, "CLI add must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/work-units.json now contains both 'BUG-001' and 'BUG-002'
    let data = read_wu_value(ws.path());
    assert!(data["workUnits"].get("BUG-001").is_some(), "BUG-001 must be present");
    assert!(data["workUnits"].get("BUG-002").is_some(), "BUG-002 must be present");

    // @step And the CLI bridge module codelet/fspec/src/create_bug.rs contains NO inline validation, id-generation, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/create_bug.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/create_bug.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "is not registered",
        "Title is required",
        "padStart",
        "prefixCounters",
        "✓ Created bug",
        "write_json_atomic",
        "Maximum nesting depth",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create-bug --help is byte-for-byte identical to the fixture
// ─────────────────────────────────────────────────────────────────────────

const HELP_FIXTURE_CB: &str = include_str!("fixtures/help/create-bug.txt");

#[test]
fn scenario_create_bug_help_matches_fixture() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec create-bug --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("create-bug")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn create-bug --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "create-bug --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/create-bug.txt
    assert_eq!(stdout, HELP_FIXTURE_CB);
}
