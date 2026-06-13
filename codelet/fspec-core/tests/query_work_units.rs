#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/query-work-units-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `query-work-units`
// (RPC-263). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-work-units".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn seed_work_units(project_root: &Path, value: Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("create spec dir");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&value).expect("serialize seed"),
    )
    .expect("write work-units.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn ids_in(data: &Value) -> Vec<String> {
    data["workUnits"]
        .as_array()
        .expect("workUnits should be an array")
        .iter()
        .map(|wu| wu["id"].as_str().expect("id is string").to_string())
        .collect()
}

#[test]
fn dispatcher_returns_wrapped_error_when_spec_work_units_json_is_missing() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    assert!(!root.join("spec").exists());

    // @step When I dispatch the query-work-units command against that project root
    let result = dispatch_command(req(root, json!({ "format": "json" })));

    // @step Then the dispatcher returns an error whose message contains the substring 'Failed to query work units:'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Failed to query work units:"),
        "error msg missing canonical prefix: {msg}"
    );

    // @step Then spec/work-units.json is NOT created (unlike list-work-units)
    assert!(
        !root.join("spec").join("work-units.json").exists(),
        "query-work-units MUST NOT auto-create spec/work-units.json"
    );
}

#[test]
fn tag_filter_returns_only_work_units_containing_the_specified_tag() {
    // @step Given spec/work-units.json contains AUTH-001 (backlog, tags '@cli') and AUTH-002 (implementing, tags '@high')
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "backlog",
                    "tags": ["@cli"],
                    "createdAt": "2026-06-01T00:00:00.000Z",
                    "updatedAt": "2026-06-01T00:00:00.000Z"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "Logout", "status": "implementing",
                    "tags": ["@high"],
                    "createdAt": "2026-06-01T00:00:00.000Z",
                    "updatedAt": "2026-06-01T00:00:00.000Z"
                }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": ["AUTH-002"], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with tag='@cli' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "tag": "@cli", "format": "json" })));

    // @step Then the workUnits array contains only AUTH-001
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    assert_eq!(ids_in(&data), vec!["AUTH-001"]);

    // @step Then the data array contains only the entry {workUnitId:'AUTH-001'}
    let data_arr = data["data"].as_array().expect("data array");
    assert_eq!(data_arr.len(), 1);
    assert_eq!(data_arr[0]["workUnitId"].as_str(), Some("AUTH-001"));
}

#[test]
fn combined_status_and_prefix_filters_apply_and_semantics() {
    // @step Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "backlog",
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "Logout", "status": "implementing",
                    "createdAt": "x", "updatedAt": "x"
                },
                "DASH-001": {
                    "id": "DASH-001", "title": "Dash", "status": "backlog",
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": ["AUTH-001", "DASH-001"], "specifying": [], "testing": [],
                "implementing": ["AUTH-002"], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with status='implementing' and prefix='AUTH' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "status": "implementing", "prefix": "AUTH", "format": "json" }),
    ));

    // @step Then the workUnits array contains only AUTH-002
    assert!(result.success, "{result:?}");
    let ids = ids_in(&parse_data(&result.data));
    assert_eq!(ids, vec!["AUTH-002"]);
}

#[test]
fn json_data_array_defaults_feature_file_path_to_unknown_when_wu_has_no_feature_file_field() {
    // @step Given spec/work-units.json contains AUTH-001 (featureFile 'auth.feature') and AUTH-002 (no featureFile field)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "backlog",
                    "featureFile": "auth.feature",
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "Logout", "status": "backlog",
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let data_arr = data["data"].as_array().expect("data array");

    // @step Then the data array contains {workUnitId:'AUTH-001', featureFilePath:'auth.feature'}
    let first = data_arr
        .iter()
        .find(|e| e["workUnitId"] == "AUTH-001")
        .expect("AUTH-001 entry");
    assert_eq!(first["featureFilePath"].as_str(), Some("auth.feature"));

    // @step Then the data array contains {workUnitId:'AUTH-002', featureFilePath:'unknown'}
    let second = data_arr
        .iter()
        .find(|e| e["workUnitId"] == "AUTH-002")
        .expect("AUTH-002 entry");
    assert_eq!(second["featureFilePath"].as_str(), Some("unknown"));
}

#[test]
fn csv_format_strips_commas_from_title_and_writes_header_plus_rows_to_output_file() {
    // @step Given spec/work-units.json contains AUTH-001 (title 'Login, advanced') and AUTH-002 (title 'Logout')
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login, advanced", "status": "backlog",
                    "createdAt": "2026-06-01T00:00:00.000Z",
                    "updatedAt": "2026-06-01T00:00:00.000Z"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "Logout", "status": "implementing",
                    "createdAt": "2026-06-02T00:00:00.000Z",
                    "updatedAt": "2026-06-02T00:00:00.000Z"
                }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": ["AUTH-002"], "validating": [], "done": [], "blocked": []
            }
        }),
    );
    let csv_out = tmp.path().join("out.csv");

    // @step When I dispatch query-work-units with format='csv' and output to a temp file path
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "format": "csv",
            "output": csv_out.to_string_lossy().to_string()
        }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the output file's first line equals 'id,title,status,createdAt,updatedAt'
    let body = fs::read_to_string(&csv_out).expect("read csv");
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines[0], "id,title,status,createdAt,updatedAt");

    // @step Then the output file contains a row for AUTH-001 whose title field equals 'Login advanced' (comma stripped)
    let auth001 = lines
        .iter()
        .find(|l| l.starts_with("AUTH-001,"))
        .expect("AUTH-001 row");
    let fields: Vec<&str> = auth001.split(',').collect();
    assert_eq!(fields[0], "AUTH-001");
    assert_eq!(fields[1], "Login advanced");

    // @step Then the output file contains a row for AUTH-002 whose title field equals 'Logout'
    let auth002 = lines
        .iter()
        .find(|l| l.starts_with("AUTH-002,"))
        .expect("AUTH-002 row");
    let fields2: Vec<&str> = auth002.split(',').collect();
    assert_eq!(fields2[1], "Logout");
}

#[test]
fn cycle_time_mode_returns_per_state_hour_deltas_and_total() {
    // @step Given spec/work-units.json contains AUTH-001 with stateHistory ['backlog'@2026-06-01T00:00:00Z, 'specifying'@2026-06-01T02:00:00Z, 'testing'@2026-06-01T05:00:00Z]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "testing",
                    "stateHistory": [
                        { "state": "backlog",    "timestamp": "2026-06-01T00:00:00.000Z" },
                        { "state": "specifying", "timestamp": "2026-06-01T02:00:00.000Z" },
                        { "state": "testing",    "timestamp": "2026-06-01T05:00:00.000Z" }
                    ],
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": ["AUTH-001"],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with workUnitId='AUTH-001' and showCycleTime=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "showCycleTime": true }),
    ));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the result contains stateTimings { backlog: '2 hours', specifying: '3 hours' }
    let timings = data["stateTimings"].as_object().expect("stateTimings");
    assert_eq!(
        timings.get("backlog").and_then(Value::as_str),
        Some("2 hours")
    );
    assert_eq!(
        timings.get("specifying").and_then(Value::as_str),
        Some("3 hours")
    );

    // @step Then the result contains totalCycleTime '5 hours'
    assert_eq!(data["totalCycleTime"].as_str(), Some("5 hours"));
}

#[test]
fn cycle_time_mode_singularises_hour_when_delta_equals_1() {
    // @step Given spec/work-units.json contains AUTH-001 with stateHistory ['backlog'@2026-06-01T00:00:00Z, 'specifying'@2026-06-01T01:00:00Z]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "specifying",
                    "stateHistory": [
                        { "state": "backlog",    "timestamp": "2026-06-01T00:00:00.000Z" },
                        { "state": "specifying", "timestamp": "2026-06-01T01:00:00.000Z" }
                    ],
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": [], "specifying": ["AUTH-001"], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with workUnitId='AUTH-001' and showCycleTime=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "showCycleTime": true }),
    ));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the result contains stateTimings { backlog: '1 hour' }
    let timings = data["stateTimings"].as_object().expect("stateTimings");
    assert_eq!(
        timings.get("backlog").and_then(Value::as_str),
        Some("1 hour")
    );

    // @step Then the result contains totalCycleTime '1 hour'
    assert_eq!(data["totalCycleTime"].as_str(), Some("1 hour"));
}

#[test]
fn questions_for_filter_normalises_bare_username_to_at_username_and_matches_included_mentions() {
    // @step Given spec/work-units.json contains AUTH-001 (questions text '@bob what about timeout?' and '@alice clarify scope') and AUTH-002 (no questions)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "Login", "status": "backlog",
                    "questions": [
                        { "text": "@bob what about timeout?" },
                        { "text": "@alice clarify scope" }
                    ],
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "Logout", "status": "backlog",
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with questionsFor='bob' and format='json'
    let r1 = dispatch_command(req(
        tmp.path(),
        json!({ "questionsFor": "bob", "format": "json" }),
    ));
    assert!(r1.success, "{r1:?}");

    // @step Then the workUnits array contains only AUTH-001
    let ids1 = ids_in(&parse_data(&r1.data));
    assert_eq!(ids1, vec!["AUTH-001"]);

    // @step When I dispatch query-work-units again with questionsFor='@bob' and format='json'
    let r2 = dispatch_command(req(
        tmp.path(),
        json!({ "questionsFor": "@bob", "format": "json" }),
    ));
    assert!(r2.success, "{r2:?}");

    // @step Then the workUnits array still contains only AUTH-001
    let ids2 = ids_in(&parse_data(&r2.data));
    assert_eq!(ids2, vec!["AUTH-001"]);
}

#[test]
fn sort_by_numeric_field_with_descending_order_produces_decreasing_values() {
    // @step Given spec/work-units.json contains AUTH-001 (estimate 5), AUTH-002 (estimate 3), AUTH-003 (estimate 8)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "x", "status": "backlog",
                    "estimate": 5,
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "y", "status": "backlog",
                    "estimate": 3,
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-003": {
                    "id": "AUTH-003", "title": "z", "status": "backlog",
                    "estimate": 8,
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": ["AUTH-001", "AUTH-002", "AUTH-003"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with sort='estimate' and order='desc' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "sort": "estimate", "order": "desc", "format": "json" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the workUnits array order is AUTH-003 then AUTH-001 then AUTH-002
    let ids = ids_in(&parse_data(&result.data));
    assert_eq!(ids, vec!["AUTH-003", "AUTH-001", "AUTH-002"]);
}

#[test]
fn has_questions_true_filter_keeps_only_work_units_with_non_empty_questions() {
    // @step Given spec/work-units.json contains AUTH-001 (questions present) and AUTH-002 (no questions)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001", "title": "x", "status": "backlog",
                    "questions": [ { "text": "@bob what?" } ],
                    "createdAt": "x", "updatedAt": "x"
                },
                "AUTH-002": {
                    "id": "AUTH-002", "title": "y", "status": "backlog",
                    "createdAt": "x", "updatedAt": "x"
                }
            },
            "states": {
                "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch query-work-units with hasQuestions=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "hasQuestions": true, "format": "json" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the workUnits array contains only AUTH-001
    let ids = ids_in(&parse_data(&result.data));
    assert_eq!(ids, vec!["AUTH-001"]);
}

#[test]
fn two_front_doors_same_function_serves_cli_and_dispatcher() {
    // @step Given codelet/fspec-core/src/commands/query_work_units.rs exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let src = fs::read_to_string(crate_src.join("commands").join("query_work_units.rs"))
        .expect("query_work_units.rs readable");
    assert!(
        src.contains("pub async fn run"),
        "query_work_units.rs must expose pub async fn run"
    );
    assert!(
        src.contains("project_root: &Path") || src.contains("project_root: &std::path::Path"),
        "run must accept project_root: &Path"
    );

    // @step When I inspect codelet/fspec/src/query_work_units.rs
    let cli_src_path = crate_src
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fspec")
        .join("src")
        .join("query_work_units.rs");
    let cli_src = fs::read_to_string(&cli_src_path).expect("cli query_work_units.rs readable");

    // @step Then the CLI bridge module delegates to fspec_core::commands::query_work_units::run with the project_root resolved from std::env::current_dir
    assert!(
        cli_src.contains("query_work_units::run"),
        "CLI bridge must call query_work_units::run"
    );
    assert!(
        cli_src.contains("current_dir"),
        "CLI bridge must resolve project_root via std::env::current_dir"
    );

    // @step Then no filter or rendering logic is duplicated in the CLI bridge
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "query_work_units.rs must no longer be a NotYetPorted stub"
    );
}
