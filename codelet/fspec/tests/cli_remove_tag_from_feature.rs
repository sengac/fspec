//! CLI surface for the `remove-tag-from-feature` subcommand on the standalone fspec
//! Rust binary — RPC-281.
//!
//! Feature: spec/features/remove-tag-from-feature-cli-subcommand.feature
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

fn run_remove_tag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-tag-from-feature");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-tag-from-feature");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

const TS_HELP_FIXTURE_RTFF: &str = include_str!("fixtures/help/remove-tag-from-feature.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully removes a tag and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_removes_tag_and_prints_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        "@wip\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I run 'fspec remove-tag-from-feature spec/features/login.feature @wip' in that tempdir
    let (code, stdout, stderr) =
        run_remove_tag(ws.path(), &["spec/features/login.feature", "@wip"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Removed @wip from spec/features/login.feature'
    assert!(
        stdout.contains("✓ Removed @wip from spec/features/login.feature"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/features/login.feature in the tempdir does NOT contain a line whose trimmed value is '@wip'
    let after = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        !after.lines().any(|l| l.trim() == "@wip"),
        "@wip line must be removed; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects removal of a tag not on the feature with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_removal_of_tag_not_on_feature() {
    // @step Given a tempdir with spec/features/login.feature containing '@critical\nFeature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        "@critical\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I run 'fspec remove-tag-from-feature spec/features/login.feature @notthere' in that tempdir
    let (code, _stdout, stderr) =
        run_remove_tag(ws.path(), &["spec/features/login.feature", "@notthere"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Tag @notthere not found on this feature'
    assert!(
        stderr.contains("Tag @notthere not found on this feature"),
        "stderr must contain canonical absent-tag message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects missing file with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_missing_file() {
    // @step Given a tempdir with NO spec/features/missing.feature file
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run 'fspec remove-tag-from-feature spec/features/missing.feature @wip' in that tempdir
    let (code, _stdout, stderr) =
        run_remove_tag(ws.path(), &["spec/features/missing.feature", "@wip"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'File not found: spec/features/missing.feature'
    assert!(
        stderr.contains("File not found: spec/features/missing.feature"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec remove-tag-from-feature --help'
    let output = Command::new(fspec_bin())
        .arg("remove-tag-from-feature")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-tag-from-feature --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "remove-tag-from-feature --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/remove-tag-from-feature.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RTFF);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing '@wip\nFeature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        "@wip\nFeature: Login\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch remove-tag-from-feature through fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' and tags=['@wip']
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-tag-from-feature".to_string(),
        args_json: r#"{"file":"spec/features/login.feature","tags":["@wip"]}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Removed @wip from spec/features/login.feature'
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Removed @wip from spec/features/login.feature"),
        "expected canonical message; got: {msg}"
    );

    // @step And the CLI bridge module codelet/fspec/src/remove_tag_from_feature.rs contains NO inline gherkin parsing or tag-filter logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_tag_from_feature.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_tag_from_feature.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "existingTags",
        "tagsToRemove",
        "filteredLines",
        "not found on this feature",
        "Tag @",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
}
