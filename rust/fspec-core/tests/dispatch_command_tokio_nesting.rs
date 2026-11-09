//! Feature: spec/features/fspec-dispatcher-tokio-nesting-safety.feature
//!
//! Moved out of list_work_units.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_core::{canonical::is_ported, dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
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
    let result = tokio::task::spawn_blocking(move || {
        dispatch_command(req(tmp.path(), json!({ "format": "json" })))
    })
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

/// Scenario: Dispatching a ported command with invalid args from inside a tokio runtime returns a structured error without hanging
#[tokio::test]
async fn dispatch_ported_command_with_invalid_args_from_tokio_runtime_returns_structured_error_without_hanging(
) {
    // @step Given the canonical command 'add-rule' is ported (is_ported returns true)
    assert!(
        is_ported("add-rule"),
        "precondition: 'add-rule' must be ported for this scenario"
    );

    // @step When I invoke dispatch_command for the 'add-rule' command with empty args_json via tokio::task::spawn_blocking
    let started = std::time::Instant::now();
    let result = tokio::task::spawn_blocking(|| {
        dispatch_command(DispatchRequest {
            command: "add-rule".to_string(),
            args_json: "{}".to_string(),
            project_root: PathBuf::from("/tmp/fspec-rpc336"),
        })
    })
    .await
    .expect("spawn_blocking joined cleanly — a panic here means the nested-runtime bug regressed");
    let elapsed = started.elapsed();

    // @step Then the DispatchResult has success=false within 2 seconds
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "dispatch_command for a ported command from inside tokio runtime took {elapsed:?} — regression: nested block_on hanging"
    );
    assert!(!result.success, "{result:?}");

    // @step Given the test is running inside an active tokio runtime via #[tokio::test]
    // (precondition satisfied by the #[tokio::test] attribute on this test fn)

    // @step Then the error message contains the substring 'Invalid args'
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Invalid args"),
        "error message missing 'Invalid args' substring: {msg}"
    );

    // @step And the error message does NOT contain the substring 'not yet ported'
    assert!(
        !msg.contains("not yet ported"),
        "a ported command must not return a NotYetPorted message: {msg}"
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
