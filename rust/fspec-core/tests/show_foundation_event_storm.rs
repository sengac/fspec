//! Feature: spec/features/show-foundation-event-storm-rust-port.feature
//!
//! Dispatcher-level tests for `show-foundation-event-storm` (RPC-306), moved out of
//! rust/fspec/tests/cli_show_foundation_event_storm.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

// ───────── Helpers ─────────

fn write_foundation(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), raw).expect("write foundation.json");
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "show-foundation-event-storm".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

// ─── Sample foundation bodies ───

fn foundation_no_event_storm() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]}}"#
}

fn foundation_with_three_active_one_deleted() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"aggregate","text":"A1"},
        {"id":2,"type":"aggregate","text":"A2"},
        {"id":3,"type":"event","text":"E1"},
        {"id":4,"type":"aggregate","text":"A3","deleted":true}
    ]}}"#
}

fn foundation_two_aggs_one_bc_one_event() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"aggregate","text":"A1"},
        {"id":2,"type":"aggregate","text":"A2"},
        {"id":3,"type":"bounded_context","text":"Work Management"},
        {"id":4,"type":"event","text":"E1"}
    ]}}"#
}

fn foundation_bc_with_linked_items() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"bounded_context","text":"Work Management"},
        {"id":10,"type":"aggregate","text":"WU","boundedContextId":1},
        {"id":11,"type":"aggregate","text":"Epic","boundedContextId":1},
        {"id":12,"type":"event","text":"Created","boundedContextId":1},
        {"id":20,"type":"aggregate","text":"Other1","boundedContextId":2},
        {"id":21,"type":"aggregate","text":"Other2","boundedContextId":2}
    ]}}"#
}

fn foundation_combined_filter_data() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"bounded_context","text":"Work Management"},
        {"id":10,"type":"aggregate","text":"WU","boundedContextId":1},
        {"id":11,"type":"aggregate","text":"Epic","boundedContextId":1},
        {"id":12,"type":"event","text":"Created","boundedContextId":1},
        {"id":20,"type":"aggregate","text":"Other","boundedContextId":2}
    ]}}"#
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 1: Missing foundation.json surfaces an error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_missing_foundation_json_surfaces_error() {
    // @step Given an empty temp project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-foundation-event-storm with no arguments
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'foundation.json'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("foundation.json"),
        "error must mention foundation.json; got: {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 2: No eventStorm field returns empty data
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_no_event_storm_field_returns_empty_data_and_message() {
    // @step Given spec/foundation.json exists without an eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_no_event_storm());

    // @step When I dispatch show-foundation-event-storm with no arguments
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data field is an empty JSON array
    let arr = parsed["data"].as_array().expect("data is array");
    assert_eq!(arr.len(), 0, "data array must be empty; got {arr:?}");

    // @step And the message field equals 'No Event Storm data in foundation.json'
    assert_eq!(
        parsed["message"].as_str(),
        Some("No Event Storm data in foundation.json")
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 3: Soft-deleted items are filtered out
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_soft_deleted_items_filtered_out() {
    // @step Given spec/foundation.json contains eventStorm.items with three active items and one item where deleted=true
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_with_three_active_one_deleted());

    // @step When I dispatch show-foundation-event-storm with no arguments
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = parsed["data"].as_array().expect("data is array");

    // @step And the data field is a JSON array with exactly 3 items
    assert_eq!(arr.len(), 3, "expected 3 active items; got {arr:?}");

    // @step And no returned item has deleted=true
    for item in arr {
        assert_ne!(item["deleted"].as_bool(), Some(true));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 4: Filtering by type returns matching items
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_filtering_by_type_returns_matching_items() {
    // @step Given spec/foundation.json contains eventStorm.items with two aggregates, one bounded_context, and one event
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_two_aggs_one_bc_one_event());

    // @step When I dispatch show-foundation-event-storm with type='aggregate'
    let result = dispatch(ws.path(), r#"{"type":"aggregate"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = parsed["data"].as_array().expect("data is array");

    // @step And the data field contains exactly 2 items
    assert_eq!(arr.len(), 2, "expected 2 aggregates; got {arr:?}");

    // @step And every returned item has type='aggregate'
    for item in arr {
        assert_eq!(item["type"].as_str(), Some("aggregate"));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 5: Filtering by context returns BC + linked
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_filtering_by_context_returns_bounded_context_plus_linked_items() {
    // @step Given spec/foundation.json contains a bounded_context with id=1 and text='Work Management' plus three items where boundedContextId=1 and two items where boundedContextId=2
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_bc_with_linked_items());

    // @step When I dispatch show-foundation-event-storm with context='Work Management'
    let result = dispatch(ws.path(), r#"{"context":"Work Management"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = parsed["data"].as_array().expect("data is array");

    // @step And the data field contains exactly 4 items
    assert_eq!(
        arr.len(),
        4,
        "expected 4 items (BC + 3 linked); got {arr:?}"
    );

    // @step And one returned item has type='bounded_context' and text='Work Management'
    let bc_count = arr
        .iter()
        .filter(|i| {
            i["type"].as_str() == Some("bounded_context")
                && i["text"].as_str() == Some("Work Management")
        })
        .count();
    assert_eq!(bc_count, 1);

    // @step And every other returned item has boundedContextId=1
    for item in arr {
        if item["type"].as_str() != Some("bounded_context") {
            assert_eq!(item["boundedContextId"].as_u64(), Some(1));
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 6: Filtering by unknown context errors and
// lists available contexts (DISC-003 rule 10 supersedes the old
// "returns an empty array" behavior)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_filtering_by_unknown_context_errors_and_lists_available_contexts() {
    // @step Given spec/foundation.json contains a bounded_context with text='Work Management' and three items linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_bc_with_linked_items());

    // @step When I dispatch show-foundation-event-storm with context='Nonexistent'
    let result = dispatch(ws.path(), r#"{"context":"Nonexistent"}"#);

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains "Unknown context 'Nonexistent'"
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("Unknown context 'Nonexistent'"),
        "error must name the bad context: {error:?}"
    );

    // @step And the error message lists 'Work Management' as an available bounded context
    assert!(
        error.contains("Work Management"),
        "error must list the available context: {error:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 7: Combined context and type filters compose
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_combined_context_and_type_filters_compose() {
    // @step Given spec/foundation.json contains a bounded_context id=1 'Work Management' with two aggregates and one event linked to it plus one aggregate linked to a different bounded context
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_combined_filter_data());

    // @step When I dispatch show-foundation-event-storm with context='Work Management' and type='aggregate'
    let result = dispatch(
        ws.path(),
        r#"{"context":"Work Management","type":"aggregate"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = parsed["data"].as_array().expect("data is array");

    // @step And the data field contains exactly 2 items
    assert_eq!(
        arr.len(),
        2,
        "expected 2 aggregates linked to WM; got {arr:?}"
    );

    // @step And every returned item has type='aggregate' and boundedContextId=1
    for item in arr {
        assert_eq!(item["type"].as_str(), Some("aggregate"));
        assert_eq!(item["boundedContextId"].as_u64(), Some(1));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 8: Shared infrastructure module is registered
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_shared_infrastructure_module_is_registered() {
    // @step Given the rust/fspec-core crate is built
    // (Compile-time guarantee.)

    // @step When I inspect rust/fspec-core/src/commands/show_foundation_event_storm.rs
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_no_event_storm());
    let result = dispatch(ws.path(), "{}");

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-306"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes show-foundation-event-storm to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when foundation.json is valid; got {result:?}"
    );
}
