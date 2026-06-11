// Feature: spec/features/add-hook-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-hook` (RPC-184).
// Each #[test] fn maps to one Gherkin scenario; @step comments mirror the
// Gherkin step text verbatim.
//
// RED phase: `add-hook` is still a NotYetPorted stub (NOT in PORTED_COMMANDS),
// so every assertion below MUST fail today — dispatch_command returns
// success=false with the canonical "not yet ported" error string instead of
// performing the documented load → modify → atomic-write round-trip.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-hook".to_string(),
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
fn scenario_creates_spec_fspec_hooks_json_when_missing_and_writes_a_single_entry() {
    // Scenario: Creates spec/fspec-hooks.json when missing and writes a single entry

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-hook with event='pre-implementing', name='lint', command='spec/hooks/lint.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "pre-implementing",
            "name": "lint",
            "command": "spec/hooks/lint.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then spec/fspec-hooks.json exists after the call
    assert!(tmp.path().join("spec/fspec-hooks.json").exists(), "config file must be created");

    // @step Then the on-disk JSON parses to a top-level object whose 'hooks' key contains exactly the event 'pre-implementing'
    let v = read_hooks_json(tmp.path());
    let hooks = v["hooks"].as_object().expect("hooks object");
    assert_eq!(hooks.len(), 1, "expected exactly one event key, got {hooks:?}");
    assert!(hooks.contains_key("pre-implementing"));

    // @step Then the 'pre-implementing' array has exactly one entry with name='lint', command='spec/hooks/lint.sh', blocking=false
    let arr = hooks["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
    assert_eq!(arr[0]["command"].as_str(), Some("spec/hooks/lint.sh"));
    assert_eq!(arr[0]["blocking"].as_bool(), Some(false));

    // @step Then the entry on disk does NOT contain a 'timeout' field
    assert!(arr[0].get("timeout").is_none(), "timeout must be omitted when not supplied");
}

#[test]
fn scenario_appends_to_an_existing_event_array_preserving_previous_entries() {
    // Scenario: Appends to an existing event array preserving previous entries

    // @step Given spec/fspec-hooks.json contains event 'post-implementing' with a single entry named 'lint'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch add-hook with event='post-implementing', name='test', command='spec/hooks/test.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "post-implementing",
            "name": "test",
            "command": "spec/hooks/test.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["post-implementing"].as_array().expect("array");

    // @step Then the on-disk 'post-implementing' array has exactly two entries
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");

    // @step Then the first entry has name='lint'
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));

    // @step Then the second entry has name='test' and command='spec/hooks/test.sh'
    assert_eq!(arr[1]["name"].as_str(), Some("test"));
    assert_eq!(arr[1]["command"].as_str(), Some("spec/hooks/test.sh"));
}

#[test]
fn scenario_adds_a_new_event_key_when_missing_from_existing_config() {
    // Scenario: Adds a new event key when missing from existing config

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

    // @step When I dispatch add-hook with event='post-implementing', name='notify', command='spec/hooks/notify.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "post-implementing",
            "name": "notify",
            "command": "spec/hooks/notify.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());
    let hooks = v["hooks"].as_object().expect("hooks object");

    // @step Then the on-disk 'hooks' object contains both 'pre-implementing' and 'post-implementing'
    assert!(hooks.contains_key("pre-implementing"));
    assert!(hooks.contains_key("post-implementing"));

    // @step Then the 'pre-implementing' event still has its 'lint' entry
    let pre = hooks["pre-implementing"].as_array().expect("pre array");
    assert_eq!(pre.len(), 1);
    assert_eq!(pre[0]["name"].as_str(), Some("lint"));

    // @step Then the 'post-implementing' event has exactly one entry named 'notify'
    let post = hooks["post-implementing"].as_array().expect("post array");
    assert_eq!(post.len(), 1);
    assert_eq!(post[0]["name"].as_str(), Some("notify"));
}

#[test]
fn scenario_omits_timeout_field_on_disk_when_not_supplied() {
    // Scenario: Omits timeout field on disk when not supplied

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false (no timeout)
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "pre-implementing",
            "name": "lint",
            "command": "lint.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the raw JSON bytes of spec/fspec-hooks.json do NOT contain the substring '"timeout"'
    let raw = read_hooks_raw(tmp.path());
    assert!(
        !raw.contains("\"timeout\""),
        "timeout field must be OMITTED entirely when not supplied; got:\n{raw}"
    );
}

#[test]
fn scenario_writes_blocking_true_and_timeout_when_supplied() {
    // Scenario: Writes blocking=true and timeout when supplied

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch add-hook with event='post-implementing', name='test', command='spec/hooks/test.sh', blocking=true, timeout=300
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "post-implementing",
            "name": "test",
            "command": "spec/hooks/test.sh",
            "blocking": true,
            "timeout": 300,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk entry has blocking=true and timeout=300
    let v = read_hooks_json(tmp.path());
    let entry = &v["hooks"]["post-implementing"][0];
    assert_eq!(entry["blocking"].as_bool(), Some(true));
    assert_eq!(entry["timeout"].as_u64(), Some(300));
}

#[test]
fn scenario_swallows_invalid_json_and_overwrites_with_the_new_config() {
    // Scenario: Swallows invalid JSON and overwrites with the new config

    // @step Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), "{ not json");

    // @step When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "pre-implementing",
            "name": "lint",
            "command": "lint.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "TS bare catch swallows parse errors; expected success=true, got {result:?}"
    );

    // @step Then the on-disk JSON parses successfully
    let v = read_hooks_json(tmp.path());

    // @step Then the on-disk 'hooks' object contains exactly the event 'pre-implementing' with one entry named 'lint'
    let hooks = v["hooks"].as_object().expect("hooks object");
    assert_eq!(hooks.len(), 1);
    let arr = hooks["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
}

#[test]
fn scenario_preserves_unknown_top_level_global_section() {
    // Scenario: Preserves unknown top-level global section

    // @step Given spec/fspec-hooks.json contains a top-level 'global' object with timeout=30 and an empty 'hooks' object
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "global": { "timeout": 30 },
  "hooks": {}
}"#,
    );

    // @step When I dispatch add-hook with event='pre-implementing', name='lint', command='lint.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "pre-implementing",
            "name": "lint",
            "command": "lint.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let v = read_hooks_json(tmp.path());

    // @step Then the on-disk JSON still contains a 'global' object with timeout=30
    assert_eq!(
        v["global"]["timeout"].as_u64(),
        Some(30),
        "global.timeout must be preserved across the round-trip; got:\n{v}"
    );

    // @step Then the on-disk 'hooks' object contains exactly the event 'pre-implementing' with one entry named 'lint'
    let hooks = v["hooks"].as_object().expect("hooks object");
    assert_eq!(hooks.len(), 1);
    let arr = hooks["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
}

#[test]
fn scenario_allows_duplicate_hook_names_within_the_same_event() {
    // Scenario: Allows duplicate hook names within the same event

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

    // @step When I dispatch add-hook with event='pre-implementing', name='lint', command='spec/hooks/other.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "pre-implementing",
            "name": "lint",
            "command": "spec/hooks/other.sh",
            "blocking": false,
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the on-disk 'pre-implementing' array has exactly two entries both named 'lint'
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
    assert_eq!(arr[1]["name"].as_str(), Some("lint"));
}

#[test]
fn scenario_preserves_event_key_insertion_order_across_writes() {
    // Scenario: Preserves event-key insertion order across writes

    // @step Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID each with one entry
    let tmp = TempDir::new().expect("tempdir");
    // Hand-write to lock object key order.
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

    // @step When I dispatch add-hook with event='MID', name='extra', command='extra.sh', blocking=false
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "event": "MID",
            "name": "extra",
            "command": "extra.sh",
            "blocking": false,
        }),
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
