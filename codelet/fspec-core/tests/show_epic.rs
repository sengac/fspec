#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/show-epic-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `show-epic`
// (RPC-302). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-epic".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_epics(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
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

// Build a work-units.json string preserving key insertion order.
fn work_units_raw(entries: &[(&str, &str, &str)]) -> String {
    // entries: (id, epic, status)
    let mut wu_body = String::new();
    for (i, (id, epic, status)) in entries.iter().enumerate() {
        if i > 0 {
            wu_body.push(',');
        }
        wu_body.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"t","epic":"{epic}","status":"{status}","createdAt":"x","updatedAt":"x"}}"#
        ));
    }
    format!(
        r#"{{"version":"0.7.1","workUnits":{{{wu_body}}},"states":{{"backlog":[],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}}}}"#
    )
}

// Build a minimal epics.json with a single epic entry.
fn epics_raw_single(id: &str, title: Option<&str>, description: Option<&str>) -> String {
    let mut body = format!(r#""id":"{id}""#);
    if let Some(t) = title {
        body.push_str(&format!(r#","title":"{t}""#));
    }
    if let Some(d) = description {
        body.push_str(&format!(r#","description":"{d}""#));
    }
    body.push_str(r#","createdAt":"x""#);
    format!(r#"{{"epics":{{"{id}":{{{body}}}}}}}"#)
}

// ---------- scenarios ----------

#[test]
fn returns_epic_not_found_when_epics_json_missing() {
    // Scenario: Returns Epic not found error when spec/epics.json is missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the show-epic command with epicId='auth' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth" })));

    // @step Then the dispatcher returns success=false with an error message exactly 'Epic auth not found'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Epic auth not found"),
        "error must contain canonical substring; got: {msg}"
    );

    // @step Then spec/epics.json does not exist after the call
    assert!(
        !tmp.path().join("spec/epics.json").exists(),
        "show-epic must NOT auto-create spec/epics.json"
    );

    // @step Then spec/work-units.json does not exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "show-epic must NOT auto-create spec/work-units.json"
    );
}

#[test]
fn returns_epic_not_found_when_id_not_registered() {
    // Scenario: Returns Epic not found error when epicId is not registered in epics.json

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step When I dispatch show-epic with epicId='nonexistent'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "nonexistent" })));

    // @step Then the dispatcher returns success=false with an error message exactly 'Epic nonexistent not found'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Epic nonexistent not found"),
        "error must contain canonical substring; got: {msg}"
    );
}

#[test]
fn escalates_malformed_epics_json_parse_error() {
    // Scenario: Escalates malformed epics.json as a structured parse error

    // @step Given spec/epics.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), "{ not valid json");

    // @step When I dispatch show-epic with epicId='auth' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse epics.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse epics.json"),
        "error message missing canonical substring; got: {msg}"
    );
}

#[test]
fn aggregates_work_unit_completion_progress() {
    // Scenario: Aggregates work-unit completion progress for the requested epic

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication' and description 'Login features'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        &epics_raw_single("auth", Some("Authentication"), Some("Login features")),
    );

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), DASH-001 (epic=dash, status=done)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "backlog"),
            ("DASH-001", "dash", "done"),
        ]),
    );

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the result has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(1));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!((pct - 50.0).abs() < 1e-9, "expected 50; got {pct}");

    // @step Then the result.epic.id equals 'auth'
    assert_eq!(data["epic"]["id"].as_str(), Some("auth"));
}

#[test]
fn treats_missing_work_units_as_zero_counts() {
    // Scenario: Treats missing work-units.json as zero counts without throwing

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the result has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(0));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!((pct - 0.0).abs() < 1e-9, "expected 0; got {pct}");
}

#[test]
fn treats_malformed_work_units_as_zero_counts() {
    // Scenario: Treats malformed work-units.json as zero counts without throwing

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed work-units.json must be silently swallowed: {result:?}"
    );

    // @step Then the result has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(0));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!((pct - 0.0).abs() < 1e-9, "expected 0; got {pct}");
}

#[test]
fn completion_percentage_one_third_rounds_to_33_33() {
    // Scenario: completionPercentage rounds 1/3 to 33.33 with 2-decimal precision

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), AUTH-003 (epic=auth, status=backlog)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "backlog"),
            ("AUTH-003", "auth", "backlog"),
        ]),
    );

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the result has totalWorkUnits=3, completedWorkUnits=1, completionPercentage=33.33
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(3));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(1));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!(
        (pct - 33.33).abs() < 1e-9,
        "expected 33.33 (2-decimal rounding); got {pct}"
    );
}

#[test]
fn completion_percentage_two_thirds_rounds_to_66_67() {
    // Scenario: completionPercentage rounds 2/3 to 66.67 with 2-decimal precision

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "done"),
            ("AUTH-003", "auth", "backlog"),
        ]),
    );

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the result has totalWorkUnits=3, completedWorkUnits=2, completionPercentage=66.67
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(3));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(2));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!(
        (pct - 66.67).abs() < 1e-9,
        "expected 66.67 (2-decimal rounding); got {pct}"
    );
}

#[test]
fn completion_percentage_returns_100_when_all_done() {
    // Scenario: completionPercentage returns 100 when every work unit is done

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done) and AUTH-002 (epic=auth, status=done)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[("AUTH-001", "auth", "done"), ("AUTH-002", "auth", "done")]),
    );

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the result has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100
    let data = parse_data(&result.data);
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(2));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!((pct - 100.0).abs() < 1e-9, "expected 100; got {pct}");
}

#[test]
fn text_format_renders_epic_header_and_progress() {
    // Scenario: Text format renders the Epic header Title Description and Progress block

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication' and description 'Login features'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        &epics_raw_single("auth", Some("Authentication"), Some("Login features")),
    );

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog), AUTH-004 (epic=auth, status=backlog)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "done"),
            ("AUTH-003", "auth", "backlog"),
            ("AUTH-004", "auth", "backlog"),
        ]),
    );

    // @step When I dispatch show-epic with epicId='auth' and format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Epic: auth'
    assert!(
        result.data.lines().any(|l| l == "Epic: auth"),
        "missing 'Epic: auth' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Title: Authentication'
    assert!(
        result.data.lines().any(|l| l == "Title: Authentication"),
        "missing 'Title: Authentication' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Description: Login features'
    assert!(
        result.data.lines().any(|l| l == "Description: Login features"),
        "missing 'Description: Login features' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Progress:'
    assert!(
        result.data.lines().any(|l| l == "Progress:"),
        "missing 'Progress:' header; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  Total work units: 4'
    assert!(
        result.data.lines().any(|l| l == "  Total work units: 4"),
        "missing exact line '  Total work units: 4'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  Completed: 2'
    assert!(
        result.data.lines().any(|l| l == "  Completed: 2"),
        "missing exact line '  Completed: 2'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  Completion: 50%'
    assert!(
        result.data.lines().any(|l| l == "  Completion: 50%"),
        "missing exact line '  Completion: 50%'; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_omits_description_when_missing() {
    // Scenario: Text format omits the Description line when description is missing

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication' and no description field
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-epic with epicId='auth' and format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Epic: auth'
    assert!(
        result.data.lines().any(|l| l == "Epic: auth"),
        "missing 'Epic: auth' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Title: Authentication'
    assert!(
        result.data.lines().any(|l| l == "Title: Authentication"),
        "missing 'Title: Authentication' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data does NOT contain the substring 'Description:'
    assert!(
        !result.data.contains("Description:"),
        "must NOT print 'Description:' when description is missing; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_renders_title_n_a_when_title_missing() {
    // Scenario: Text format renders Title N/A when epic title is missing

    // @step Given spec/epics.json contains epic 'auth' with no title field
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", None, None));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-epic with epicId='auth' and format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Title: N/A'
    assert!(
        result.data.lines().any(|l| l == "Title: N/A"),
        "missing 'Title: N/A' line; got:\n{}",
        result.data
    );
}

#[test]
fn default_format_is_text() {
    // Scenario: Default format is text when format flag is not supplied

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-epic with epicId='auth' and no format flag
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'Epic: auth'
    assert!(
        result.data.lines().any(|l| l == "Epic: auth"),
        "missing 'Epic: auth' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Title: Authentication'
    assert!(
        result.data.lines().any(|l| l == "Title: Authentication"),
        "missing 'Title: Authentication' line; got:\n{}",
        result.data
    );
}

#[test]
fn json_format_emits_two_space_indent_with_canonical_fields() {
    // Scenario: JSON format emits two-space indented payload with the canonical field set

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch show-epic with epicId='auth' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "epicId": "auth", "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has an 'epic' object key
    let data = parse_data(&result.data);
    assert!(
        data["epic"].is_object(),
        "root.epic must be an object; got:\n{}",
        result.data
    );

    // @step Then the root object has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    assert_eq!(data["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(data["completedWorkUnits"].as_u64(), Some(0));
    let pct = data["completionPercentage"].as_f64().expect("completionPercentage f64");
    assert!((pct - 0.0).abs() < 1e-9, "expected 0; got {pct}");

    // @step Then the root.epic object has id='auth' and title='Authentication'
    assert_eq!(data["epic"]["id"].as_str(), Some("auth"));
    assert_eq!(data["epic"]["title"].as_str(), Some("Authentication"));

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"epic\"")),
        "expected a line starting with two-space indent + \"epic\"; got:\n{}",
        result.data
    );
}

#[test]
fn missing_epic_id_surfaces_invalid_args_error() {
    // Scenario: Missing epicId argument surfaces a structured InvalidArgs error

    // @step Given spec/epics.json contains epic 'auth' with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), &epics_raw_single("auth", Some("Authentication"), None));

    // @step When I dispatch show-epic with no epicId argument
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'failed to parse args'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("failed to parse args"),
        "expected 'failed to parse args' substring; got: {msg}"
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules already exist for reuse

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the module io::ensure::read_work_units_or_empty exists and is publicly accessible from the crate root
    let ensure_src = fs::read_to_string(crate_src.join("io/ensure.rs"))
        .expect("io/ensure.rs readable");
    assert!(
        ensure_src.contains("pub fn read_work_units_or_empty"),
        "io/ensure.rs must declare `pub fn read_work_units_or_empty`; got:\n{ensure_src}"
    );

    // @step Then types::epic::Epic exists with id, title, description and a flatten extra map
    let epic_src = fs::read_to_string(crate_src.join("types/epic.rs"))
        .expect("types/epic.rs readable");
    assert!(
        epic_src.contains("pub struct Epic"),
        "types/epic.rs must declare `pub struct Epic`; got:\n{epic_src}"
    );
    assert!(
        epic_src.contains("pub id: String"),
        "Epic.id must be String; got:\n{epic_src}"
    );
    assert!(
        epic_src.contains("title: Option<String>") && epic_src.contains("description: Option<String>"),
        "Epic must expose Option<String> title and description; got:\n{epic_src}"
    );
    assert!(
        epic_src.contains("#[serde(flatten)]"),
        "Epic must carry #[serde(flatten)] extra map for forward-compat; got:\n{epic_src}"
    );

    // @step Then commands/show_epic.rs delegates to these shared modules rather than embedding its own filesystem logic
    let show_src = fs::read_to_string(crate_src.join("commands/show_epic.rs"))
        .expect("commands/show_epic.rs readable");
    assert!(
        show_src.contains("read_work_units_or_empty"),
        "commands/show_epic.rs must delegate to shared io::ensure::read_work_units_or_empty helper; got:\n{show_src}"
    );

    // @step Then commands/show_epic.rs does NOT return FspecCoreError::NotYetPorted
    assert!(
        !show_src.contains("FspecCoreError::NotYetPorted"),
        "commands/show_epic.rs must no longer be a NotYetPorted stub"
    );
}
