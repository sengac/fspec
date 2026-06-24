//! CLI surface for the `add-tag-to-feature` subcommand on the standalone fspec
//! Rust binary — RPC-193.
//!
//! Feature: spec/features/add-tag-to-feature-cli-subcommand.feature
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

fn run_add_tag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-tag-to-feature");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-tag-to-feature");
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

fn write_canonical_tags_json(project_root: &Path) {
    let body = serde_json::json!({
        "categories": [
            {"name": "Phase Tags", "description": "", "required": true, "tags": []},
            {"name": "Component Tags", "description": "", "required": true, "tags": []},
            {"name": "Feature Group Tags", "description": "", "required": true, "tags": []},
            {"name": "Technical Tags", "description": "", "required": false, "tags": []},
            {"name": "Platform Tags", "description": "", "required": false, "tags": []},
            {"name": "Priority Tags", "description": "", "required": false, "tags": []},
            {"name": "Status Tags", "description": "", "required": false, "tags": []},
            {"name": "Testing Tags", "description": "", "required": false, "tags": []},
            {"name": "Automation Tags", "description": "", "required": false, "tags": []}
        ]
    });
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write tags.json");
}

const FEATURE_LOGIN_PLAIN: &str = "Feature: Login\n  Scenario: A\n    Given x\n";

const TS_HELP_FIXTURE_ATTF: &str = include_str!("fixtures/help/add-tag-to-feature.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully adds a single tag and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_adds_single_tag_and_prints_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I run 'fspec add-tag-to-feature spec/features/login.feature @critical' in that tempdir
    let (code, stdout, stderr) =
        run_add_tag(ws.path(), &["spec/features/login.feature", "@critical"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added @critical to spec/features/login.feature'
    assert!(
        stdout.contains("✓ Added @critical to spec/features/login.feature"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/features/login.feature in the tempdir contains the line '@critical' above 'Feature: Login'
    let after = read_feature(ws.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let crit = lines
        .iter()
        .position(|l| *l == "@critical")
        .expect("@critical line");
    let feat = lines
        .iter()
        .position(|l| *l == "Feature: Login")
        .expect("Feature line");
    assert!(crit < feat, "@critical must appear above Feature header");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces invalid-format errors with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_invalid_format_errors_with_stderr_error_prefix() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I run 'fspec add-tag-to-feature spec/features/login.feature InvalidTag' in that tempdir
    let (code, _stdout, stderr) =
        run_add_tag(ws.path(), &["spec/features/login.feature", "InvalidTag"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Invalid tag format. Tags must start with @'
    assert!(
        stderr.contains("Invalid tag format. Tags must start with @"),
        "stderr must contain canonical invalid-format message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --validate-registry rejects unregistered tag with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_validate_registry_rejects_unregistered_tag() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n' and spec/tags.json carrying the canonical 9-category default
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );
    write_canonical_tags_json(ws.path());

    // @step When I run 'fspec add-tag-to-feature spec/features/login.feature @unregistered --validate-registry' in that tempdir
    let (code, _stdout, stderr) = run_add_tag(
        ws.path(),
        &[
            "spec/features/login.feature",
            "@unregistered",
            "--validate-registry",
        ],
    );

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Tag @unregistered is not registered in spec/tags.json'
    assert!(
        stderr.contains("Tag @unregistered is not registered in spec/tags.json"),
        "stderr must contain canonical registry-miss message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints consolidated system-reminder block after success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_consolidated_system_reminder_after_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n' and spec/tags.json carrying the canonical 9-category default
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );
    write_canonical_tags_json(ws.path());

    // @step When I run 'fspec add-tag-to-feature spec/features/login.feature @unknown' in that tempdir
    let (code, stdout, stderr) =
        run_add_tag(ws.path(), &["spec/features/login.feature", "@unknown"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added @unknown to spec/features/login.feature'
    assert!(
        stdout.contains("✓ Added @unknown to spec/features/login.feature"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '<system-reminder>'
    assert!(
        stdout.contains("<system-reminder>"),
        "stdout must contain reminder opener; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'is not registered in spec/tags.json'
    assert!(
        stdout.contains("is not registered in spec/tags.json"),
        "stdout must contain unregistered-tag reminder body; got:\n{stdout}"
    );

    // @step And stdout contains the substring '</system-reminder>'
    assert!(
        stdout.contains("</system-reminder>"),
        "stdout must contain reminder closer; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec add-tag-to-feature --help'
    let output = Command::new(fspec_bin())
        .arg("add-tag-to-feature")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-tag-to-feature --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "add-tag-to-feature --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-tag-to-feature.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_ATTF);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-tag-to-feature through fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' and tags=['@cli']
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-tag-to-feature".to_string(),
        args_json: r#"{"file":"spec/features/login.feature","tags":["@cli"]}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Added @cli to spec/features/login.feature'
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added @cli to spec/features/login.feature"),
        "expected canonical message; got: {msg}"
    );

    // @step And the CLI bridge module codelet/fspec/src/add_tag_to_feature.rs contains NO inline gherkin parsing, tag-validation regex, or insertion logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_tag_to_feature.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_tag_to_feature.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "WORK_UNIT_TAG_PATTERN",
        "is_work_unit_tag",
        "@[A-Z]{2,6}",
        "categories",
        "TagsData",
        "splice",
        "insertIndex",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
    // (Asserted indirectly by the forbidden-token sweep above.)
}
