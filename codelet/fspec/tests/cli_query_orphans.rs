//! CLI surface for the `query-orphans` subcommand on the standalone
//! fspec Rust binary — RPC-262.
//!
//! Feature: spec/features/query-orphans-cli-subcommand.feature
//! Feature: spec/features/query-orphans-rust-port.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;

use common::fspec_bin;

fn run_query(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-orphans");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-orphans");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-orphans".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Build a work-units.json. `entries` is a slice of (id, status, optional_epic, extra_fields).
fn work_units_with(entries: &[(&str, &str, Option<&str>, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, epic, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Some(e) = epic {
            obj.insert("epic".into(), Value::String((*e).to_string()));
        }
        if let Value::Object(map) = extra {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
        wus.insert((*id).to_string(), Value::Object(obj));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────
// rust-port scenarios (dispatcher contract)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_empty_workspace_returns_empty_orphans() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the returned JSON has orphans=[]
    let data = parse_data(&result.data);
    assert_eq!(data["orphans"].as_array().map(|a| a.len()), Some(0));

    // @step And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(tmp.path().join("spec/work-units.json").exists());
}

#[test]
fn scenario_dispatcher_non_blank_epic_not_orphaned() {
    // @step Given spec/work-units.json contains A with epic='auth' and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", Some("auth"), json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans=[]
    assert_eq!(data["orphans"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_dispatcher_non_empty_blocks_not_orphaned() {
    // @step Given spec/work-units.json contains A with no epic and blocks=['X']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", None, json!({ "blocks": ["X"] }))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans=[]
    assert_eq!(data["orphans"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_dispatcher_no_epic_no_relations_is_orphaned() {
    // @step Given spec/work-units.json contains A with no epic and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", None, json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));

    // @step And that entry has status equal to A's status and suggestedActions=['Assign epic','Add relationship','Delete']
    assert_eq!(arr[0]["status"].as_str(), Some("backlog"));
    let actions: Vec<&str> = arr[0]["suggestedActions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(actions, vec!["Assign epic", "Add relationship", "Delete"]);
}

#[test]
fn scenario_dispatcher_whitespace_epic_is_treated_as_no_epic() {
    // @step Given spec/work-units.json contains A with epic='   ' and no relationship arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "backlog", Some("   "), json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));
}

#[test]
fn scenario_dispatcher_empty_relationship_arrays_treated_as_no_relationships() {
    // @step Given spec/work-units.json contains A with no epic and blocks=[] blockedBy=[] dependsOn=[] relatesTo=[]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "A",
            "backlog",
            None,
            json!({ "blocks": [], "blockedBy": [], "dependsOn": [], "relatesTo": [] }),
        )]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing exactly one entry whose id='A'
    let arr = data["orphans"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str(), Some("A"));
}

#[test]
fn scenario_dispatcher_exclude_done_filters_done_orphans() {
    // @step Given spec/work-units.json contains DONE-1 with status='done' and no epic and no relationships, and OPEN-1 with status='backlog' and no epic and no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("DONE-1", "done", None, json!({})),
            ("OPEN-1", "backlog", None, json!({})),
        ]),
    );

    // @step When I dispatch query-orphans with output='json' and excludeDone=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "output": "json", "excludeDone": false }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has orphans containing both DONE-1 and OPEN-1
    let ids: Vec<&str> = data["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    assert!(ids.contains(&"DONE-1"));
    assert!(ids.contains(&"OPEN-1"));

    // @step When I dispatch query-orphans with output='json' and excludeDone=true
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({ "output": "json", "excludeDone": true }),
    ));
    let data2 = parse_data(&result2.data);

    // @step Then the returned JSON has orphans containing only OPEN-1
    let ids2: Vec<&str> = data2["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    assert_eq!(ids2, vec!["OPEN-1"]);
}

#[test]
fn scenario_dispatcher_orphans_in_insertion_order() {
    // @step Given spec/work-units.json contains FIRST-1 then SECOND-1 then THIRD-1 all with no epic and no relationships
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("FIRST-1", "backlog", None, json!({})),
            ("SECOND-1", "backlog", None, json!({})),
            ("THIRD-1", "backlog", None, json!({})),
        ]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));
    let data = parse_data(&result.data);
    let arr = data["orphans"].as_array().expect("array");

    // @step Then orphans[0].id='FIRST-1' and orphans[1].id='SECOND-1' and orphans[2].id='THIRD-1'
    assert_eq!(arr[0]["id"].as_str(), Some("FIRST-1"));
    assert_eq!(arr[1]["id"].as_str(), Some("SECOND-1"));
    assert_eq!(arr[2]["id"].as_str(), Some("THIRD-1"));
}

#[test]
fn scenario_dispatcher_orphan_field_declaration_order() {
    // @step Given spec/work-units.json contains a single orphaned work unit ORPH-1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("ORPH-1", "backlog", None, json!({}))]),
    );

    // @step When I dispatch query-orphans with output='json'
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the orphan object's field declaration order is id, title, status, suggestedActions
    let raw = &result.data;
    let expected = ["\"id\"", "\"title\"", "\"status\"", "\"suggestedActions\""];
    let mut positions = Vec::new();
    for f in &expected {
        positions.push(raw.find(f).unwrap_or_else(|| panic!("missing {f}\n{raw}")));
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field declaration order violated: {positions:?}\nraw:\n{raw}"
        );
    }
}

#[test]
fn scenario_dispatcher_malformed_work_units_json_yields_parse_error() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch query-orphans against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "output": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success);
    let err = result.error.clone().unwrap_or_default();
    assert!(
        err.contains("Failed to parse work-units.json"),
        "expected error containing 'Failed to parse work-units.json'; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// cli-subcommand scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_query_orphans_with_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec query-orphans --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-orphans")
        .arg("--help")
        .output()
        .expect("spawn fspec query-orphans --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "fspec query-orphans --help must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'query-orphans'
    assert!(
        stdout.contains("query-orphans") || stdout.contains("QUERY-ORPHANS"),
        "help must describe the query-orphans subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_no_options_empty_workspace_prints_success() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-orphans` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the exact line '✓ No orphaned work units found.'
    assert!(
        stdout.contains("✓ No orphaned work units found."),
        "expected success line; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_output_json_empty_workspace_prints_empty_array() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-orphans --output json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--output", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout parses as JSON whose root object has orphans=[]
    let parsed: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {e}\nstdout:\n{stdout}"));
    assert_eq!(parsed["orphans"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_cli_text_output_lists_orphan_with_suggested_actions() {
    // @step Given a workspace whose spec/work-units.json contains MISC-001 with no epic and no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[("MISC-001", "backlog", None, json!({}))]),
    );

    // @step When I run `./codelet/target/release/fspec query-orphans` from that workspace
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'Found 1 orphaned work unit(s):'
    assert!(stdout.contains("Found 1 orphaned work unit(s):"), "expected header; got:\n{stdout}");
    // @step And stdout contains the substring 'MISC-001'
    assert!(stdout.contains("MISC-001"), "expected id; got:\n{stdout}");
    // @step And stdout contains the substring 'No epic or dependency relationships'
    assert!(stdout.contains("No epic or dependency relationships"), "expected warning; got:\n{stdout}");
    // @step And stdout contains the substring 'Assign epic'
    assert!(stdout.contains("Assign epic"), "expected suggested action; got:\n{stdout}");
}

#[test]
fn scenario_cli_exclude_done_flag_suppresses_done_orphans() {
    // @step Given a workspace whose spec/work-units.json contains DONE-1 with status='done' (orphaned) and OPEN-1 with status='backlog' (orphaned)
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[
            ("DONE-1", "done", None, json!({})),
            ("OPEN-1", "backlog", None, json!({})),
        ]),
    );

    // @step When I run `./codelet/target/release/fspec query-orphans --exclude-done --output json` from that workspace
    let (code, stdout, stderr) = run_query(ws.path(), &["--exclude-done", "--output", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout parses as JSON whose orphans array contains OPEN-1 and does NOT contain DONE-1
    let parsed: Value = serde_json::from_str(&stdout).expect("JSON");
    let ids: Vec<&str> = parsed["orphans"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    assert!(ids.contains(&"OPEN-1"), "expected OPEN-1; got {ids:?}");
    assert!(!ids.contains(&"DONE-1"), "DONE-1 must be filtered; got {ids:?}");
}

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./codelet/target/release/fspec query-orphans --output json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--output", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}, stderr={stderr}");
    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "expected 'Error:'; got:\n{stderr}");
    // @step And stderr contains the substring 'Failed to parse work-units.json'
    assert!(stderr.contains("Failed to parse work-units.json"), "got:\n{stderr}");
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains MISC-001 with no epic and no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[("MISC-001", "backlog", None, json!({}))]),
    );

    // @step When I dispatch query-orphans through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    let result = dispatch_command(req(ws.path(), json!({ "output": "json" })));
    assert!(result.success, "dispatcher must succeed");
    let dispatcher_data: Value = serde_json::from_str(&result.data).expect("JSON");

    // @step And I run `./codelet/target/release/fspec query-orphans --output json` against the same workspace
    let (code, stdout, _) = run_query(ws.path(), &["--output", "json"]);
    assert_eq!(code, 0, "binary path must exit 0");
    let binary_data: Value = serde_json::from_str(&stdout).expect("JSON");

    // @step Then both invocations produce JSON with orphans[0].id='MISC-001'
    assert_eq!(dispatcher_data["orphans"][0]["id"].as_str(), Some("MISC-001"));
    assert_eq!(binary_data["orphans"][0]["id"].as_str(), Some("MISC-001"));

    // @step And the CLI bridge module codelet/fspec/src/query_orphans.rs contains NO inline orphan-detection or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_orphans.rs");
    assert!(bridge_path.exists(), "bridge must exist: {}", bridge_path.display());
    let bridge_src = fs::read_to_string(&bridge_path).expect("read bridge");
    for forbidden in [
        "isOrphaned",
        "is_orphaned",
        "hasEpic",
        "has_epic",
        "Assign epic",
        "suggestedActions",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_QO: &str = include_str!("fixtures/help/query-orphans.txt");

#[test]
fn scenario_query_orphans_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec query-orphans --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-orphans")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-orphans --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/query-orphans.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QO);

    // @step And stdout starts with a blank line followed by 'QUERY-ORPHANS'
    assert!(stdout.starts_with("\nQUERY-ORPHANS\n"));
}
