// Feature: spec/features/list-tags-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-tags`
// (RPC-251). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: these tests MUST fail today because
// `commands::list_tags::run` still returns
// `FspecCoreError::NotYetPorted`. Phase C will replace the stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-tags".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_tags_file(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags file");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Two-category fixture: Phase Tags (with @critical) then Component Tags
/// (with @cli). Insertion order on the wire is preserved by writing the
/// raw JSON text by hand.
fn two_categories_json() -> String {
    r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "Phase identification",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    },
    {
      "name": "Component Tags",
      "description": "Architectural component",
      "required": true,
      "tags": [
        { "name": "@cli", "description": "CLI surface" }
      ]
    }
  ]
}"#
    .to_string()
}

// ---------- scenarios ----------

#[test]
fn scenario_auto_creates_tags_json_with_canonical_nine_category_default() {
    // Scenario: Auto-creates spec/tags.json with the canonical nine-category default when missing

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").join("tags.json").exists());

    // @step When I dispatch the list-tags command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/tags.json exists after the call
    assert!(
        tmp.path().join("spec").join("tags.json").exists(),
        "list-tags must auto-create the spec tags file"
    );

    // @step And the dispatcher result's categories array has length 9 in the order Phase Tags, Component Tags, Feature Group Tags, Technical Tags, Platform Tags, Priority Tags, Status Tags, Testing Tags, Automation Tags
    let data = parse_data(&result.data);
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 9, "expected 9 default categories, got {cats:?}");
    let expected = [
        "Phase Tags",
        "Component Tags",
        "Feature Group Tags",
        "Technical Tags",
        "Platform Tags",
        "Priority Tags",
        "Status Tags",
        "Testing Tags",
        "Automation Tags",
    ];
    for (i, name) in expected.iter().enumerate() {
        assert_eq!(
            cats[i]["name"].as_str(),
            Some(*name),
            "category[{i}] mismatch; got {:?}",
            cats[i]
        );
    }
}

#[test]
fn scenario_preserves_insertion_order_of_categories_on_disk() {
    // Scenario: Preserves insertion order of categories on disk (not alphabetical)

    // @step Given spec/tags.json contains exactly two categories in the order Automation Tags then Phase Tags
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    { "name": "Automation Tags", "description": "x", "required": false, "tags": [] },
    { "name": "Phase Tags", "description": "x", "required": true, "tags": [] }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the categories array contains exactly two entries in order Automation Tags then Phase Tags
    let data = parse_data(&result.data);
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 2, "expected 2 entries; got {cats:?}");
    assert_eq!(cats[0]["name"].as_str(), Some("Automation Tags"));
    assert_eq!(cats[1]["name"].as_str(), Some("Phase Tags"));
}

#[test]
fn scenario_sorts_tags_within_each_category_alphabetically() {
    // Scenario: Sorts tags within each category alphabetically by tag name

    // @step Given spec/tags.json contains a Phase Tags category with tags '@zed' (description 'Z desc') and '@aaa' (description 'A desc') in that insertion order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@zed", "description": "Z desc" },
        { "name": "@aaa", "description": "A desc" }
      ]
    }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let phase_tags = &data["categories"].as_array().expect("cats")[0]["tags"]
        .as_array()
        .expect("phase tags array");

    // @step Then the Phase Tags entry's tags array contains exactly two entries
    assert_eq!(phase_tags.len(), 2, "got {phase_tags:?}");

    // @step Then the first tag entry has tag='@aaa' and description='A desc'
    assert_eq!(phase_tags[0]["tag"].as_str(), Some("@aaa"));
    assert_eq!(phase_tags[0]["description"].as_str(), Some("A desc"));

    // @step Then the second tag entry has tag='@zed' and description='Z desc'
    assert_eq!(phase_tags[1]["tag"].as_str(), Some("@zed"));
    assert_eq!(phase_tags[1]["description"].as_str(), Some("Z desc"));
    // @step And the first tag entry has tag='@aaa' and description='A desc'
    // @step And the second tag entry has tag='@zed' and description='Z desc'
}

#[test]
fn scenario_projects_only_tag_and_description_fields() {
    // Scenario: Projects only tag and description fields, ignoring auxiliary Tag-interface fields

    // @step Given spec/tags.json contains a Phase Tags category with a single tag whose name is '@critical', description is 'Critical features', and which also carries auxiliary fields 'usage', 'scope', and 'examples'
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        {
          "name": "@critical",
          "description": "Critical features",
          "usage": "use sparingly",
          "scope": "high-priority work",
          "examples": "auth, security"
        }
      ]
    }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the first Phase Tags entry has tag='@critical' and description='Critical features'
    let data = parse_data(&result.data);
    let entry = &data["categories"].as_array().expect("cats")[0]["tags"]
        .as_array()
        .expect("tags")[0];
    assert_eq!(entry["tag"].as_str(), Some("@critical"));
    assert_eq!(entry["description"].as_str(), Some("Critical features"));

    // @step Then the first Phase Tags entry does NOT contain the field 'usage'
    assert!(
        entry.get("usage").is_none(),
        "must NOT project 'usage' field; got {entry:?}"
    );

    // @step Then the first Phase Tags entry does NOT contain the field 'scope'
    assert!(
        entry.get("scope").is_none(),
        "must NOT project 'scope' field; got {entry:?}"
    );

    // @step Then the first Phase Tags entry does NOT contain the field 'examples'
    assert!(
        entry.get("examples").is_none(),
        "must NOT project 'examples' field; got {entry:?}"
    );
    // @step And the first Phase Tags entry has tag='@critical' and description='Critical features'
    // @step And the first Phase Tags entry does NOT contain the field 'usage'
    // @step And the first Phase Tags entry does NOT contain the field 'scope'
    // @step And the first Phase Tags entry does NOT contain the field 'examples'
}

#[test]
fn scenario_restricts_output_to_matching_category() {
    // Scenario: Restricts output to the matching category when --category is supplied

    // @step Given spec/tags.json contains Phase Tags (with '@critical' description 'Critical features') and Component Tags (with '@cli' description 'CLI surface')
    let tmp = TempDir::new().expect("tempdir");
    write_tags_file(tmp.path(), &two_categories_json());

    // @step When I dispatch list-tags with category='Phase Tags' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "category": "Phase Tags", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the categories array contains exactly one entry whose name is 'Phase Tags'
    let data = parse_data(&result.data);
    let cats = data["categories"].as_array().expect("categories");
    assert_eq!(cats.len(), 1, "expected 1 entry; got {cats:?}");
    assert_eq!(cats[0]["name"].as_str(), Some("Phase Tags"));

    // @step And the response data does NOT contain the substring 'Component Tags'
    assert!(
        !result.data.contains("Component Tags"),
        "category filter must drop Component Tags entirely; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_returns_structured_error_when_category_unknown() {
    // Scenario: Returns structured error when --category does not match any category exactly

    // @step Given spec/tags.json contains Phase Tags and Component Tags categories
    let tmp = TempDir::new().expect("tempdir");
    write_tags_file(tmp.path(), &two_categories_json());

    // @step When I dispatch list-tags with category='No Such Category'
    let result = dispatch_command(req(tmp.path(), json!({ "category": "No Such Category" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the substring 'Category not found: No Such Category. Available categories: Phase Tags, Component Tags'
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains(
            "Category not found: No Such Category. Available categories: Phase Tags, Component Tags"
        ),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_escalates_malformed_tags_json_as_parse_error() {
    // Scenario: Escalates malformed tags.json as a structured parse error

    // @step Given spec/tags.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_tags_file(tmp.path(), "{ not valid json");

    // @step When I dispatch list-tags against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse tags.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse tags.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_text_format_renders_header_and_per_tag_lines() {
    // Scenario: Text format renders header line and per-tag lines per category

    // @step Given spec/tags.json contains Phase Tags (with '@critical' description 'Critical features') and Component Tags (empty)
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    },
    {
      "name": "Component Tags",
      "description": "x",
      "required": true,
      "tags": []
    }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the substring 'Phase Tags (1 tags)'
    assert!(
        result.data.contains("Phase Tags (1 tags)"),
        "missing 'Phase Tags (1 tags)'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  @critical - Critical features'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  @critical - Critical features"),
        "missing exact line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring 'Component Tags (0 tags)'
    assert!(
        result.data.contains("Component Tags (0 tags)"),
        "missing 'Component Tags (0 tags)'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '  No tags registered'
    assert!(
        result.data.lines().any(|l| l == "  No tags registered"),
        "missing '  No tags registered'; got:\n{}",
        result.data
    );
    // @step And the DispatchResult.data contains the exact line ' @critical - Critical features'
    // @step And the DispatchResult.data contains the substring 'Component Tags (0 tags)'
    // @step And the DispatchResult.data contains the exact line ' No tags registered'
}

#[test]
fn scenario_text_format_emits_trailing_blank_line() {
    // Scenario: Text format emits a trailing blank line after the last category

    // @step Given spec/tags.json contains a single Phase Tags category with '@critical' (description 'Critical features')
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data ends with a trailing newline character
    assert!(
        result.data.ends_with('\n'),
        "text output must end with a newline; got:\n{:?}",
        result.data
    );

    // @step Then the last non-empty line of the DispatchResult.data is '  @critical - Critical features'
    let last_non_empty = result
        .data
        .lines()
        .rfind(|l| !l.is_empty())
        .expect("at least one non-empty line");
    assert_eq!(
        last_non_empty, "  @critical - Critical features",
        "unexpected last non-empty line; full output:\n{}",
        result.data
    );
    // @step And the last non-empty line of the DispatchResult.data is ' @critical - Critical features'
}

#[test]
fn scenario_json_format_emits_two_space_indent() {
    // Scenario: JSON format emits two-space indented payload with categories array

    // @step Given spec/tags.json contains a single Phase Tags category with '@critical' (description 'Critical features')
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": true,
      "tags": [
        { "name": "@critical", "description": "Critical features" }
      ]
    }
  ]
}"#;
    write_tags_file(tmp.path(), raw);

    // @step When I dispatch list-tags with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has a 'categories' array of length 1
    let data = parse_data(&result.data);
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 1);

    // @step Then the first categories entry has name='Phase Tags' and a tags array of length 1
    let cat = &cats[0];
    assert_eq!(cat["name"].as_str(), Some("Phase Tags"));
    let tags = cat["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);

    // @step Then the first tag entry has tag='@critical' and description='Critical features'
    assert_eq!(tags[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(tags[0]["description"].as_str(), Some("Critical features"));

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result
            .data
            .lines()
            .any(|l| l.starts_with("  \"categories\"")),
        "expected line starting with two-space indent + \"categories\"; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected four-space-indented `{{` line opening a category entry; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l.starts_with("      \"name\"")),
        "expected six-space-indented \"name\" field; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under rust/fspec-core for reuse by other tag commands

    // @step Given the rust/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect rust/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the module io::ensure::ensure_tags_file exists and is publicly accessible from the crate root
    let ensure_src =
        fs::read_to_string(crate_src.join("io/ensure.rs")).expect("io/ensure.rs readable");
    assert!(
        ensure_src.contains("pub fn ensure_tags_file"),
        "io/ensure.rs must declare `pub fn ensure_tags_file`; got:\n{ensure_src}"
    );

    // @step And types::tags::TagsData exists as a public type
    let tags_path = crate_src.join("types/tags.rs");
    assert!(
        tags_path.exists(),
        "types/tags.rs must exist; got missing: {}",
        tags_path.display()
    );
    let tags_src = fs::read_to_string(&tags_path).expect("types/tags.rs readable");
    assert!(
        tags_src.contains("pub struct TagsData"),
        "types/tags.rs must declare `pub struct TagsData`; got:\n{tags_src}"
    );

    // @step And commands/list_tags.rs no longer declares the NotYetPorted stub
    let list_src = fs::read_to_string(crate_src.join("commands/list_tags.rs"))
        .expect("commands/list_tags.rs readable");
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "commands/list_tags.rs must no longer be a NotYetPorted stub"
    );
}
// @step And the first categories entry has name='Phase Tags' and a tags array of length 1
// @step And the first tag entry has tag='@critical' and description='Critical features'
// @step And the DispatchResult.data uses 2-space indentation
