#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/list-work-units-rust-port.feature
// Feature: spec/features/fspec-dispatcher-tokio-nesting-safety.feature  (RPC-327)
//
// This test file validates the acceptance criteria for the Rust port of the
// `list-work-units` fspec command (RPC-253) AND the RPC-327 regression that
// `dispatch_command` MUST be safe to call from inside an active tokio runtime
// (the agent loop's `#[tokio::main]`). Each scenario maps to exactly one
// #[test] function below with @step comments mirroring the Gherkin.

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-work-units".to_string(),
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

fn three_unit_store() -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001",
                "title": "Login feature",
                "status": "backlog",
                "epic": "ux",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            },
            "AUTH-002": {
                "id": "AUTH-002",
                "title": "Logout feature",
                "status": "implementing",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            },
            "DASH-001": {
                "id": "DASH-001",
                "title": "User dashboard",
                "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["AUTH-001", "DASH-001"],
            "specifying": [],
            "testing": [],
            "implementing": ["AUTH-002"],
            "validating": [],
            "done": [],
            "blocked": []
        }
    })
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data).unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn ids_in(data: &Value) -> Vec<String> {
    data["workUnits"]
        .as_array()
        .expect("workUnits should be an array")
        .iter()
        .map(|wu| wu["id"].as_str().expect("id is string").to_string())
        .collect()
}

// ---------- scenarios ----------

#[test]
fn auto_creates_work_units_and_prefixes_files_on_first_run() {
    // Scenario: Auto-creates spec/work-units.json and spec/prefixes.json on first run

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    assert!(!root.join("spec").exists(), "precondition: spec/ must not exist");

    // @step When I dispatch the list-work-units command against that project root
    let result = dispatch_command(req(root, json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true with an empty workUnits array
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["workUnits"].as_array().map(Vec::len),
        Some(0),
        "expected empty workUnits array, got {}",
        result.data
    );

    // @step Then spec/work-units.json exists with version '0.7.1' and all 7 Kanban states present and empty
    let wu_path = root.join("spec").join("work-units.json");
    assert!(wu_path.exists(), "spec/work-units.json must be created");
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&wu_path).unwrap())
        .expect("file should be valid JSON");
    assert_eq!(on_disk["version"].as_str(), Some("0.7.1"));
    let states = on_disk["states"]
        .as_object()
        .expect("states must be an object");
    for s in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = states
            .get(s)
            .unwrap_or_else(|| panic!("state '{s}' must exist"))
            .as_array()
            .unwrap_or_else(|| panic!("state '{s}' must be an array"));
        assert!(arr.is_empty(), "state '{s}' must be empty on first run");
    }

    // @step Then spec/prefixes.json exists with an empty prefixes object
    let prefixes_path = root.join("spec").join("prefixes.json");
    assert!(prefixes_path.exists(), "spec/prefixes.json must be created");
    let prefixes: Value = serde_json::from_str(&fs::read_to_string(&prefixes_path).unwrap())
        .expect("prefixes file should be valid JSON");
    let prefixes_obj = prefixes["prefixes"]
        .as_object()
        .expect("prefixes.json must have 'prefixes' object");
    assert!(prefixes_obj.is_empty(), "prefixes object must be empty on first run");
}

#[test]
fn lists_all_work_units_in_insertion_order_when_no_filters_applied() {
    // Scenario: Lists all work units in insertion order when no filters are applied

    // @step Given spec/work-units.json contains AUTH-001 (backlog, epic 'ux'), AUTH-002 (implementing), DASH-001 (backlog) in that order
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_unit_store());

    // @step When I dispatch list-work-units with no filters and format=json
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the workUnits array contains exactly AUTH-001, AUTH-002, DASH-001 in that order
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        ids_in(&data),
        vec!["AUTH-001", "AUTH-002", "DASH-001"],
        "insertion order from work-units.json must be preserved"
    );

    // @step Then each entry contains id, title, and status fields, and AUTH-001 also contains an epic field equal to 'ux'
    let arr = data["workUnits"].as_array().expect("array");
    for entry in arr {
        assert!(entry.get("id").is_some(), "entry must have id");
        assert!(entry.get("title").is_some(), "entry must have title");
        assert!(entry.get("status").is_some(), "entry must have status");
    }
    let auth001 = arr.iter().find(|e| e["id"] == "AUTH-001").unwrap();
    assert_eq!(auth001["epic"].as_str(), Some("ux"));
    let auth002 = arr.iter().find(|e| e["id"] == "AUTH-002").unwrap();
    assert!(
        !auth002.as_object().unwrap().contains_key("epic"),
        "epic must be omitted when not set"
    );
}

#[test]
fn filters_by_status_when_status_flag_provided() {
    // Scenario: Filters by status when --status flag is provided

    // @step Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_unit_store());

    // @step When I dispatch list-work-units with status='backlog' and format=json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "status": "backlog", "format": "json" }),
    ));

    // @step Then the workUnits array contains exactly AUTH-001 and DASH-001 and does not contain AUTH-002
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let ids = ids_in(&data);
    assert!(ids.contains(&"AUTH-001".to_string()));
    assert!(ids.contains(&"DASH-001".to_string()));
    assert!(!ids.contains(&"AUTH-002".to_string()));
}

#[test]
fn filters_by_prefix_appending_hyphen_before_starts_with_match() {
    // Scenario: Filters by prefix appending hyphen before startsWith match

    // @step Given spec/work-units.json contains AUTH-001, AUTH-002, DASH-001 and AUTHX-001
    let tmp = TempDir::new().expect("tempdir");
    let mut store = three_unit_store();
    store["workUnits"]["AUTHX-001"] = json!({
        "id": "AUTHX-001",
        "title": "AuthX poison pill",
        "status": "backlog",
        "createdAt": "2026-06-01T00:00:00.000Z",
        "updatedAt": "2026-06-01T00:00:00.000Z"
    });
    seed_work_units(tmp.path(), store);

    // @step When I dispatch list-work-units with prefix='AUTH' and format=json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "format": "json" }),
    ));

    // @step Then the workUnits array contains exactly AUTH-001 and AUTH-002 and excludes both DASH-001 and AUTHX-001
    assert!(result.success, "{result:?}");
    let ids = ids_in(&parse_data(&result.data));
    assert!(ids.contains(&"AUTH-001".to_string()));
    assert!(ids.contains(&"AUTH-002".to_string()));
    assert!(
        !ids.contains(&"AUTHX-001".to_string()),
        "AUTHX-001 must NOT match prefix=AUTH (the TS impl appends a '-' before startsWith)"
    );
    assert!(!ids.contains(&"DASH-001".to_string()));
}

#[test]
fn filters_by_epic_with_exact_equality() {
    // Scenario: Filters by epic with exact equality

    // @step Given spec/work-units.json contains AUTH-001 (epic 'ux'), AUTH-002 (no epic), DASH-001 (epic 'platform')
    let tmp = TempDir::new().expect("tempdir");
    let mut store = three_unit_store();
    store["workUnits"]["DASH-001"]["epic"] = json!("platform");
    seed_work_units(tmp.path(), store);

    // @step When I dispatch list-work-units with epic='ux' and format=json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "epic": "ux", "format": "json" }),
    ));

    // @step Then the workUnits array contains only AUTH-001
    assert!(result.success, "{result:?}");
    let ids = ids_in(&parse_data(&result.data));
    assert_eq!(ids, vec!["AUTH-001"]);
}

#[test]
fn filters_by_type_defaulting_missing_type_to_story() {
    // Scenario: Filters by type defaulting missing type to story

    // @step Given spec/work-units.json contains AUTH-001 with no type field and TASK-001 with type='task'
    let tmp = TempDir::new().expect("tempdir");
    let store = json!({
        "version": "0.7.1",
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001",
                "title": "Login feature",
                "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            },
            "TASK-001": {
                "id": "TASK-001",
                "title": "CI setup",
                "type": "task",
                "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["AUTH-001", "TASK-001"],
            "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    });
    seed_work_units(tmp.path(), store);

    // @step When I dispatch list-work-units with type='story' and format=json
    let result_story = dispatch_command(req(
        tmp.path(),
        json!({ "type": "story", "format": "json" }),
    ));

    // @step Then the workUnits array contains AUTH-001 and does not contain TASK-001
    assert!(result_story.success, "{result_story:?}");
    let ids_story = ids_in(&parse_data(&result_story.data));
    assert!(ids_story.contains(&"AUTH-001".to_string()));
    assert!(!ids_story.contains(&"TASK-001".to_string()));

    // @step When I dispatch list-work-units again with type='task' and format=json
    let result_task = dispatch_command(req(
        tmp.path(),
        json!({ "type": "task", "format": "json" }),
    ));

    // @step Then the workUnits array contains only TASK-001
    assert!(result_task.success, "{result_task:?}");
    let ids_task = ids_in(&parse_data(&result_task.data));
    assert_eq!(ids_task, vec!["TASK-001"]);
}

#[test]
fn combines_multiple_filters_with_and_semantics() {
    // Scenario: Combines multiple filters with AND semantics

    // @step Given spec/work-units.json contains AUTH-001 (backlog), AUTH-002 (implementing), DASH-001 (backlog)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_unit_store());

    // @step When I dispatch list-work-units with status='backlog' and prefix='AUTH' and format=json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "status": "backlog", "prefix": "AUTH", "format": "json" }),
    ));

    // @step Then the workUnits array contains only AUTH-001
    assert!(result.success, "{result:?}");
    let ids = ids_in(&parse_data(&result.data));
    assert_eq!(ids, vec!["AUTH-001"]);
}

#[test]
fn text_format_prints_no_work_units_found_for_empty_result() {
    // Scenario: Text format prints No work units found for empty result

    // @step Given spec/work-units.json contains no work units
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {},
            "states": {
                "backlog": [], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch list-work-units with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the DispatchResult.data contains the string 'No work units found'
    assert!(result.success, "{result:?}");
    assert!(
        result.data.contains("No work units found"),
        "data missing empty-result sentinel: {}",
        result.data
    );
}

#[test]
fn text_format_prints_work_units_header_and_entries_when_populated() {
    // Scenario: Text format prints work units header and entries when populated

    // @step Given spec/work-units.json contains AUTH-001 (backlog, title 'Login feature', epic 'ux')
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": {
                    "id": "AUTH-001",
                    "title": "Login feature",
                    "status": "backlog",
                    "epic": "ux",
                    "createdAt": "2026-06-01T00:00:00.000Z",
                    "updatedAt": "2026-06-01T00:00:00.000Z"
                }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch list-work-units with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the DispatchResult.data contains 'Work Units (1)' and 'AUTH-001 [backlog]' and 'Login feature' and 'Epic: ux'
    assert!(result.success, "{result:?}");
    assert!(result.data.contains("Work Units (1)"), "missing header: {}", result.data);
    assert!(result.data.contains("AUTH-001 [backlog]"), "missing id line: {}", result.data);
    assert!(result.data.contains("Login feature"), "missing title: {}", result.data);
    assert!(result.data.contains("Epic: ux"), "missing epic line: {}", result.data);
}

#[test]
fn returns_structured_error_when_work_units_json_is_malformed() {
    // Scenario: Returns structured error when work-units.json is malformed

    // @step Given spec/work-units.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).unwrap();
    fs::write(spec.join("work-units.json"), "{ not valid json").unwrap();

    // @step When I dispatch list-work-units against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "expected success=false for malformed JSON, got {result:?}");
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under codelet/fspec-core for reuse by other commands

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the modules io::project_root, io::locked_file, io::ensure, and types::work_unit exist and are publicly accessible from the crate root
    for rel in [
        "io/mod.rs",
        "io/project_root.rs",
        "io/locked_file.rs",
        "io/ensure.rs",
        "types/mod.rs",
        "types/work_unit.rs",
    ] {
        let p: PathBuf = crate_src.join(rel);
        assert!(p.exists(), "expected shared module file to exist: {}", p.display());
    }
    let lib_src = fs::read_to_string(crate_src.join("lib.rs")).expect("lib.rs readable");
    assert!(lib_src.contains("pub mod io"), "lib.rs must `pub mod io`");
    assert!(lib_src.contains("pub mod types"), "lib.rs must `pub mod types`");

    // @step Then list_work_units::run delegates to these shared modules rather than embedding its own filesystem logic
    let list_src = fs::read_to_string(crate_src.join("commands").join("list_work_units.rs"))
        .expect("list_work_units.rs readable");
    assert!(
        list_src.contains("ensure_work_units_file")
            || list_src.contains("io::ensure")
            || list_src.contains("crate::io"),
        "list_work_units.rs must delegate to shared io helpers (got: {list_src})"
    );
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "list_work_units.rs must no longer be a NotYetPorted stub"
    );
}

/// Regression: `dispatch_command` MUST be callable from within an active
/// tokio runtime — that is the exact context the agent loop runs in, where
/// `FspecToolFacadeWrapper::call` is polled by `#[tokio::main]`. Prior to
/// the fix in this commit, `run_ported` built a fresh `current_thread`
/// tokio runtime and called `block_on()` on it, which dead-locks (or
/// panics) when nested inside another runtime. The user-visible symptom
/// was the agent loop hanging indefinitely on the `Fspec` tool call.
///
/// The fix replaces the nested-runtime path with a sync poll-once helper
/// (`poll_sync_future`) since every ported command and every Phase 1 stub
/// performs no genuine `.await` work.
///
/// Scenario: Dispatching list-work-units from inside an active tokio runtime returns synchronously without hanging
#[tokio::test]
async fn dispatch_command_does_not_hang_when_called_from_inside_a_tokio_runtime() {
    // @step Given a tempdir project root seeded with spec/work-units.json containing AUTH-001, AUTH-002, and DASH-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_unit_store());

    // @step Given the test is running inside an active tokio runtime via #[tokio::test]
    // (precondition satisfied by the #[tokio::test] attribute on this test fn)

    // @step When I invoke dispatch_command for the list-work-units command via tokio::task::spawn_blocking
    let started = std::time::Instant::now();
    let result =
        tokio::task::spawn_blocking(move || dispatch_command(req(tmp.path(), json!({ "format": "json" }))))
            .await
            .expect("spawn_blocking joined cleanly — a panic here means the nested-runtime bug regressed");
    let elapsed = started.elapsed();

    // @step Then the DispatchResult has success=true within 2 seconds
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "dispatch_command from inside tokio runtime took {elapsed:?} — regression: nested block_on hanging"
    );
    assert!(result.success, "{result:?}");

    // @step Then the workUnits array contains AUTH-001, AUTH-002, and DASH-001 in insertion order
    let data = parse_data(&result.data);
    assert_eq!(ids_in(&data), vec!["AUTH-001", "AUTH-002", "DASH-001"]);
}

/// Scenario: Dispatching an unported command from inside a tokio runtime returns NotYetPorted instead of hanging
#[tokio::test]
async fn dispatch_command_returns_not_yet_ported_when_called_from_inside_a_tokio_runtime() {
    // @step Given the canonical command map registers 'add-rule' as a Phase 1 stub
    // (precondition: 'add-rule' is in CANONICAL_COMMANDS and is_ported('add-rule') == false; verified in dispatcher_test.rs)

    // @step Given the test is running inside an active tokio runtime via #[tokio::test]
    // (precondition satisfied by the #[tokio::test] attribute on this test fn)

    // @step When I invoke dispatch_command for the 'add-rule' command via tokio::task::spawn_blocking
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(|| {
        dispatch_command(DispatchRequest {
            command: "add-rule".to_string(),
            args_json: "{}".to_string(),
            project_root: std::path::PathBuf::from("/tmp/fspec-rpc327"),
        })
    })
    .await
    .expect("spawn_blocking joined cleanly — a panic here means the nested-runtime bug regressed");
    let elapsed = started.elapsed();

    // @step Then the DispatchResult has success=false within 2 seconds
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "dispatch_command for unported stub from inside tokio runtime took {elapsed:?} — regression: nested block_on hanging"
    );
    assert!(!result.success, "{result:?}");

    // @step Then the error message contains the substring 'not yet ported'
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("not yet ported"),
        "error message missing canonical 'not yet ported' substring: {msg}"
    );
}

/// Scenario: Dispatching list-work-units from a synchronous test context still works (backwards compatibility)
#[test]
fn dispatch_command_still_works_from_a_synchronous_test_context_for_backwards_compatibility() {
    // @step Given a tempdir project root seeded with a representative spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_unit_store());

    // @step Given the test is a plain #[test] with no surrounding tokio runtime
    // (precondition satisfied by the plain #[test] attribute on this test fn)

    // @step When I invoke dispatch_command for the list-work-units command directly
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the DispatchResult has success=true and the filter/render output matches the existing list_work_units test suite expectations
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    assert_eq!(ids_in(&data), vec!["AUTH-001", "AUTH-002", "DASH-001"]);
}
