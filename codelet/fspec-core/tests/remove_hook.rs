// Feature: spec/features/remove-hook-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-hook`
// (RPC-275). Each #[test] fn maps to one Gherkin scenario; @step comments
// mirror the Gherkin step text verbatim.
//
// RED phase: `remove-hook` is still a NotYetPorted stub (NOT in
// PORTED_COMMANDS), so every assertion below MUST fail today —
// dispatch_command returns success=false with the canonical "not yet
// ported" error string instead of performing the documented filter +
// atomic-write round-trip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-hook".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_hooks(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

fn read_hooks_json(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("fspec-hooks.json"))
        .expect("read fspec-hooks.json");
    serde_json::from_str(&raw).expect("parse fspec-hooks.json")
}

fn read_hooks_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec").join("fspec-hooks.json"))
        .expect("read fspec-hooks.json raw")
}

// ---------- scenarios ----------

#[test]
fn scenario_removes_a_single_named_entry_leaving_siblings_intact() {
    // Scenario: Removes a single named entry leaving siblings intact

    // @step Given spec/fspec-hooks.json contains event 'post-implementing' with entries named 'lint' and 'test' in that order
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false },
      { "name": "test", "command": "spec/hooks/test.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='post-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "post-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk 'post-implementing' array has exactly one entry named 'test'
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["post-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1, "expected 1 remaining entry, got {arr:?}");
    assert_eq!(arr[0]["name"].as_str(), Some("test"));
}

#[test]
fn scenario_empty_array_after_removal_is_retained_event_key_not_deleted() {
    // Scenario: Empty array after removal is retained (event key NOT deleted)

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='pre-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());
    let hooks = v["hooks"].as_object().expect("hooks object");

    // @step Then the on-disk 'hooks' object still contains the key 'pre-implementing'
    assert!(
        hooks.contains_key("pre-implementing"),
        "event key must NOT be deleted when its array becomes empty; got {hooks:?}"
    );

    // @step Then the on-disk 'pre-implementing' array is exactly []
    let arr = hooks["pre-implementing"].as_array().expect("array");
    assert!(arr.is_empty(), "expected empty array, got {arr:?}");
}

#[test]
fn scenario_all_duplicate_entries_with_the_same_name_are_removed() {
    // Scenario: All duplicate entries with the same name are removed

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with three entries — two named 'lint' and one named 'other'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "a.sh", "blocking": false },
      { "name": "other", "command": "o.sh", "blocking": false },
      { "name": "lint", "command": "b.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='pre-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk 'pre-implementing' array has exactly one entry named 'other'
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1, "expected 1 remaining entry, got {arr:?}");
    assert_eq!(arr[0]["name"].as_str(), Some("other"));
}

#[test]
fn scenario_missing_event_key_is_a_silent_noop_success() {
    // Scenario: Missing event key is a silent no-op success

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='post-implementing', name='test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "post-implementing", "name": "test" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());
    let hooks = v["hooks"].as_object().expect("hooks object");

    // @step Then the on-disk 'hooks' object contains only the key 'pre-implementing'
    assert_eq!(hooks.len(), 1, "expected only pre-implementing, got {hooks:?}");
    assert!(hooks.contains_key("pre-implementing"));

    // @step Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')
    let arr = hooks["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
}

#[test]
fn scenario_removing_a_name_that_does_not_exist_is_a_silent_noop_success() {
    // Scenario: Removing a name that does not exist is a silent no-op success

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='pre-implementing', name='nonexistent'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "nonexistent" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
}

#[test]
fn scenario_enoent_on_spec_fspec_hooks_json_propagates_an_error() {
    // Scenario: ENOENT on spec/fspec-hooks.json propagates an error

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch remove-hook with event='pre-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false on missing config file; got {result:?}"
    );

    // @step Then the dispatcher error message indicates an IO failure
    let combined = format!("{} {}", result.data, result.error.clone().unwrap_or_default());
    let lower = combined.to_lowercase();
    assert!(
        lower.contains("io")
            || lower.contains("no such file")
            || lower.contains("not found")
            || lower.contains("read"),
        "expected IO-failure error message, got data={:?} error={:?}",
        result.data,
        result.error
    );
}

#[test]
fn scenario_invalid_json_propagates_a_parsejson_error_no_silent_overwrite() {
    // Scenario: Invalid JSON propagates a ParseJson error (NO silent overwrite)

    // @step Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), "{ not json");

    // @step When I dispatch remove-hook with event='pre-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "remove-hook must propagate parse errors, not swallow them; got {result:?}"
    );

    // @step Then the dispatcher error message indicates a parse failure for fspec-hooks.json
    let combined = format!("{} {}", result.data, result.error.clone().unwrap_or_default());
    assert!(
        combined.contains("fspec-hooks.json"),
        "expected parse error message to reference fspec-hooks.json; got data={:?} error={:?}",
        result.data,
        result.error
    );

    // @step Then the raw bytes of spec/fspec-hooks.json equal '{ not json' (file unchanged)
    let raw = read_hooks_raw(tmp.path());
    assert_eq!(raw, "{ not json", "config file must be UNCHANGED on parse error; got: {raw:?}");
}

#[test]
fn scenario_preserves_unknown_top_level_fields_and_adjacent_entries() {
    // Scenario: Preserves unknown top-level fields and adjacent entries

    // @step Given spec/fspec-hooks.json contains a top-level 'global' object with timeout=30 and event 'pre-implementing' with entries named 'lint' and 'keep' where 'keep' has command='spec/hooks/keep.sh' and blocking=true and timeout=120
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "global": { "timeout": 30 },
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false },
      { "name": "keep", "command": "spec/hooks/keep.sh", "blocking": true, "timeout": 120 }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='pre-implementing', name='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "pre-implementing", "name": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());

    // @step Then the on-disk JSON still contains a 'global' object with timeout=30
    assert_eq!(
        v["global"]["timeout"].as_u64(),
        Some(30),
        "global.timeout must be preserved; got:\n{v}"
    );

    // @step Then the on-disk 'pre-implementing' array has exactly one entry named 'keep' with command='spec/hooks/keep.sh', blocking=true, and timeout=120
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1, "expected 1 remaining entry, got {arr:?}");
    let entry = &arr[0];
    assert_eq!(entry["name"].as_str(), Some("keep"));
    assert_eq!(entry["command"].as_str(), Some("spec/hooks/keep.sh"));
    assert_eq!(entry["blocking"].as_bool(), Some(true));
    assert_eq!(entry["timeout"].as_u64(), Some(120));
}

#[test]
fn scenario_preserves_event_key_insertion_order_across_writes() {
    // Scenario: Preserves event-key insertion order across writes

    // @step Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID each with one entry
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "ZED": [ { "name": "z", "command": "z.sh", "blocking": false } ],
    "AAA": [ { "name": "a", "command": "a.sh", "blocking": false } ],
    "MID": [ { "name": "m", "command": "m.sh", "blocking": false } ]
  }
}"#,
    );

    // @step When I dispatch remove-hook with event='AAA', name='a'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "event": "AAA", "name": "a" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk events appear in the order ZED, AAA, MID
    let raw = read_hooks_raw(tmp.path());
    let z = raw.find("\"ZED\"").expect("ZED present");
    let a = raw.find("\"AAA\"").expect("AAA present");
    let m = raw.find("\"MID\"").expect("MID present");
    assert!(
        z < a && a < m,
        "expected ZED < AAA < MID; got z={z} a={a} m={m}\n{raw}"
    );
}
