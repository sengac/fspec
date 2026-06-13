#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/list-prefixes-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-prefixes`
// (RPC-248). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-prefixes".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_prefixes(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("prefixes.json"), raw).expect("write prefixes.json");
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn prefixes_with(entries: &[(&str, &str)]) -> String {
    let mut obj = serde_json::Map::new();
    for (key, desc) in entries {
        obj.insert(
            key.to_string(),
            json!({
                "prefix": key,
                "description": desc,
                "createdAt": "2026-06-01T00:00:00.000Z"
            }),
        );
    }
    serde_json::to_string_pretty(&json!({ "prefixes": Value::Object(obj) })).unwrap()
}

fn work_units_with(entries: &[(&str, &str)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status) in entries {
        wus.insert(
            id.to_string(),
            json!({
                "id": id,
                "title": format!("title for {id}"),
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }),
        );
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn first_prefix(data: &Value) -> &Value {
    &data["prefixes"].as_array().expect("prefixes array")[0]
}

// ---------- scenarios ----------

#[test]
fn returns_empty_prefixes_list_when_spec_does_not_exist() {
    // Scenario: Returns an empty prefixes list when spec/ does not exist and does not auto-create files

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the list-prefixes command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true with an empty prefixes array
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["prefixes"].as_array().map(Vec::len),
        Some(0),
        "expected empty prefixes array, got {}",
        result.data
    );

    // @step Then spec/prefixes.json does not exist after the call
    assert!(
        !tmp.path().join("spec/prefixes.json").exists(),
        "list-prefixes must NOT auto-create spec/prefixes.json"
    );

    // @step Then spec/work-units.json does not exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "list-prefixes must NOT auto-create spec/work-units.json"
    );
}

#[test]
fn aggregates_work_unit_completion_progress_per_prefix() {
    // Scenario: Aggregates work-unit completion progress per prefix

    // @step Given spec/prefixes.json contains AUTH (description 'Auth features') and DASH (description 'Dashboard') in that order
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(
        tmp.path(),
        &prefixes_with(&[("AUTH", "Auth features"), ("DASH", "Dashboard")]),
    );

    // @step Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), DASH-001 (done), DASH-002 (done)
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "done"),
            ("AUTH-002", "backlog"),
            ("DASH-001", "done"),
            ("DASH-002", "done"),
        ]),
    );

    // @step When I dispatch list-prefixes with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["prefixes"].as_array().expect("prefixes array");

    // @step Then the prefixes array contains exactly two entries in order AUTH then DASH
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");
    assert_eq!(arr[0]["prefix"].as_str(), Some("AUTH"));
    assert_eq!(arr[1]["prefix"].as_str(), Some("DASH"));

    // @step Then the AUTH entry has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    assert_eq!(arr[0]["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[0]["completedWorkUnits"].as_u64(), Some(1));
    assert_eq!(arr[0]["completionPercentage"].as_u64(), Some(50));

    // @step Then the DASH entry has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100
    assert_eq!(arr[1]["totalWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[1]["completedWorkUnits"].as_u64(), Some(2));
    assert_eq!(arr[1]["completionPercentage"].as_u64(), Some(100));
}

#[test]
fn treats_missing_work_units_json_as_zero_counts() {
    // Scenario: Treats missing work-units.json as zero counts without throwing

    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch list-prefixes with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the AUTH entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    let auth = first_prefix(&data);
    assert_eq!(auth["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completionPercentage"].as_u64(), Some(0));
}

#[test]
fn treats_malformed_work_units_json_as_zero_counts() {
    // Scenario: Treats malformed work-units.json as zero counts without throwing

    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch list-prefixes with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "malformed work-units.json must be silently swallowed: {result:?}"
    );

    // @step Then the AUTH entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let data = parse_data(&result.data);
    let auth = first_prefix(&data);
    assert_eq!(auth["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(auth["completionPercentage"].as_u64(), Some(0));
}

#[test]
fn escalates_malformed_prefixes_json_as_structured_parse_error() {
    // Scenario: Escalates malformed prefixes.json as a structured parse error

    // @step Given spec/prefixes.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), "{ not valid json");

    // @step When I dispatch list-prefixes against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse prefixes.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn preserves_insertion_order_not_alphabetical() {
    // Scenario: Preserves insertion order of prefixes.json (not alphabetical)

    // @step Given spec/prefixes.json contains three prefixes registered in order ZED, AAA, MID
    let tmp = TempDir::new().expect("tempdir");
    // Hand-write the JSON so the upstream object key order is preserved on
    // the wire. (json! / serde_json::Map would alphabetize.)
    let raw = r#"{
  "prefixes": {
    "ZED": { "prefix": "ZED", "description": "Z desc", "createdAt": "x" },
    "AAA": { "prefix": "AAA", "description": "A desc", "createdAt": "x" },
    "MID": { "prefix": "MID", "description": "M desc", "createdAt": "x" }
  }
}"#;
    write_prefixes(tmp.path(), raw);

    // @step When I dispatch list-prefixes with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the substring 'Prefixes (3)'
    assert!(
        result.data.contains("Prefixes (3)"),
        "missing header: {}",
        result.data
    );

    // @step Then the substring 'ZED' appears before 'AAA' which appears before 'MID' in the output
    let zed = result.data.find("ZED").expect("ZED present");
    let aaa = result.data.find("AAA").expect("AAA present");
    let mid = result.data.find("MID").expect("MID present");
    assert!(
        zed < aaa && aaa < mid,
        "expected insertion order ZED < AAA < MID; got ZED={zed} AAA={aaa} MID={mid}\n{}",
        result.data
    );
}

#[test]
fn text_format_omits_work_units_line_when_total_is_zero() {
    // Scenario: Text format omits the 'Work Units' line when totalWorkUnits is zero

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json contains no work units whose id starts with 'AUTH-'
    write_work_units(
        tmp.path(),
        &work_units_with(&[("OTHER-001", "done"), ("OTHER-002", "backlog")]),
    );

    // @step When I dispatch list-prefixes with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line 'AUTH'
    assert!(
        result.data.lines().any(|l| l == "AUTH"),
        "expected an exact 'AUTH' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line '  Auth features'
    assert!(
        result.data.lines().any(|l| l == "  Auth features"),
        "expected an exact '  Auth features' description line; got:\n{}",
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
fn text_format_renders_progress_with_math_round_one_third() {
    // Scenario: Text format renders completion progress with Math.round percentage semantics

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), AUTH-003 (backlog)
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "done"),
            ("AUTH-002", "backlog"),
            ("AUTH-003", "backlog"),
        ]),
    );

    // @step When I dispatch list-prefixes with format='text'
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

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (done), AUTH-003 (backlog)
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("AUTH-001", "done"),
            ("AUTH-002", "done"),
            ("AUTH-003", "backlog"),
        ]),
    );

    // @step When I dispatch list-prefixes with format='text'
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
fn json_format_emits_two_space_indent_with_canonical_field_set() {
    // Scenario: JSON format emits two-space indented payload with the canonical field set

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), &prefixes_with(&[("AUTH", "Auth features")]));

    // @step Given spec/work-units.json does NOT exist
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch list-prefixes with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has a 'prefixes' array of length 1
    let data = parse_data(&result.data);
    let arr = data["prefixes"].as_array().expect("prefixes array");
    assert_eq!(arr.len(), 1);

    // @step Then the first prefixes entry contains fields prefix='AUTH', description='Auth features', totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    let entry = &arr[0];
    assert_eq!(entry["prefix"].as_str(), Some("AUTH"));
    assert_eq!(entry["description"].as_str(), Some("Auth features"));
    assert_eq!(entry["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(entry["completedWorkUnits"].as_u64(), Some(0));
    assert_eq!(entry["completionPercentage"].as_u64(), Some(0));

    // @step Then the DispatchResult.data uses 2-space indentation
    // serde_json::to_string_pretty produces 2-space indent by default. We
    // verify the indentation pattern by walking the nested structure:
    //   level 1: `  "prefixes"` (2 spaces — root field)
    //   level 2: `    {`        (4 spaces — array entry open brace)
    //   level 3: `      "prefix"` (6 spaces — nested field)
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"prefixes\"")),
        "expected a line starting with two-space indent + \"prefixes\"; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected a four-space-indented `{{` line opening the prefixes entry; got:\n{}",
        result.data
    );
    assert!(
        result
            .data
            .lines()
            .any(|l| l.starts_with("      \"prefix\"")),
        "expected a line starting with six-space indent + \"prefix\"; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_prints_no_prefixes_found_for_empty_prefixes_object() {
    // Scenario: Text format prints 'No prefixes found' for an empty prefixes file

    // @step Given spec/prefixes.json exists with an empty prefixes object
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), r#"{ "prefixes": {} }"#);

    // @step When I dispatch list-prefixes with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No prefixes found'
    assert_eq!(
        result.data, "No prefixes found",
        "expected exact 'No prefixes found' sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under codelet/fspec-core for reuse by other commands

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the modules io::ensure::read_prefixes_or_empty and io::ensure::read_work_units_or_empty exist and are publicly accessible from the crate root
    let ensure_src =
        fs::read_to_string(crate_src.join("io/ensure.rs")).expect("io/ensure.rs readable");
    assert!(
        ensure_src.contains("pub fn read_prefixes_or_empty"),
        "io/ensure.rs must declare `pub fn read_prefixes_or_empty`; got:\n{ensure_src}"
    );
    assert!(
        ensure_src.contains("pub fn read_work_units_or_empty"),
        "io/ensure.rs must declare `pub fn read_work_units_or_empty`; got:\n{ensure_src}"
    );

    // @step Then types::prefix::Prefix exists and PrefixesData.prefixes is keyed by an IndexMap to preserve insertion order
    let prefix_path: PathBuf = crate_src.join("types/prefix.rs");
    assert!(
        prefix_path.exists(),
        "types/prefix.rs must exist; got missing: {}",
        prefix_path.display()
    );
    let prefix_src = fs::read_to_string(&prefix_path).expect("types/prefix.rs readable");
    assert!(
        prefix_src.contains("pub struct Prefix"),
        "types/prefix.rs must declare `pub struct Prefix`; got:\n{prefix_src}"
    );
    let work_unit_src = fs::read_to_string(crate_src.join("types/work_unit.rs"))
        .expect("types/work_unit.rs readable");
    // Accept either the short form `IndexMap<String, Prefix>` or the
    // fully-qualified form `IndexMap<String, crate::types::prefix::Prefix>`.
    let uses_indexmap_of_prefix = work_unit_src.contains("IndexMap<String, Prefix>")
        || work_unit_src.contains("IndexMap<String, crate::types::prefix::Prefix>");
    assert!(
        uses_indexmap_of_prefix,
        "PrefixesData.prefixes must be `IndexMap<String, Prefix>` (or fully-qualified equivalent) to preserve insertion order; got:\n{work_unit_src}"
    );

    // @step Then list_prefixes::run delegates to these shared modules rather than embedding its own filesystem logic
    let list_src = fs::read_to_string(crate_src.join("commands/list_prefixes.rs"))
        .expect("commands/list_prefixes.rs readable");
    assert!(
        list_src.contains("read_prefixes_or_empty")
            && list_src.contains("read_work_units_or_empty"),
        "commands/list_prefixes.rs must delegate to shared io helpers; got:\n{list_src}"
    );
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "commands/list_prefixes.rs must no longer be a NotYetPorted stub"
    );
}
