//! CLI surface for the `add-rule` subcommand on the standalone fspec
//! Rust binary — RPC-189.
//!
//! Feature: spec/features/add-rule-cli-subcommand.feature
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

fn run_add_rule(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-rule");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-rule");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_unit(id: &str, status: &str) -> String {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<serde_json::Value> = if *st == status {
            vec![serde_json::Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), serde_json::Value::Array(arr));
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_AR: &str = include_str!("fixtures/help/add-rule.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_rule_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-rule --help`
    let output = Command::new(fspec_bin())
        .arg("add-rule")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-rule --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "add-rule --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-rule.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AR);

    // @step And stdout starts with a blank line followed by 'ADD-RULE'
    assert!(stdout.starts_with("\nADD-RULE\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully appends a rule and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_appends_rule_and_prints_success_line() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I run `fspec add-rule AUTH-001 "Email must be valid format"` in that tempdir
    let (code, stdout, stderr) =
        run_add_rule(ws.path(), &["AUTH-001", "Email must be valid format"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Rule added successfully'
    assert!(
        stdout.contains("✓ Rule added successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.rules has length 1
    let v = read_work_units(ws.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules array");
    assert_eq!(rules.len(), 1);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].text='Email must be valid format'
    assert_eq!(
        rules[0]["text"].as_str(),
        Some("Email must be valid format")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a non-specifying status with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_non_specifying_status_with_exit_1_and_error_prefix() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "backlog"));
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec add-rule AUTH-001 "Anything"` in that tempdir
    let (code, _stdout, stderr) = run_add_rule(ws.path(), &["AUTH-001", "Anything"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add rule:'
    assert!(
        stderr.contains("✗ Failed to add rule:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        stderr.contains("Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "stderr must contain canonical phase-guard message; got:\n{stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-rule via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' rule='R1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-rule".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","rule":"R1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-rule AUTH-001 "R2"` afterwards exits 0
    let (code, stdout, stderr) = run_add_rule(ws.path(), &["AUTH-001", "R2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.rules has length 2
    let v = read_work_units(ws.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules array");
    assert_eq!(rules.len(), 2);

    // @step And the CLI bridge module codelet/fspec/src/add_rule.rs contains NO inline rule construction, status guard, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_rule.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_rule.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The bridge IS allowed to print the success line (TS-parity: `output.log('✓ Rule added
    // successfully')` lives in the action callback at `src/commands/add-rule.ts:79`, NOT in
    // the `addRule()` function which returns `{success, ruleCount}` JSON). The Rust port
    // mirrors that asymmetry — core::add_rule::run returns the JSON payload (for the
    // LLM-facing dispatcher), the bridge renders the success line (for the shell user).
    // Every OTHER literal below would constitute domain-logic duplication and is forbidden.
    for forbidden in [
        "RuleItem",
        "nextRuleId",
        "ensure_work_units_file",
        "write_json_atomic",
        "Can only add rules",
        "does not exist",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
