// Feature: spec/features/list-hooks-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-hooks`
// (RPC-247). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: list-hooks is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical
// "not yet ported" error string instead of the expected
// `events` payload / text rendering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-hooks".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_hooks(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn scenario_returns_empty_events_with_message_when_file_missing() {
    // Scenario: Returns empty events with 'No hooks are configured' when spec/fspec-hooks.json does not exist

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the list-hooks command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has events array of length 0
    let data = parse_data(&result.data);
    assert_eq!(
        data["events"].as_array().map(Vec::len),
        Some(0),
        "expected empty events array, got {}",
        result.data
    );

    // @step Then the parsed JSON has message field equal to 'No hooks are configured'
    assert_eq!(
        data["message"].as_str(),
        Some("No hooks are configured"),
        "expected message='No hooks are configured', got {}",
        result.data
    );

    // @step Then spec/fspec-hooks.json does not exist after the call
    assert!(
        !tmp.path().join("spec/fspec-hooks.json").exists(),
        "list-hooks must NOT auto-create spec/fspec-hooks.json"
    );
}

#[test]
fn scenario_returns_event_hook_mapping_when_populated() {
    // Scenario: Returns event/hook mapping when spec/fspec-hooks.json is populated

    // @step Given spec/fspec-hooks.json contains event 'post-implementing' with hooks named 'lint' and 'test' in that order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "s.sh" },
      { "name": "test", "command": "t.sh" }
    ]
  }
}"#;
    write_hooks(tmp.path(), raw);

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);
    let arr = data["events"].as_array().expect("events array");

    // @step Then the events array contains exactly one entry
    assert_eq!(arr.len(), 1, "expected 1 event, got {arr:?}");

    // @step Then the first event has event='post-implementing' and hooks=['lint','test']
    assert_eq!(arr[0]["event"].as_str(), Some("post-implementing"));
    let hooks = arr[0]["hooks"].as_array().expect("hooks array");
    assert_eq!(hooks.len(), 2);
    assert_eq!(hooks[0].as_str(), Some("lint"));
    assert_eq!(hooks[1].as_str(), Some("test"));

    // @step Then the parsed JSON does NOT contain a top-level 'message' field
    assert!(
        data.get("message").is_none(),
        "expected NO top-level message field, got {}",
        result.data
    );
}

#[test]
fn scenario_empty_hooks_object_is_no_events_without_message() {
    // Scenario: Treats empty hooks object as no events without a message field

    // @step Given spec/fspec-hooks.json exists and parses to an object whose 'hooks' field is the empty object
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), r#"{ "hooks": {} }"#);

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);

    // @step Then the events array has length 0
    assert_eq!(
        data["events"].as_array().map(Vec::len),
        Some(0),
        "expected empty events array, got {}",
        result.data
    );

    // @step Then the parsed JSON does NOT contain a top-level 'message' field
    assert!(
        data.get("message").is_none(),
        "happy-path empty must NOT include message; got {}",
        result.data
    );
}

#[test]
fn scenario_swallows_invalid_json_as_empty_with_message() {
    // Scenario: Swallows invalid JSON as empty result with 'No hooks are configured' message

    // @step Given spec/fspec-hooks.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(tmp.path(), "{ not json");

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "TS bare catch swallows parse errors; expected success=true, got {result:?}"
    );

    let data = parse_data(&result.data);

    // @step Then the events array has length 0
    assert_eq!(
        data["events"].as_array().map(Vec::len),
        Some(0),
        "expected empty events array on parse failure, got {}",
        result.data
    );

    // @step Then the parsed JSON has message field equal to 'No hooks are configured'
    assert_eq!(
        data["message"].as_str(),
        Some("No hooks are configured"),
        "parse-error path must carry the canonical sentinel message, got {}",
        result.data
    );
}

#[test]
fn scenario_preserves_insertion_order_of_events() {
    // Scenario: Preserves insertion order of events (not alphabetical)

    // @step Given spec/fspec-hooks.json contains three events declared in order ZED, AAA, MID
    let tmp = TempDir::new().expect("tempdir");
    // Hand-write so the object key order is preserved on the wire.
    let raw = r#"{
  "hooks": {
    "ZED": [ { "name": "z", "command": "z.sh" } ],
    "AAA": [ { "name": "a", "command": "a.sh" } ],
    "MID": [ { "name": "m", "command": "m.sh" } ]
  }
}"#;
    write_hooks(tmp.path(), raw);

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data["events"].as_array().expect("events array");

    // @step Then the events array contains three entries in order ZED, AAA, MID
    assert_eq!(arr.len(), 3, "expected 3 events, got {arr:?}");
    assert_eq!(arr[0]["event"].as_str(), Some("ZED"));
    assert_eq!(arr[1]["event"].as_str(), Some("AAA"));
    assert_eq!(arr[2]["event"].as_str(), Some("MID"));
}

#[test]
fn scenario_emits_null_for_hooks_missing_name_field() {
    // Scenario: Emits null for hooks missing the name field

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with two hook entries — the first with name='lint' and the second with NO name field
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "l.sh" },
      { "command": "x.sh" }
    ]
  }
}"#;
    write_hooks(tmp.path(), raw);

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data["events"].as_array().expect("events array");

    // @step Then the first event's hooks array equals ['lint', null]
    let hooks = arr[0]["hooks"].as_array().expect("hooks array");
    assert_eq!(hooks.len(), 2, "expected two hook entries, got {hooks:?}");
    assert_eq!(hooks[0].as_str(), Some("lint"));
    assert!(
        hooks[1].is_null(),
        "missing-name hook must surface as JSON null, got {}",
        hooks[1]
    );
}

#[test]
fn scenario_json_format_two_space_indent_for_empty_case() {
    // Scenario: JSON format emits two-space indented payload for the empty/missing case

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-hooks with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data starts with the exact string "{\n  \"events\": [],\n"
    assert!(
        result.data.starts_with("{\n  \"events\": [],\n"),
        "expected 2-space indented JSON opener; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact substring "\"message\": \"No hooks are configured\""
    assert!(
        result
            .data
            .contains("\"message\": \"No hooks are configured\""),
        "missing canonical message field; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_populated_help_example_layout() {
    // Scenario: Text format renders the populated case using the documented help-example layout

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with hooks ['lint'] and event 'post-implementing' with hooks ['test', 'notify'] in that order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "l.sh" }
    ],
    "post-implementing": [
      { "name": "test", "command": "t.sh" },
      { "name": "notify", "command": "n.sh" }
    ]
  }
}"#;
    write_hooks(tmp.path(), raw);

    // @step When I dispatch list-hooks with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Configured Hooks:'
    assert!(
        result.data.contains("Configured Hooks:"),
        "missing 'Configured Hooks:' header; got:\n{}",
        result.data
    );

    // @step Then the substring 'pre-implementing:' appears before 'post-implementing:' in the output
    let pre = result
        .data
        .find("pre-implementing:")
        .expect("pre-implementing: present");
    let post = result
        .data
        .find("post-implementing:")
        .expect("post-implementing: present");
    assert!(
        pre < post,
        "expected pre-implementing < post-implementing; pre={pre} post={post}\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line 'pre-implementing:'
    assert!(
        result.data.lines().any(|l| l == "pre-implementing:"),
        "missing exact line 'pre-implementing:'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  - lint'
    assert!(
        result.data.lines().any(|l| l == "  - lint"),
        "missing exact line '  - lint'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line 'post-implementing:'
    assert!(
        result.data.lines().any(|l| l == "post-implementing:"),
        "missing exact line 'post-implementing:'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  - test'
    assert!(
        result.data.lines().any(|l| l == "  - test"),
        "missing exact line '  - test'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  - notify'
    assert!(
        result.data.lines().any(|l| l == "  - notify"),
        "missing exact line '  - notify'; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_empty_prints_no_hooks_sentinel() {
    // Scenario: Text format prints 'No hooks are configured' for the empty/missing case

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-hooks with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No hooks are configured'
    assert_eq!(
        result.data, "No hooks are configured",
        "expected exact 'No hooks are configured' sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_default_format_is_text() {
    // Scenario: Default format (no format key supplied) is text

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-hooks with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No hooks are configured'
    assert_eq!(
        result.data, "No hooks are configured",
        "default format must be text and render the empty sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_renders_unnamed_placeholder_when_hook_lacks_name_field() {
    // Scenario: Renders unnamed placeholder when a hook lacks the name field
    //
    // RPC-247 follow-up fix: the impl at codelet/fspec-core/src/commands/list_hooks.rs:221
    // emits `  - (unnamed)\n` for hooks missing the `name` field in the text
    // renderer. This scenario locks in that behavior so a future refactor can't
    // silently drop the marker.

    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with a single hook entry that has NO name field but a command field
    let tmp = TempDir::new().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{ "hooks": { "pre-implementing": [ { "command": "/bin/true" } ] } }"#,
    );

    // @step When I dispatch list-hooks with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line '  - (unnamed)'
    assert!(
        result.data.lines().any(|l| l == "  - (unnamed)"),
        "missing exact line '  - (unnamed)'; got:\n{}",
        result.data
    );
}
