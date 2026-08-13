//! CLI surface for the `check` subcommand on the standalone fspec Rust
//! binary — RPC-201.
//!
//! Feature: spec/features/check-cli-subcommand.feature
//!
//! PHASE B (TESTING): the clap subcommand + CLI bridge are not yet wired,
//! so these tests are RED until PHASE C. Each scenario maps 1:1 to a
//! Gherkin scenario; @step comments mirror the step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_check(cwd: &Path) -> (i32, String, String) {
    let output = Command::new(fspec_bin())
        .arg("check")
        .current_dir(cwd)
        .output()
        .expect("spawn fspec check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_tags(root: &Path, component: &[&str], feature_group: &[&str]) {
    let to_tags = |names: &[&str]| -> Vec<Value> {
        names
            .iter()
            .map(|n| json!({ "name": n, "description": "x" }))
            .collect()
    };
    let data = json!({
        "categories": [
            { "name": "Component Tags", "description": "", "required": true, "tags": to_tags(component) },
            { "name": "Feature Group Tags", "description": "", "required": true, "tags": to_tags(feature_group) },
            { "name": "Technical Tags", "description": "", "required": false, "tags": Vec::<Value>::new() }
        ]
    });
    write_file(
        root,
        "spec/tags.json",
        &serde_json::to_string_pretty(&data).unwrap(),
    );
}

fn valid_feature(name: &str) -> String {
    format!("@comp @grp\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/check.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes check with -v/--verbose and prints byte-parity help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_check_with_verbose_byte_parity_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec check --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("check")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec check --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "check --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/check.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout starts with a blank line followed by 'CHECK'
    assert!(stdout.starts_with("\nCHECK\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI passes and exits 0 for valid registered feature files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_passes_and_exits_0_for_valid_registered_feature_files() {
    // @step Given a project root whose spec/features holds valid feature files with registered tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_file(ws.path(), "spec/features/a.feature", &valid_feature("A"));
    write_file(ws.path(), "spec/features/b.feature", &valid_feature("B"));

    // @step When I run `./rust/target/release/fspec check` from that directory
    let (code, stdout, stderr) = run_check(ws.path());

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step Then stdout contains the substring 'Gherkin syntax: PASS'
    assert!(stdout.contains("Gherkin syntax: PASS"), "stdout={stdout}");

    // @step Then stdout contains the substring 'Tag validation: PASS'
    assert!(stdout.contains("Tag validation: PASS"), "stdout={stdout}");

    // @step Then stdout contains the substring 'All checks passed'
    assert!(stdout.contains("All checks passed"), "stdout={stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when a feature file has invalid Gherkin syntax
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_feature_has_invalid_gherkin() {
    // @step Given a project root whose spec/features holds a feature file with invalid Gherkin syntax
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_file(
        ws.path(),
        "spec/features/broken.feature",
        "this is not gherkin",
    );

    // @step When I run `./rust/target/release/fspec check` from that directory
    let (code, stdout, stderr) = run_check(ws.path());

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}, stdout={stdout}");

    // @step Then stdout contains the substring 'Gherkin syntax: FAIL'
    assert!(stdout.contains("Gherkin syntax: FAIL"), "stdout={stdout}");

    // @step Then stdout contains the substring 'Some checks failed'
    assert!(stdout.contains("Some checks failed"), "stdout={stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports the no-files case and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_no_files_case_and_exits_0() {
    // @step Given a project root with no feature files under spec/features
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec check` from that directory
    let (code, _stdout, stderr) = run_check(ws.path());

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/features holds valid feature files with registered tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_file(ws.path(), "spec/features/a.feature", &valid_feature("A"));

    // @step When I dispatch check through fspec_core::dispatch::dispatch_command
    let req = codelet_fspec_core::DispatchRequest {
        command: "check".to_string(),
        args_json: "{}".to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data reports the same gherkinStatus and tagStatus the CLI renders against the same on-disk state
    let (code, stdout, _stderr) = run_check(ws.path());
    assert_eq!(code, 0);
    assert_eq!(dispatcher_data["gherkinStatus"].as_str(), Some("PASS"));
    assert_eq!(dispatcher_data["tagStatus"].as_str(), Some("PASS"));
    assert!(stdout.contains("Gherkin syntax: PASS"));
    assert!(stdout.contains("Tag validation: PASS"));

    // @step Then the CLI bridge module rust/fspec/src/check.rs contains NO inline parsing, tag-validation, or check-aggregation logic — its only computation is JSON arg marshalling and display rendering from the envelope
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/check.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/check.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Gherkin syntax error in",
        "glob_feature_files",
        "parse_feature_lenient",
        "validate_tags",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic)"
        );
    }
}
