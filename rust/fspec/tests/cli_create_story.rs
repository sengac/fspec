//! CLI surface for the `create-story` subcommand on the standalone fspec
//! Rust binary — RPC-214.
//!
//! Feature: spec/features/create-story-cli-subcommand.feature
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

fn run_create_story(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("create-story");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec create-story");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn spec_dir(cwd: &Path) -> std::path::PathBuf {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    spec
}

fn write_foundation(cwd: &Path) {
    let spec = spec_dir(cwd);
    fs::write(spec.join("foundation.json"), r#"{"version":"2.0.0"}"#)
        .expect("write foundation.json");
}

fn write_prefixes(cwd: &Path, prefixes: &[&str]) {
    let spec = spec_dir(cwd);
    let mut obj = serde_json::Map::new();
    for p in prefixes {
        obj.insert(
            (*p).to_string(),
            serde_json::json!({"prefix": p, "description": format!("{p} features"), "createdAt": "2026-06-01T00:00:00.000Z"}),
        );
    }
    let data = serde_json::json!({"prefixes": serde_json::Value::Object(obj)});
    fs::write(
        spec.join("prefixes.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write prefixes.json");
}

fn write_epics(cwd: &Path, epics: &[&str]) {
    let spec = spec_dir(cwd);
    let mut obj = serde_json::Map::new();
    for e in epics {
        obj.insert(
            (*e).to_string(),
            serde_json::json!({"id": e, "title": format!("title {e}"), "createdAt": "2026-06-01T00:00:00.000Z"}),
        );
    }
    let data = serde_json::json!({"epics": serde_json::Value::Object(obj)});
    fs::write(
        spec.join("epics.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write epics.json");
}

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn read_epics(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/epics.json")).expect("read epics.json");
    serde_json::from_str(&raw).expect("parse epics.json")
}

const TS_HELP_FIXTURE_CS: &str = include_str!("fixtures/help/create-story.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create-story --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_create_story_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec create-story --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("create-story")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn create-story --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "create-story --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/create-story.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CS);

    // @step And stdout starts with a blank line followed by 'CREATE-STORY'
    assert!(stdout.starts_with("\nCREATE-STORY\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes create-story with positional args and the three optional flags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_create_story_with_args_and_flags() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec create-story --help`
    let output = Command::new(fspec_bin())
        .arg("create-story")
        .arg("--help")
        .output()
        .expect("spawn create-story --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0);

    // @step And stdout mentions the `<prefix>` argument
    assert!(stdout.contains("prefix"), "got:\n{stdout}");

    // @step And stdout mentions the `<title>` argument
    assert!(stdout.contains("title"), "got:\n{stdout}");

    // @step And stdout advertises the `--description` flag (or its `-d` short form)
    assert!(
        stdout.contains("--description") || stdout.contains("-d"),
        "got:\n{stdout}"
    );

    // @step And stdout advertises the `--epic` flag (or its `-e` short form)
    assert!(
        stdout.contains("--epic") || stdout.contains("-e"),
        "got:\n{stdout}"
    );

    // @step And stdout advertises the `--parent` flag (or its `-p` short form)
    assert!(
        stdout.contains("--parent") || stdout.contains("-p"),
        "got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a minimal story and prints the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_minimal_story_and_prints_success_block() {
    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefixes(ws.path(), &["AUTH"]);

    // @step When I run `./rust/target/release/fspec create-story AUTH "User login"` in that tempdir
    let (code, stdout, stderr) = run_create_story(ws.path(), &["AUTH", "User login"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created story AUTH-001'
    assert!(
        stdout.lines().any(|l| l == "✓ Created story AUTH-001"),
        "missing checkmark line; got:\n{stdout}"
    );

    // @step And stdout contains the line '  Title: User login'
    assert!(
        stdout.lines().any(|l| l == "  Title: User login"),
        "missing title line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk contains a work unit AUTH-001 with type='story'
    let v = read_work_units(ws.path());
    assert_eq!(v["workUnits"]["AUTH-001"]["type"].as_str(), Some("story"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a story with -e epic and includes the Epic line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_story_with_epic_flag() {
    // @step Given a project root tempdir with spec/foundation.json, spec/prefixes.json registering prefix AUTH, and spec/epics.json containing epic 'auth'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefixes(ws.path(), &["AUTH"]);
    write_epics(ws.path(), &["auth"]);

    // @step When I run `./rust/target/release/fspec create-story AUTH "User login" -e auth` in that tempdir
    let (code, stdout, stderr) = run_create_story(ws.path(), &["AUTH", "User login", "-e", "auth"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created story AUTH-001'
    assert!(
        stdout.lines().any(|l| l == "✓ Created story AUTH-001"),
        "got:\n{stdout}"
    );

    // @step And stdout contains the line '  Epic: auth'
    assert!(
        stdout.lines().any(|l| l == "  Epic: auth"),
        "got:\n{stdout}"
    );

    // @step And spec/epics.json on disk shows epic 'auth' workUnits contains 'AUTH-001'
    let e = read_epics(ws.path());
    let work_units = e["epics"]["auth"]["workUnits"]
        .as_array()
        .expect("workUnits array");
    assert!(work_units.iter().any(|x| x.as_str() == Some("AUTH-001")));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unregistered prefix with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unregistered_prefix_with_exit_1() {
    // @step Given a project root tempdir with spec/foundation.json present and an empty spec/prefixes.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefixes(ws.path(), &[]);

    // @step When I run `./rust/target/release/fspec create-story NOPE "User login"` in that tempdir
    let (code, stdout, stderr) = run_create_story(ws.path(), &["NOPE", "User login"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "got:\n{stderr}");

    // @step And stderr contains the substring "Prefix 'NOPE' is not registered"
    assert!(
        stderr.contains("Prefix 'NOPE' is not registered"),
        "got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a missing foundation with exit 1 and the foundation-missing message
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_missing_foundation_with_exit_1() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec create-story AUTH "User login"` in that tempdir
    let (code, stdout, stderr) = run_create_story(ws.path(), &["AUTH", "User login"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Project foundation not found'
    assert!(
        stderr.contains("Project foundation not found"),
        "got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/foundation.json present and spec/prefixes.json registering prefix AUTH
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path());
    write_prefixes(ws.path(), &["AUTH"]);

    // @step When I dispatch create-story via fspec_core::dispatch::dispatch_command with prefix='AUTH' title='First'
    let req = codelet_fspec_core::DispatchRequest {
        command: "create-story".to_string(),
        args_json: r#"{"prefix":"AUTH","title":"First"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `./rust/target/release/fspec create-story AUTH "Second"` afterwards exits 0
    let (code, stdout, stderr) = run_create_story(ws.path(), &["AUTH", "Second"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk contains both 'AUTH-001' and 'AUTH-002'
    let v = read_work_units(ws.path());
    assert!(
        v["workUnits"].get("AUTH-001").is_some(),
        "AUTH-001 must exist"
    );
    assert!(
        v["workUnits"].get("AUTH-002").is_some(),
        "AUTH-002 must exist"
    );

    // @step And the CLI bridge module rust/fspec/src/create_story.rs contains NO inline foundation check, prefix validation, id generation, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/create_story.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/create_story.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "is not registered",
        "Maximum nesting depth",
        "prefixCounters",
        "write_json_atomic",
        "check_foundation_exists",
        "padStart",
        "{:03}",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
