//! CLI surface for the `remove-rule` subcommand — RPC-279.
//!
//! Feature: spec/features/remove-rule-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_remove_rule(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-rule");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-rule");
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

fn seed_with_rules(id: &str, status: &str, rules: serde_json::Value) -> String {
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
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), serde_json::Value::String(id.to_string()));
    wu.insert("title".into(), serde_json::Value::String("title".into()));
    wu.insert("type".into(), serde_json::Value::String("story".into()));
    wu.insert(
        "status".into(),
        serde_json::Value::String(status.to_string()),
    );
    wu.insert(
        "createdAt".into(),
        serde_json::Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    wu.insert(
        "updatedAt".into(),
        serde_json::Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    if !matches!(rules, serde_json::Value::Null) {
        wu.insert("rules".into(), rules);
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": { id: serde_json::Value::Object(wu) },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_RR: &str = include_str!("fixtures/help/remove-rule.txt");

#[test]
fn scenario_remove_rule_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH
    // @step When I run `fspec remove-rule --help`
    let output = Command::new(fspec_bin())
        .arg("remove-rule")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-rule --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    // @step Then the exit code is 0
    assert_eq!(code, 0, "remove-rule --help must exit 0; stderr={stderr}");
    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-rule.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RR);
    // @step And stdout starts with a blank line followed by 'REMOVE-RULE'
    assert!(stdout.starts_with("\nREMOVE-RULE\n"));
}

#[test]
fn scenario_cli_soft_deletes_rule_and_prints_canonical_success_line() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'Email must be valid',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = serde_json::json!([
        {"id": 0, "text": "Email must be valid", "deleted": false, "createdAt": "x"}
    ]);
    write_work_units(ws.path(), &seed_with_rules("AUTH-001", "specifying", rules));
    // @step When I run `fspec remove-rule AUTH-001 0` in that tempdir
    let (code, stdout, stderr) = run_remove_rule(ws.path(), &["AUTH-001", "0"]);
    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");
    // @step And stdout contains the substring '✓ Removed rule: "Email must be valid"'
    assert!(
        stdout.contains("✓ Removed rule: \"Email must be valid\""),
        "stdout must contain canonical success line; got:\n{stdout}"
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true
    let v = read_work_units(ws.path());
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["deleted"].as_bool(),
        Some(true)
    );
}

#[test]
fn scenario_cli_rejects_unknown_rule_id_with_exit_1_and_error_prefix() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = serde_json::json!([{"id": 0, "text": "r0", "deleted": false, "createdAt": "x"}]);
    write_work_units(ws.path(), &seed_with_rules("AUTH-001", "specifying", rules));
    // @step When I run `fspec remove-rule AUTH-001 99` in that tempdir
    let (code, _stdout, stderr) = run_remove_rule(ws.path(), &["AUTH-001", "99"]);
    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to remove rule:'
    assert!(
        stderr.contains("✗ Failed to remove rule:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );
    // @step And stderr contains the substring 'Rule with ID 99 not found'
    assert!(
        stderr.contains("Rule with ID 99 not found"),
        "stderr must contain canonical message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_matches_ts_nan_behaviour_when_index_is_non_numeric() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = serde_json::json!([{"id": 0, "text": "r0", "deleted": false, "createdAt": "x"}]);
    write_work_units(ws.path(), &seed_with_rules("AUTH-001", "specifying", rules));
    // @step When I run `fspec remove-rule AUTH-001 abc` in that tempdir
    let (code, _stdout, stderr) = run_remove_rule(ws.path(), &["AUTH-001", "abc"]);
    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to remove rule:'
    assert!(
        stderr.contains("✗ Failed to remove rule:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );
    // @step And stderr contains the substring 'Rule with ID NaN not found'
    assert!(
        stderr.contains("Rule with ID NaN not found"),
        "stderr must mirror TS `parseInt('abc') → NaN` path; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,...},{id:1,...}]
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = serde_json::json!([
        {"id": 0, "text": "r0", "deleted": false, "createdAt": "x"},
        {"id": 1, "text": "r1", "deleted": false, "createdAt": "x"}
    ]);
    write_work_units(ws.path(), &seed_with_rules("AUTH-001", "specifying", rules));
    // @step When I dispatch remove-rule via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-rule".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","index":0}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    // @step And running `fspec remove-rule AUTH-001 1` afterwards exits 0
    let (code, stdout, stderr) = run_remove_rule(ws.path(), &["AUTH-001", "1"]);
    assert_eq!(
        code, 0,
        "CLI remove must succeed; stdout={stdout}, stderr={stderr}"
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true and AUTH-001.rules[1].deleted=true
    let v = read_work_units(ws.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules");
    assert_eq!(rules[0]["deleted"].as_bool(), Some(true));
    assert_eq!(rules[1]["deleted"].as_bool(), Some(true));
    // @step And the CLI bridge module rust/fspec/src/remove_rule.rs contains NO inline soft-delete or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_rule.rs");
    assert!(
        bridge_path.exists(),
        "bridge must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "Rule with ID",
        "has no rules",
        "already deleted",
        "deletedAt",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}
