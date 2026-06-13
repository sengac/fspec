#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/show-deleted-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `show-deleted`
// (RPC-301). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-deleted".to_string(),
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

/// Build a `work-units.json` containing a single AUTH-001 entry whose
/// `rules` / `examples` / `questions` / `architectureNotes` arrays are
/// supplied verbatim as raw JSON array fragments. Empty fragments
/// (`""`) omit the field entirely so we can test the "absent arrays"
/// branch.
fn auth_001_workunits(
    rules: &str,
    examples: &str,
    questions: &str,
    architecture_notes: &str,
) -> String {
    let mut fields = String::from(
        r#""id":"AUTH-001","title":"t","status":"backlog","createdAt":"x","updatedAt":"x""#,
    );
    if !rules.is_empty() {
        fields.push_str(&format!(r#","rules":{rules}"#));
    }
    if !examples.is_empty() {
        fields.push_str(&format!(r#","examples":{examples}"#));
    }
    if !questions.is_empty() {
        fields.push_str(&format!(r#","questions":{questions}"#));
    }
    if !architecture_notes.is_empty() {
        fields.push_str(&format!(r#","architectureNotes":{architecture_notes}"#));
    }
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "AUTH-001": {{ {fields} }}
  }},
  "states": {{
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns deleted items in canonical concatenation order with only id text and deletedAt fields
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn returns_deleted_items_in_canonical_concatenation_order() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule 'first rule', one live rule, one live example, one deleted example 'first ex', one deleted question 'first q', and one deleted architecture note 'first note'
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"first rule","deleted":true,"createdAt":"x","deletedAt":"2025-01-31T12:00:00.000Z"},
        {"id":1,"text":"live rule","deleted":false,"createdAt":"x"}
    ]"#;
    let examples = r#"[
        {"id":0,"text":"live ex","deleted":false,"createdAt":"x"},
        {"id":1,"text":"first ex","deleted":true,"createdAt":"x","deletedAt":"2025-02-01T08:00:00.000Z"}
    ]"#;
    let questions = r#"[
        {"id":0,"text":"first q","deleted":true,"selected":false,"createdAt":"x","deletedAt":"2025-02-02T09:00:00.000Z"}
    ]"#;
    let arch_notes = r#"[
        {"id":0,"text":"first note","deleted":true,"createdAt":"x","deletedAt":"2025-02-03T10:00:00.000Z"}
    ]"#;
    write_work_units(
        tmp.path(),
        &auth_001_workunits(rules, examples, questions, arch_notes),
    );

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true with totalDeleted=4
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["totalDeleted"].as_u64(), Some(4));

    // @step Then the deletedItems text fields read 'first rule', 'first ex', 'first q', 'first note' in that exact order
    let items = data["deletedItems"].as_array().expect("deletedItems array");
    assert_eq!(items.len(), 4);
    assert_eq!(items[0]["text"].as_str(), Some("first rule"));
    assert_eq!(items[1]["text"].as_str(), Some("first ex"));
    assert_eq!(items[2]["text"].as_str(), Some("first q"));
    assert_eq!(items[3]["text"].as_str(), Some("first note"));

    // @step Then each deletedItems entry contains only id, text, and deletedAt fields and drops createdAt, selected, answered, and answer
    for entry in items {
        let obj = entry.as_object().expect("entry object");
        assert!(obj.contains_key("id"), "missing id field: {entry}");
        assert!(obj.contains_key("text"), "missing text field: {entry}");
        // deletedAt may be present or omitted depending on whether the
        // source carried it; here every test item has one.
        assert!(obj.contains_key("deletedAt"), "missing deletedAt: {entry}");
        for forbidden in &["createdAt", "selected", "answered", "answer", "deleted"] {
            assert!(
                !obj.contains_key(*forbidden),
                "entry must not carry `{forbidden}`: {entry}"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Auto-creates work-units.json and fails when the requested work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn auto_creates_work_units_json_and_fails_when_unit_missing() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch show-deleted with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId":"AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message containing the substring "Work unit 'AUTH-001' does not exist"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "error missing canonical substring: {msg}"
    );

    // @step Then spec/work-units.json exists after the call with an empty workUnits object
    let path = tmp.path().join("spec/work-units.json");
    assert!(
        path.exists(),
        "show-deleted MUST auto-create spec/work-units.json (load-or-init parity with TS)"
    );
    let raw = fs::read_to_string(&path).expect("read work-units.json");
    let parsed: Value = serde_json::from_str(&raw).expect("parse work-units.json");
    let wus = parsed["workUnits"].as_object().expect("workUnits object");
    assert!(wus.is_empty(), "expected empty workUnits, got {wus:?}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns empty deletedItems for a work unit that has never had soft-deletes
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn returns_empty_deleted_items_when_arrays_absent() {
    // @step Given spec/work-units.json contains a work unit AUTH-001 with NO rules, examples, questions, or architectureNotes arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &auth_001_workunits("", "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true with totalDeleted=0
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["totalDeleted"].as_u64(), Some(0));

    // @step Then the deletedItems array is empty
    assert_eq!(data["deletedItems"].as_array().map(Vec::len), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Excludes items whose deleted flag is false or missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn excludes_items_with_deleted_false_or_missing() {
    // @step Given spec/work-units.json contains AUTH-001 with one rule whose deleted=false and one rule with no deleted field
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"explicit live","deleted":false,"createdAt":"x"},
        {"id":1,"text":"no flag","createdAt":"x"}
    ]"#;
    write_work_units(tmp.path(), &auth_001_workunits(rules, "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true with totalDeleted=0
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["totalDeleted"].as_u64(), Some(0));

    // @step Then the deletedItems array is empty
    assert_eq!(data["deletedItems"].as_array().map(Vec::len), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Omits deletedAt from the JSON payload when the field is absent on the source item
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn omits_deleted_at_when_absent_on_source_item() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule that has NO deletedAt field
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":7,"text":"No timestamp","deleted":true,"createdAt":"x"}
    ]"#;
    write_work_units(tmp.path(), &auth_001_workunits(rules, "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));

    // @step Then the dispatcher returns success=true with totalDeleted=1
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["totalDeleted"].as_u64(), Some(1));

    // @step Then the first deletedItems entry contains id and text but the deletedAt field is omitted from the JSON
    let items = data["deletedItems"].as_array().expect("deletedItems array");
    let first = items[0].as_object().expect("object");
    assert_eq!(first.get("id").and_then(Value::as_u64), Some(7));
    assert_eq!(
        first.get("text").and_then(Value::as_str),
        Some("No timestamp")
    );
    assert!(
        !first.contains_key("deletedAt"),
        "deletedAt must be omitted when absent on source: {first:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text format renders the empty case as 'No deleted items found'
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn text_format_renders_empty_case_sentinel() {
    // @step Given spec/work-units.json contains AUTH-001 with no deleted items
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &auth_001_workunits("", "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"text"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No deleted items found'
    assert_eq!(
        result.data, "No deleted items found",
        "expected exact 'No deleted items found' sentinel; got: {:?}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text format renders the populated case with header item lines and timestamps
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn text_format_renders_populated_case_with_header_and_item_lines() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule (id=0, text='First rule', deletedAt='2025-01-31T12:00:00.000Z') and one deleted example (id=1, text='Second example', deletedAt='2025-02-01T08:00:00.000Z')
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"First rule","deleted":true,"createdAt":"x","deletedAt":"2025-01-31T12:00:00.000Z"}
    ]"#;
    let examples = r#"[
        {"id":1,"text":"Second example","deleted":true,"createdAt":"x","deletedAt":"2025-02-01T08:00:00.000Z"}
    ]"#;
    write_work_units(tmp.path(), &auth_001_workunits(rules, examples, "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"text"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Deleted items in AUTH-001 (2 total):'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Deleted items in AUTH-001 (2 total):"),
        "missing exact header line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  [0] First rule (deleted: 2025-01-31T12:00:00.000Z)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  [0] First rule (deleted: 2025-01-31T12:00:00.000Z)"),
        "missing exact rule line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  [1] Second example (deleted: 2025-02-01T08:00:00.000Z)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  [1] Second example (deleted: 2025-02-01T08:00:00.000Z)"),
        "missing exact example line; got:\n{}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text format omits the deleted timestamp suffix when deletedAt is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn text_format_omits_deleted_timestamp_suffix_when_missing() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule (id=7, text='No timestamp item') and NO deletedAt field on that rule
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":7,"text":"No timestamp item","deleted":true,"createdAt":"x"}
    ]"#;
    write_work_units(tmp.path(), &auth_001_workunits(rules, "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"text"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line '  [7] No timestamp item'
    assert!(
        result.data.lines().any(|l| l == "  [7] No timestamp item"),
        "missing exact item line '  [7] No timestamp item'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data does NOT contain the substring 'deleted:'
    assert!(
        !result.data.contains("deleted:"),
        "text format must NOT include 'deleted:' suffix when deletedAt is absent; got:\n{}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Defaults to text format when the format argument is omitted
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn defaults_to_text_format_when_format_omitted() {
    // @step Given spec/work-units.json contains AUTH-001 with no deleted items
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &auth_001_workunits("", "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and no format field supplied
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId":"AUTH-001"})));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No deleted items found'
    assert_eq!(
        result.data, "No deleted items found",
        "default text format must render sentinel exactly; got: {:?}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Returns a structured error when workUnitId is missing from the args
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn returns_error_when_work_unit_id_missing() {
    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch show-deleted with an empty args object
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message describing the missing workUnitId argument
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("workunitid")
            || lower.contains("work unit id")
            || lower.contains("workunit"),
        "error message must mention the missing workUnitId arg; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: JSON format emits 2-space indented payload with the canonical field set
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn json_format_emits_two_space_indent_with_canonical_field_set() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule (id=2, text='X', deletedAt='2025-06-01T00:00:00.000Z')
    let tmp = TempDir::new().expect("tempdir");
    let rules = r#"[
        {"id":2,"text":"X","deleted":true,"createdAt":"x","deletedAt":"2025-06-01T00:00:00.000Z"}
    ]"#;
    write_work_units(tmp.path(), &auth_001_workunits(rules, "", "", ""));

    // @step When I dispatch show-deleted with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId":"AUTH-001","format":"json"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root has success=true, workUnitId='AUTH-001', totalDeleted=1, and a deletedItems array of length 1
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["workUnitId"].as_str(), Some("AUTH-001"));
    assert_eq!(data["totalDeleted"].as_u64(), Some(1));
    assert_eq!(data["deletedItems"].as_array().map(Vec::len), Some(1));

    // @step Then the DispatchResult.data uses 2-space indentation
    // serde_json::to_string_pretty produces 2-space indent by default. We
    // verify the indentation pattern by walking the nested structure:
    //   level 1: `  "success"` (2 spaces — root field)
    //   level 2: `    {`        (4 spaces — array entry open brace)
    //   level 3: `      "id"`   (6 spaces — nested field)
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"success\""))
            || result
                .data
                .lines()
                .any(|l| l.starts_with("  \"workUnitId\""))
            || result
                .data
                .lines()
                .any(|l| l.starts_with("  \"deletedItems\"")),
        "expected a line starting with two-space-indented root field; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected a four-space-indented `{{` line opening the deletedItems entry; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l.starts_with("      \"id\"")),
        "expected a six-space-indented `\"id\"` nested-field line; got:\n{}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shared infrastructure delegation
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shared_infrastructure_delegation_uses_ensure_work_units_file() {
    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/commands/show_deleted.rs
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let src = fs::read_to_string(crate_src.join("commands/show_deleted.rs"))
        .expect("commands/show_deleted.rs readable");

    // @step Then the file calls io::ensure::ensure_work_units_file rather than embedding its own work-units.json read logic
    assert!(
        src.contains("ensure_work_units_file"),
        "commands/show_deleted.rs must delegate to the shared ensure_work_units_file helper; got:\n{src}"
    );

    // @step Then the file does NOT contain the substring 'FspecCoreError::NotYetPorted'
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "commands/show_deleted.rs must no longer be a NotYetPorted stub; got:\n{src}"
    );
}
