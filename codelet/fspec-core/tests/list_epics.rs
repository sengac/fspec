#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/list-epics-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-epics`
// (RPC-243). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-epics".to_string(),
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

fn first_epic(data: &Value) -> &Value {
    &data["epics"].as_array().expect("epics array")[0]
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

// ---------- scenarios ----------

#[test]
fn returns_empty_epics_list_when_spec_does_not_exist() {
    // Scenario: Returns an empty epics list when spec/ does not exist and does not auto-create files

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the list-epics command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true with an empty epics array
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["epics"].as_array().map(Vec::len),
        Some(0),
        "expected empty epics array, got {}",
        result.data
    );

    // @step Then spec/epics.json does not exist after the call
    assert!(
        !tmp.path().join("spec/epics.json").exists(),
        "list-epics must NOT auto-create spec/epics.json"
    );

    // @step Then spec/work-units.json does not exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "list-epics must NOT auto-create spec/work-units.json"
    );
}

#[test]
fn aggregates_work_unit_completion_progress_per_epic_by_exact_match() {
    // Scenario: Aggregates work-unit completion progress per epic by exact-match

    // @step Given spec/epics.json contains auth (title 'Authentication', description 'Login features') and dash (title 'Dashboard', description 'Dashboard features') in that order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "epics": {
    "auth": { "id": "auth", "title": "Authentication", "description": "Login features", "createdAt": "x" },
    "dash": { "id": "dash", "title": "Dashboard", "description": "Dashboard features", "createdAt": "x" }
  }
}"#;
    write_epics(tmp.path(), raw);

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), DASH-001 (epic=dash, status=done), DASH-002 (epic=dash, status=done)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "backlog"),
            ("DASH-001", "dash", "done"),
            ("DASH-002", "dash", "done"),
        ]),
    );

    // @step When I dispatch list-epics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["epics"].as_array().expect("epics array");

    // @step Then the epics array contains exactly two entries in order auth then dash
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");
    assert_eq!(arr[0]["id"].as_str(), Some("auth"));
    assert_eq!(arr[1]["id"].as_str(), Some("dash"));

    // @step Then the auth entry has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    assert_eq!(arr[0]["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[0]["completedWorkUnits"].as_u64(), Some(1));
    assert_eq!(arr[0]["completionPercentage"].as_u64(), Some(50));

    // @step Then the dash entry has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100
    assert_eq!(arr[1]["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[1]["completedWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[1]["completionPercentage"].as_u64(), Some(100));
}

#[test]
fn treats_missing_work_units_json_as_zero_counts() {
    // Scenario: Treats missing work-units.json as zero counts without throwing

    // @step Given spec/epics.json contains auth (title 'Authentication')
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch list-epics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the auth entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    let auth = first_epic(&data);
    assert_eq!(auth["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completionPercentage"].as_u64(), Some(0));
}

#[test]
fn treats_malformed_work_units_json_as_zero_counts() {
    // Scenario: Treats malformed work-units.json as zero counts without throwing

    // @step Given spec/epics.json contains auth (title 'Authentication')
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch list-epics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed work-units.json must be silently swallowed: {result:?}"
    );

    // @step Then the auth entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    let auth = first_epic(&data);
    assert_eq!(auth["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completionPercentage"].as_u64(), Some(0));
}

#[test]
fn escalates_malformed_epics_json_as_structured_parse_error() {
    // Scenario: Escalates malformed epics.json as a structured parse error

    // @step Given spec/epics.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), "{ not valid json");

    // @step When I dispatch list-epics against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse epics.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse epics.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn preserves_insertion_order_not_alphabetical() {
    // Scenario: Preserves insertion order of epics.json (not alphabetical)

    // @step Given spec/epics.json contains three epics registered in order zed, aaa, mid
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "epics": {
    "zed": { "id": "zed", "title": "Z title" },
    "aaa": { "id": "aaa", "title": "A title" },
    "mid": { "id": "mid", "title": "M title" }
  }
}"#;
    write_epics(tmp.path(), raw);

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the substring 'Epics (3)'
    assert!(
        result.data.contains("Epics (3)"),
        "missing header: {}",
        result.data
    );

    // @step Then the substring 'zed' appears before 'aaa' which appears before 'mid' in the output
    let zed = result.data.find("zed").expect("zed present");
    let aaa = result.data.find("aaa").expect("aaa present");
    let mid = result.data.find("mid").expect("mid present");
    assert!(
        zed < aaa && aaa < mid,
        "expected insertion order zed < aaa < mid; got zed={zed} aaa={aaa} mid={mid}\n{}",
        result.data
    );
}

#[test]
fn text_format_omits_work_units_line_when_total_is_zero() {
    // Scenario: Text format omits the 'Work Units' line when totalWorkUnits is zero

    // @step Given spec/epics.json contains auth with title 'Authentication' and description 'Login features'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication", "description": "Login features" } } }"#,
    );

    // @step Given spec/work-units.json contains no work units whose epic equals 'auth'
    write_work_units(
        tmp.path(),
        &work_units_raw(&[("OTHER-001", "other", "done")]),
    );

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'auth'
    assert!(
        result.data.lines().any(|l| l == "auth"),
        "expected exact 'auth' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line '  Authentication'
    assert!(
        result.data.lines().any(|l| l == "  Authentication"),
        "expected exact '  Authentication' title line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line '  Login features'
    assert!(
        result.data.lines().any(|l| l == "  Login features"),
        "expected exact '  Login features' description line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data does NOT contain the substring 'Work Units:'
    assert!(
        !result.data.contains("Work Units:"),
        "must NOT print 'Work Units:' line when totalWorkUnits is 0; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_omits_description_line_when_missing() {
    // Scenario: Text format omits the description line when the description is missing

    // @step Given spec/epics.json contains auth with title 'Authentication' and no description field
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'auth'
    assert!(
        result.data.lines().any(|l| l == "auth"),
        "expected exact 'auth' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line '  Authentication'
    assert!(
        result.data.lines().any(|l| l == "  Authentication"),
        "expected exact '  Authentication' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data does NOT contain the substring '  Login features'
    assert!(
        !result.data.contains("  Login features"),
        "must NOT contain '  Login features' when description is missing; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_renders_progress_with_math_round_one_third() {
    // Scenario: Text format renders completion progress with Math.round semantics (1/3 rounds to 33)

    // @step Given spec/epics.json contains auth with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=backlog), AUTH-003 (epic=auth, status=backlog)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "backlog"),
            ("AUTH-003", "auth", "backlog"),
        ]),
    );

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line '  Work Units: 1/3 (33%)'
    assert!(
        result.data.lines().any(|l| l == "  Work Units: 1/3 (33%)"),
        "missing exact progress line '  Work Units: 1/3 (33%)'; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_rounds_two_thirds_to_sixty_seven_percent() {
    // Scenario: Text format rounds 2/3 progress to 67 percent

    // @step Given spec/epics.json contains auth with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json contains AUTH-001 (epic=auth, status=done), AUTH-002 (epic=auth, status=done), AUTH-003 (epic=auth, status=backlog)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "auth", "done"),
            ("AUTH-002", "auth", "done"),
            ("AUTH-003", "auth", "backlog"),
        ]),
    );

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line '  Work Units: 2/3 (67%)'
    assert!(
        result.data.lines().any(|l| l == "  Work Units: 2/3 (67%)"),
        "missing exact progress line '  Work Units: 2/3 (67%)'; got:\n{}",
        result.data
    );
}

#[test]
fn json_format_emits_two_space_indent_omitting_unset_optional_fields() {
    // Scenario: JSON format emits two-space indented payload omitting unset optional fields

    // @step Given spec/epics.json contains auth with title 'Authentication' and no description
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch list-epics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has an 'epics' array of length 1
    let data = parse_data(&result.data);
    let arr = data["epics"].as_array().expect("epics array");
    assert_eq!(arr.len(), 1);

    // @step Then the first epics entry has id='auth', title='Authentication', totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let entry = &arr[0];
    assert_eq!(entry["id"].as_str(), Some("auth"));
    assert_eq!(entry["title"].as_str(), Some("Authentication"));
    assert_eq!(entry["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(entry["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(entry["completionPercentage"].as_u64(), Some(0));

    // @step Then the first epics entry does NOT contain a 'description' key
    let obj = entry.as_object().expect("entry is object");
    assert!(
        !obj.contains_key("description"),
        "description key must be omitted when None; got entry: {entry}"
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"epics\"")),
        "expected a line starting with two-space indent + \"epics\"; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected a four-space-indented `{{` line opening the epics entry; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l.starts_with("      \"id\"")),
        "expected a line starting with six-space indent + \"id\"; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_prints_no_epics_found_for_empty_epics_object() {
    // Scenario: Text format prints 'No epics found' for an empty epics object

    // @step Given spec/epics.json exists with an empty epics object
    let tmp = TempDir::new().expect("tempdir");
    write_epics(tmp.path(), r#"{ "epics": {} }"#);

    // @step When I dispatch list-epics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No epics found'
    assert_eq!(
        result.data, "No epics found",
        "expected exact 'No epics found' sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn work_units_with_unmatched_epic_field_are_ignored() {
    // Scenario: Work units with unmatched epic field are ignored by aggregation

    // @step Given spec/epics.json contains auth with title 'Authentication'
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{ "epics": { "auth": { "id": "auth", "title": "Authentication" } } }"#,
    );

    // @step Given spec/work-units.json contains AUTH-001 (epic=nonexistent, status=done) and AUTH-002 (epic=auth, status=done)
    write_work_units(
        tmp.path(),
        &work_units_raw(&[
            ("AUTH-001", "nonexistent", "done"),
            ("AUTH-002", "auth", "done"),
        ]),
    );

    // @step When I dispatch list-epics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the auth entry has totalWorkUnits=1, completedWorkUnits=1, completionPercentage=100
    let data = parse_data(&result.data);
    let auth = first_epic(&data);
    assert_eq!(auth["totalWorkUnits"].as_u64(), Some(1));
    assert_eq!(auth["completedWorkUnits"].as_u64(), Some(1));
    assert_eq!(auth["completionPercentage"].as_u64(), Some(100));
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under codelet/fspec-core for reuse

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the module io::ensure::read_epics_or_empty exists and is publicly accessible from the crate root
    let ensure_src =
        fs::read_to_string(crate_src.join("io/ensure.rs")).expect("io/ensure.rs readable");
    assert!(
        ensure_src.contains("pub fn read_epics_or_empty"),
        "io/ensure.rs must declare `pub fn read_epics_or_empty`; got:\n{ensure_src}"
    );

    // @step Then types::epic::Epic exists and EpicsData.epics is keyed by an IndexMap to preserve insertion order
    let epic_path = crate_src.join("types/epic.rs");
    assert!(
        epic_path.exists(),
        "types/epic.rs must exist; got missing: {}",
        epic_path.display()
    );
    let epic_src = fs::read_to_string(&epic_path).expect("types/epic.rs readable");
    assert!(
        epic_src.contains("pub struct Epic"),
        "types/epic.rs must declare `pub struct Epic`; got:\n{epic_src}"
    );
    let work_unit_src = fs::read_to_string(crate_src.join("types/work_unit.rs"))
        .expect("types/work_unit.rs readable");
    let uses_indexmap_of_epic = work_unit_src.contains("IndexMap<String, Epic>")
        || work_unit_src.contains("IndexMap<String, crate::types::epic::Epic>");
    assert!(
        uses_indexmap_of_epic,
        "EpicsData.epics must be `IndexMap<String, Epic>` (or fully-qualified equivalent) to preserve insertion order; got:\n{work_unit_src}"
    );

    // @step Then list_epics::run delegates to these shared modules rather than embedding its own filesystem logic
    let list_src = fs::read_to_string(crate_src.join("commands/list_epics.rs"))
        .expect("commands/list_epics.rs readable");
    assert!(
        list_src.contains("read_epics_or_empty"),
        "commands/list_epics.rs must delegate to shared io helpers; got:\n{list_src}"
    );
    // Scope the absence check to the run() function body only — the divider
    // comment block and inline tests legitimately mention the variant name in
    // documentation/assertions, so a file-wide substring check is too broad.
    let run_start = list_src
        .find("pub async fn run(")
        .expect("run() not found in list_epics.rs");
    let run_end = list_src[run_start..]
        .find("\nfn ")
        .or_else(|| list_src[run_start..].find("\n#[cfg(test)]"))
        .map(|i| run_start + i)
        .unwrap_or(list_src.len());
    let run_body = &list_src[run_start..run_end];
    assert!(
        !run_body.contains("FspecCoreError::NotYetPorted"),
        "list_epics::run must not emit NotYetPorted; got run body:\n{run_body}"
    );
}
