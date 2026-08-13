#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/show-work-unit-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `show-work-unit`
// (RPC-308). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-work-unit".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Build a `work-units.json` carrying a single AUTH-001 entry with the
/// supplied per-field raw JSON fragments. Empty fragments are skipped so
/// callers can test the "absent field" branch.
#[allow(clippy::too_many_arguments)]
fn build_workunits(
    id: &str,
    title: &str,
    wu_type: &str,
    status: &str,
    extras: &[(&str, &str)],
) -> String {
    let mut fields = format!(
        r#""id":"{id}","title":"{title}","type":"{wu_type}","status":"{status}","createdAt":"2025-01-01T00:00:00.000Z","updatedAt":"2025-01-02T00:00:00.000Z""#
    );
    for (k, v) in extras {
        if v.is_empty() {
            continue;
        }
        fields.push_str(&format!(r#","{k}":{v}"#));
    }
    let all_states = [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ];
    let mut state_pairs = Vec::new();
    for st in &all_states {
        if *st == status {
            state_pairs.push(format!(r#""{st}":["{id}"]"#));
        } else {
            state_pairs.push(format!(r#""{st}":[]"#));
        }
    }
    let state_list = state_pairs.join(",");
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{ "{id}": {{ {fields} }} }},
  "states": {{ {state_list} }}
}}"#
    )
}

fn minimal_unit(id: &str, title: &str, status: &str) -> String {
    build_workunits(id, title, "story", status, &[])
}

// Helper that clears FSPEC_DISABLE_REMINDERS for tests that need reminders ON.
fn unset_disable_reminders() {
    std::env::remove_var("FSPEC_DISABLE_REMINDERS");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns a minimal work unit with declaration-order fields and an empty linkedFeatures array
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn returns_minimal_work_unit_with_empty_linked_features() {
    unset_disable_reminders();
    // @step Given a tempdir whose spec/work-units.json contains AUTH-001 with title='Login', status='backlog', no rules/examples/questions/notes, and no estimate
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "Login", "backlog"));

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the DispatchResult.data parses as JSON with id='AUTH-001', title='Login', type='story', status='backlog'
    assert_eq!(data["id"].as_str(), Some("AUTH-001"));
    assert_eq!(data["title"].as_str(), Some("Login"));
    assert_eq!(data["type"].as_str(), Some("story"));
    assert_eq!(data["status"].as_str(), Some("backlog"));

    // @step Then the JSON payload's linkedFeatures field is an empty array
    assert_eq!(data["linkedFeatures"].as_array().map(Vec::len), Some(0));

    // @step Then the JSON payload omits both systemReminders and systemReminder (backlog status suppresses the missing-estimate reminder)
    let obj = data.as_object().expect("root object");
    assert!(
        !obj.contains_key("systemReminders"),
        "must omit systemReminders for backlog"
    );
    assert!(
        !obj.contains_key("systemReminder"),
        "must omit systemReminder for backlog"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns success=false when spec/work-units.json is absent
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn returns_failure_when_work_units_json_absent_and_does_not_auto_create() {
    unset_disable_reminders();
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId":"AUTH-001"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message does NOT contain the substring "does not exist" (TS bare readFile escalates ENOENT; the Rust port surfaces a structured I/O error and does NOT auto-create)
    let msg = result.error.as_ref().expect("error message");
    assert!(
        !msg.contains("does not exist"),
        "error must NOT surface 'does not exist' for ENOENT path; got: {msg}"
    );

    // @step Then spec/work-units.json was NOT created in the directory
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "show-work-unit MUST NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns success=false with the canonical missing-work-unit message
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn returns_canonical_missing_work_unit_message() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 (any minimal shape)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "t", "backlog"));

    // @step When I dispatch show-work-unit with workUnitId='UNKNOWN-999' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"UNKNOWN-999","format":"json"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Work unit 'UNKNOWN-999' does not exist"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Work unit 'UNKNOWN-999' does not exist"),
        "missing canonical substring; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Projects active rules and omits soft-deleted entries in non-verbose mode
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn projects_active_rules_and_omits_soft_deleted() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with rules=[{id:0,text:'A',deleted:false},{id:1,text:'B',deleted:true},{id:2,text:'C',deleted:false}] and status='implementing'
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"A","deleted":false,"createdAt":"x"},
        {"id":1,"text":"B","deleted":true,"createdAt":"x"},
        {"id":2,"text":"C","deleted":false,"createdAt":"x"}
    ]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("rules", rules)],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's rules array equals ["[0] A", "[2] C"]
    let arr = data["rules"].as_array().expect("rules array");
    let strings: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(strings, vec!["[0] A", "[2] C"]);

    // @step Then the JSON payload does NOT contain a deletedRules field
    let obj = data.as_object().expect("root object");
    assert!(
        !obj.contains_key("deletedRules"),
        "deletedRules must be omitted"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Projects active examples and omits soft-deleted entries
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn projects_active_examples_and_omits_soft_deleted() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with examples=[{id:0,text:'E1',deleted:false},{id:1,text:'gone',deleted:true}] and status='implementing'
    let tmp = TempDir::new().expect("tempdir");
    let examples = r#"[
        {"id":0,"text":"E1","deleted":false,"createdAt":"x"},
        {"id":1,"text":"gone","deleted":true,"createdAt":"x"}
    ]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("examples", examples)],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's examples array equals ["[0] E1"]
    let arr = data["examples"].as_array().expect("examples array");
    let strings: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(strings, vec!["[0] E1"]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Filters questions by both deleted and selected flags
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn filters_questions_by_deleted_and_selected_flags() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with questions=[{id:0,text:'Q1',deleted:false,selected:false},{id:1,text:'answered',deleted:false,selected:true},{id:2,text:'gone',deleted:true,selected:false}] and status='implementing'
    let tmp = TempDir::new().expect("tempdir");
    let questions = r#"[
        {"id":0,"text":"Q1","deleted":false,"selected":false,"createdAt":"x"},
        {"id":1,"text":"answered","deleted":false,"selected":true,"createdAt":"x"},
        {"id":2,"text":"gone","deleted":true,"selected":false,"createdAt":"x"}
    ]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("questions", questions)],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's questions array equals ["[0] Q1"]
    let arr = data["questions"].as_array().expect("questions array");
    let strings: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(strings, vec!["[0] Q1"]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Projects active architecture notes and omits soft-deleted entries
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn projects_active_architecture_notes_and_omits_soft_deleted() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with architectureNotes=[{id:0,text:'N1',deleted:false},{id:1,text:'N2',deleted:false},{id:2,text:'gone',deleted:true}] and status='implementing'
    let tmp = TempDir::new().expect("tempdir");
    let notes = r#"[
        {"id":0,"text":"N1","deleted":false,"createdAt":"x"},
        {"id":1,"text":"N2","deleted":false,"createdAt":"x"},
        {"id":2,"text":"gone","deleted":true,"createdAt":"x"}
    ]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("architectureNotes", notes)],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's architectureNotes array equals ["[0] N1", "[1] N2"]
    let arr = data["architectureNotes"]
        .as_array()
        .expect("architectureNotes array");
    let strings: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(strings, vec!["[0] N1", "[1] N2"]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Rejects legacy bare-string question entries with a canonical error
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn rejects_legacy_bare_string_question_entries() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with questions=["bare string"] (legacy format)
    let tmp = TempDir::new().expect("tempdir");
    let questions = r#"["bare string"]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("questions", questions)],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Invalid question format. Questions must be QuestionItem objects."
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Invalid question format. Questions must be QuestionItem objects."),
        "missing canonical legacy-rejection substring; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Emits the bare soft-delete count notice when rules has both active and deleted entries
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn emits_soft_delete_count_notice_for_mixed_rules() {
    // @step Given spec/work-units.json contains AUTH-001 with status='implementing', estimate=3, and rules=[{id:0,text:'a',deleted:false},{id:1,text:'b',deleted:false},{id:2,text:'c',deleted:false},{id:3,text:'d',deleted:true}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"a","deleted":false,"createdAt":"x"},
        {"id":1,"text":"b","deleted":false,"createdAt":"x"},
        {"id":2,"text":"c","deleted":false,"createdAt":"x"},
        {"id":3,"text":"d","deleted":true,"createdAt":"x"}
    ]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("rules", rules), ("estimate", "3")],
        ),
    );

    // @step Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    unset_disable_reminders();

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's systemReminders array contains the bare string "3 active items (1 deleted)"
    let arr = data["systemReminders"]
        .as_array()
        .expect("systemReminders array");
    let has = arr.iter().any(|v| {
        v.as_str()
            .map(|s| s.contains("3 active items (1 deleted)"))
            .unwrap_or(false)
    });
    assert!(has, "missing soft-delete count notice; got: {arr:?}");

    // @step Then the JSON payload's systemReminder field is a single <system-reminder>…</system-reminder> block containing the substring "3 active items (1 deleted)"
    let block = data["systemReminder"]
        .as_str()
        .expect("systemReminder string");
    assert!(
        block.starts_with("<system-reminder>"),
        "must start with <system-reminder>: {block}"
    );
    assert!(
        block.ends_with("</system-reminder>"),
        "must end with </system-reminder>: {block}"
    );
    assert!(
        block.contains("3 active items (1 deleted)"),
        "wrapped block missing notice substring: {block}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Inherits feature-level work-unit tags onto scenarios that lack their own override
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn inherits_feature_level_work_unit_tags_onto_scenarios() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "t", "backlog"));

    // @step Given spec/features/auth.feature has '@AUTH-001' as a feature-level tag, a 'Login' scenario with NO scenario-level work-unit tag, and a 'Logout' scenario carrying its own '@AUTH-002' override
    let feat_dir = tmp.path().join("spec/features");
    fs::create_dir_all(&feat_dir).expect("mkdir features");
    let body = "@AUTH-001\nFeature: Auth\n\n  Scenario: Login\n    Given a user\n    When they log in\n    Then ok\n\n  @AUTH-002\n  Scenario: Logout\n    Given a session\n    When they log out\n    Then ok\n";
    fs::write(feat_dir.join("auth.feature"), body).expect("write feature");

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's linkedFeatures array has exactly one entry whose file ends with 'spec/features/auth.feature'
    let arr = data["linkedFeatures"]
        .as_array()
        .expect("linkedFeatures array");
    assert_eq!(
        arr.len(),
        1,
        "expected exactly one linked feature, got {arr:?}"
    );
    let file = arr[0]["file"].as_str().expect("file string");
    assert!(
        file.ends_with("spec/features/auth.feature")
            || file.ends_with("spec\\features\\auth.feature"),
        "file path mismatch: {file}"
    );

    // @step Then that entry's scenarios array references only the 'Login' scenario (the Logout scenario is excluded because of its own @AUTH-002 override)
    let scenarios = arr[0]["scenarios"].as_array().expect("scenarios array");
    let names: Vec<&str> = scenarios
        .iter()
        .map(|v| v.get("name").and_then(|n| n.as_str()).unwrap_or(""))
        .collect();
    assert!(names.contains(&"Login"), "Login must be present: {names:?}");
    assert!(
        !names.contains(&"Logout"),
        "Logout must be excluded: {names:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Silently degrades when spec/features/ does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn silently_degrades_when_features_dir_missing() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "t", "backlog"));

    // @step Given there is no spec/features/ directory in the project root
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's linkedFeatures field is an empty array (the missing directory is NOT escalated)
    assert_eq!(data["linkedFeatures"].as_array().map(Vec::len), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Consolidates multiple system reminders into a single wrapped block
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn consolidates_multiple_system_reminders_into_single_block() {
    // @step Given spec/work-units.json contains AUTH-001 with status='specifying', no estimate, no rules, no examples
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &build_workunits("AUTH-001", "t", "story", "specifying", &[]),
    );

    // @step Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    unset_disable_reminders();

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's systemReminders array contains at least two entries (missing-estimate AND empty-example-mapping)
    let arr = data["systemReminders"]
        .as_array()
        .expect("systemReminders array");
    assert!(arr.len() >= 2, "expected >=2 reminders; got {arr:?}");

    // @step Then the JSON payload's systemReminder field is a single <system-reminder>…</system-reminder> block whose body joins the reminders with a blank line
    let block = data["systemReminder"]
        .as_str()
        .expect("systemReminder string");
    assert!(
        block.starts_with("<system-reminder>"),
        "must start with <system-reminder>: {block}"
    );
    assert!(
        block.ends_with("</system-reminder>"),
        "must end with </system-reminder>: {block}"
    );
    // a blank-line separator (\n\n) should appear inside the consolidated block
    assert!(
        block.contains("\n\n"),
        "consolidated block must join reminders with a blank line: {block}"
    );
    // There must be exactly one <system-reminder> open tag (re-wrapped).
    assert_eq!(
        block.matches("<system-reminder>").count(),
        1,
        "consolidated block must wrap exactly once: {block}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Emits the large-estimate reminder with the create-feature-file-first branch
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn emits_large_estimate_reminder_with_create_feature_file_first_branch() {
    // @step Given spec/work-units.json contains AUTH-001 with type='story', estimate=21, status='implementing'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "t",
            "story",
            "implementing",
            &[("estimate", "21")],
        ),
    );

    // @step Given there is no spec/features/ directory or no feature file tagged @AUTH-001
    assert!(!tmp.path().join("spec/features").exists());

    // @step Given the environment variable FSPEC_DISABLE_REMINDERS is unset
    unset_disable_reminders();

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload's systemReminders array contains a reminder whose body contains the exact substring "LARGE ESTIMATE WARNING"
    let arr = data["systemReminders"]
        .as_array()
        .expect("systemReminders array");
    let large_warn = arr
        .iter()
        .find(|v| {
            v.as_str()
                .map(|s| s.contains("LARGE ESTIMATE WARNING"))
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or_else(|| panic!("missing LARGE ESTIMATE WARNING reminder; got: {arr:?}"));

    // @step Then that same reminder contains the exact substring "CREATE FEATURE FILE FIRST"
    let body = large_warn.as_str().unwrap_or("");
    assert!(
        body.contains("CREATE FEATURE FILE FIRST"),
        "expected CREATE FEATURE FILE FIRST branch; got: {body}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Honours the FSPEC_DISABLE_REMINDERS=1 environment gate
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn honours_fspec_disable_reminders_env_gate() {
    // @step Given the environment variable FSPEC_DISABLE_REMINDERS is set to "1"
    std::env::set_var("FSPEC_DISABLE_REMINDERS", "1");
    // RAII guard so failures still restore env
    struct R;
    impl Drop for R {
        fn drop(&mut self) {
            std::env::remove_var("FSPEC_DISABLE_REMINDERS");
        }
    }
    let _guard = R;

    // @step Given spec/work-units.json contains AUTH-001 with status='specifying', no estimate, no rules, no examples
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &build_workunits("AUTH-001", "t", "story", "specifying", &[]),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the JSON payload omits both systemReminders and systemReminder
    let obj = data.as_object().expect("root object");
    assert!(
        !obj.contains_key("systemReminders"),
        "systemReminders must be omitted under FSPEC_DISABLE_REMINDERS=1"
    );
    assert!(
        !obj.contains_key("systemReminder"),
        "systemReminder must be omitted under FSPEC_DISABLE_REMINDERS=1"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text format renders a multi-section dump with type status epic and dependency lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn text_format_renders_multi_section_dump_with_type_status_epic_and_dependency_lines() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with title='Login', description='Implement auth', epic='auth', parent='RPC-003', blocks=['X-1','X-2'], rules=[{id:0,text:'must be 8+ chars',deleted:false}], examples=[{id:0,text:'happy path',deleted:false}], attachments=['spec/attachments/AUTH-001/diagram.png'], status='backlog'
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[{"id":0,"text":"must be 8+ chars","deleted":false,"createdAt":"x"}]"#;
    let examples = r#"[{"id":0,"text":"happy path","deleted":false,"createdAt":"x"}]"#;
    let attachments = r#"["spec/attachments/AUTH-001/diagram.png"]"#;
    let blocks = r#"["X-1","X-2"]"#;
    write_work_units(
        tmp.path(),
        &build_workunits(
            "AUTH-001",
            "Login",
            "story",
            "backlog",
            &[
                ("description", r#""Implement auth""#),
                ("epic", r#""auth""#),
                ("parent", r#""RPC-003""#),
                ("blocks", blocks),
                ("rules", rules),
                ("examples", examples),
                ("attachments", attachments),
            ],
        ),
    );

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"text"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let out = &result.data;

    // @step Then the DispatchResult.data contains the line "AUTH-001"
    assert!(
        out.lines().any(|l| l.contains("AUTH-001")),
        "missing AUTH-001 line; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "Type: story"
    assert!(
        out.contains("Type: story"),
        "missing Type: story; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "Status: backlog"
    assert!(
        out.contains("Status: backlog"),
        "missing Status: backlog; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the substring "Epic: auth"
    assert!(
        out.contains("Epic: auth"),
        "missing Epic: auth; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the substring "Parent: RPC-003"
    assert!(
        out.contains("Parent: RPC-003"),
        "missing Parent: RPC-003; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the substring "Blocks: X-1, X-2"
    assert!(
        out.contains("Blocks: X-1, X-2"),
        "missing Blocks: X-1, X-2; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "Rules:"
    assert!(
        out.lines().any(|l| l.trim_end() == "Rules:"),
        "missing Rules: header; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "  [0] must be 8+ chars"
    assert!(
        out.lines().any(|l| l == "  [0] must be 8+ chars"),
        "missing rule line '  [0] must be 8+ chars'; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "Examples:"
    assert!(
        out.lines().any(|l| l.trim_end() == "Examples:"),
        "missing Examples: header; got:\n{out}"
    );

    // @step Then the DispatchResult.data contains the line "  [0] happy path"
    assert!(
        out.lines().any(|l| l == "  [0] happy path"),
        "missing example line '  [0] happy path'; got:\n{out}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: JSON format emits a 2-space indented payload with the canonical field set
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn json_format_emits_two_space_indented_payload_with_canonical_field_set() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with title='x', status='backlog'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "x", "backlog"));

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data uses 2-space indentation
    // serde_json::to_string_pretty emits 2-space indentation; verify a top-level field has the 2-space prefix.
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"id\"")
            || l.starts_with("  \"title\"")
            || l.starts_with("  \"status\"")
            || l.starts_with("  \"linkedFeatures\"")),
        "expected a 2-space-indented root field; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data parses as JSON whose root contains id, title, type, status, createdAt, updatedAt, linkedFeatures
    let data = parse_data(&result.data);
    for k in [
        "id",
        "title",
        "type",
        "status",
        "createdAt",
        "updatedAt",
        "linkedFeatures",
    ] {
        assert!(
            data.get(k).is_some(),
            "missing root field `{k}`; got: {data}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Defaults to text format when the format argument is omitted
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn defaults_to_text_format_when_format_omitted() {
    unset_disable_reminders();
    // @step Given spec/work-units.json contains AUTH-001 with title='x', status='backlog'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_unit("AUTH-001", "x", "backlog"));

    // @step When I dispatch show-work-unit with workUnitId='AUTH-001' and no format field supplied
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId":"AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data starts with a section that contains the line "AUTH-001"
    assert!(
        result.data.lines().take(10).any(|l| l.contains("AUTH-001")),
        "expected AUTH-001 near the top of the text dump; got:\n{}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns a structured error when workUnitId is missing from the dispatcher args
// ─────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn returns_error_when_work_unit_id_missing_from_args() {
    unset_disable_reminders();
    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch show-work-unit with an empty args object
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message describes the missing workUnitId argument
    let msg = result.error.as_ref().expect("error message");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("workunitid")
            || lower.contains("work unit id")
            || lower.contains("workunit"),
        "error must mention missing workUnitId; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shared infrastructure delegation
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shared_infrastructure_delegation_uses_gherkin_crate() {
    // @step Given the rust/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect rust/fspec-core/src/commands/show_work_unit.rs
    let src_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/show_work_unit.rs");
    let src = fs::read_to_string(&src_path).expect("commands/show_work_unit.rs readable");

    // @step Then the file does NOT contain the substring "FspecCoreError::NotYetPorted"
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "commands/show_work_unit.rs must no longer be a NotYetPorted stub; got:\n{src}"
    );

    // @step Then the file uses the shared gherkin crate to parse feature files (mirroring show_feature.rs)
    assert!(
        src.contains("gherkin"),
        "commands/show_work_unit.rs must use the gherkin crate to parse feature files; got:\n{src}"
    );
}
