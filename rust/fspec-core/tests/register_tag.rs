#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/register-tag-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `register-tag`
// (RPC-265). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED PHASE: Until the supervisor wires `commands::register_tag::run` into
// the dispatcher (replacing the NotYetPorted stub), these tests will fail
// with `result.success == false` and `error` containing "not yet ported".
// That is the expected red state at the end of Phase B.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "register-tag".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_tags(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn read_tags(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/tags.json"))
        .expect("read tags.json after dispatch");
    serde_json::from_str(&raw).expect("tags.json on disk is valid JSON")
}

fn find_category<'a>(data: &'a Value, name: &str) -> Option<&'a Value> {
    data["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|c| c["name"].as_str() == Some(name))
}

fn category_tag_names(data: &Value, cat: &str) -> Vec<String> {
    find_category(data, cat)
        .expect("category present")
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// ---------- scenarios ----------

#[test]
fn auto_creates_tags_json_and_registers_new_tag_in_existing_category() {
    // Scenario: Auto-creates tags.json and registers a new tag in an existing category

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch the register-tag command with tag '@api', category 'Technical Tags', and description 'API integration features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@api",
            "category": "Technical Tags",
            "description": "API integration features"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success=true; got error={:?}",
        result.error
    );

    // @step And spec/tags.json exists after the call
    assert!(tmp.path().join("spec/tags.json").exists());

    // @step And spec/TAGS.md exists after the call
    assert!(tmp.path().join("spec/TAGS.md").exists());

    // @step And the Technical Tags category on disk contains a tag with name '@api' and description 'API integration features'
    let on_disk = read_tags(tmp.path());
    let tech = find_category(&on_disk, "Technical Tags").expect("Technical Tags present");
    let api = tech["tags"]
        .as_array()
        .expect("tags array")
        .iter()
        .find(|t| t["name"].as_str() == Some("@api"))
        .expect("@api present");
    assert_eq!(
        api["description"].as_str(),
        Some("API integration features")
    );

    // @step And the dispatcher output contains the substring 'Successfully registered @api in Technical Tags'
    assert!(
        result
            .data
            .contains("Successfully registered @api in Technical Tags"),
        "missing canonical success substring; got:\n{}",
        result.data
    );
}

#[test]
fn rejects_duplicate_tag_across_all_categories() {
    // Scenario: Rejects duplicate tag across all categories

    // @step Given spec/tags.json contains a tag '@cli' under Component Tags
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Component Tags", "description": "c", "required": true, "tags": [
      {"name": "@cli", "description": "CLI component"}
    ]},
    {"name": "Technical Tags", "description": "t", "required": false, "tags": []}
  ]
}"#;
    write_tags(tmp.path(), raw);
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch register-tag with tag '@cli', category 'Component Tags', and description 'CLI component'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@cli",
            "category": "Component Tags",
            "description": "CLI component"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Tag @cli is already registered in Component Tags'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Tag @cli is already registered in Component Tags"),
        "missing duplicate-tag substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(
        before, after,
        "tags.json must be unchanged on duplicate-tag failure"
    );
}

#[test]
fn rejects_tag_missing_leading_at_character() {
    // Scenario: Rejects tag missing leading @ character

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch register-tag with tag 'InvalidTag', category 'Technical Tags', and description 'Invalid format'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "InvalidTag",
            "category": "Technical Tags",
            "description": "Invalid format"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Invalid tag format: "InvalidTag". Valid format is @lowercase-with-hyphens'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains(
            r#"Invalid tag format: "InvalidTag". Valid format is @lowercase-with-hyphens"#
        ),
        "missing invalid-tag-format substring; got: {msg}"
    );
}

#[test]
fn normalises_uppercase_tag_to_lowercase_and_reports_conversion() {
    // Scenario: Normalises uppercase tag to lowercase and reports conversion

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch register-tag with tag '@API-Integration', category 'Technical Tags', and description 'API features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@API-Integration",
            "category": "Technical Tags",
            "description": "API features"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success; got error={:?}",
        result.error
    );

    // @step And the Technical Tags category on disk contains a tag with name '@api-integration'
    let on_disk = read_tags(tmp.path());
    let names = category_tag_names(&on_disk, "Technical Tags");
    assert!(
        names.iter().any(|n| n == "@api-integration"),
        "@api-integration must be present in Technical Tags; got: {names:?}"
    );

    // @step And the dispatcher output contains the substring 'Successfully registered @api-integration (converted from @API-Integration) in Technical Tags'
    assert!(
        result.data.contains(
            "Successfully registered @api-integration (converted from @API-Integration) in Technical Tags"
        ),
        "missing canonical converted-success substring; got:\n{}",
        result.data
    );
}

#[test]
fn rejects_tag_containing_characters_outside_allowed_regex() {
    // Scenario: Rejects tag containing characters outside the allowed regex

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch register-tag with tag '@x_underscore', category 'Technical Tags', and description 'desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@x_underscore",
            "category": "Technical Tags",
            "description": "desc"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Invalid tag format: "@x_underscore". Valid format is @lowercase-with-hyphens'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains(
            r#"Invalid tag format: "@x_underscore". Valid format is @lowercase-with-hyphens"#
        ),
        "missing invalid-tag-format substring for underscore tag; got: {msg}"
    );
}

#[test]
fn rejects_unknown_category_with_canonical_available_list() {
    // Scenario: Rejects unknown category with canonical Available categories list

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch register-tag with tag '@custom', category 'NonExistent Category', and description 'desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@custom",
            "category": "NonExistent Category",
            "description": "desc"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Invalid category: "NonExistent Category". Available categories: Phase Tags, Component Tags, Feature Group Tags, Technical Tags, Platform Tags, Priority Tags, Status Tags, Testing Tags, Automation Tags'
    let msg = result.error.as_ref().expect("error msg");
    let expected = r#"Invalid category: "NonExistent Category". Available categories: Phase Tags, Component Tags, Feature Group Tags, Technical Tags, Platform Tags, Priority Tags, Status Tags, Testing Tags, Automation Tags"#;
    assert!(
        msg.contains(expected),
        "missing canonical invalid-category substring; got: {msg}"
    );
}

#[test]
fn matches_category_case_insensitively_but_writes_canonical_name() {
    // Scenario: Matches category name case-insensitively but writes canonical on-disk name in success message

    // @step Given an empty project root directory with no spec/tags.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch register-tag with tag '@custom', category 'technical tags', and description 'desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@custom",
            "category": "technical tags",
            "description": "desc"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success; got error={:?}",
        result.error
    );

    // @step And the dispatcher output contains the substring 'Successfully registered @custom in Technical Tags'
    assert!(
        result
            .data
            .contains("Successfully registered @custom in Technical Tags"),
        "missing canonical case-insensitive success substring; got:\n{}",
        result.data
    );

    // @step And the Technical Tags category on disk contains a tag with name '@custom'
    let on_disk = read_tags(tmp.path());
    let names = category_tag_names(&on_disk, "Technical Tags");
    assert!(
        names.iter().any(|n| n == "@custom"),
        "@custom must be present in Technical Tags; got: {names:?}"
    );
}

#[test]
fn sorts_tags_alphabetically_within_matched_category_after_insert() {
    // Scenario: Sorts tags alphabetically within the matched category after insert

    // @step Given spec/tags.json contains a Phase Tags category with tags '@zed', '@aaa', and '@mid' in that insertion order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@zed", "description": "z"},
      {"name": "@aaa", "description": "a"},
      {"name": "@mid", "description": "m"}
    ]},
    {"name": "Component Tags", "description": "c", "required": true, "tags": []}
  ]
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch register-tag with tag '@bcd', category 'Phase Tags', and description 'b desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@bcd",
            "category": "Phase Tags",
            "description": "b desc"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success; got error={:?}",
        result.error
    );

    // @step And the Phase Tags category on disk contains tags in the order '@aaa', '@bcd', '@mid', '@zed'
    let on_disk = read_tags(tmp.path());
    let names = category_tag_names(&on_disk, "Phase Tags");
    assert_eq!(
        names,
        vec!["@aaa", "@bcd", "@mid", "@zed"],
        "Phase Tags must be alphabetised after insert; got: {names:?}"
    );
}

#[test]
fn preserves_auxiliary_top_level_fields_and_bumps_statistics_last_updated() {
    // Scenario: Preserves auxiliary top-level fields and bumps statistics.lastUpdated

    // @step Given spec/tags.json contains populated auxiliary fields combinationExamples, usageGuidelines, and references plus an initial statistics.lastUpdated timestamp
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "categories": [
    {"name": "Technical Tags", "description": "t", "required": false, "tags": []}
  ],
  "combinationExamples": [
    {"title": "demo", "tags": "@a @b", "interpretation": ["a", "b"]}
  ],
  "usageGuidelines": {
    "requiredCombinations": {"title": "req", "requirements": ["one"], "minimumExample": "@x"},
    "recommendedCombinations": {"title": "rec", "includes": ["two"], "recommendedExample": "@y"},
    "orderingConvention": {"title": "ord", "order": ["three"], "example": "@z"}
  },
  "references": [
    {"title": "spec", "url": "https://example.com"}
  ],
  "statistics": {
    "lastUpdated": "1999-01-01T00:00:00.000Z",
    "phaseStats": [],
    "componentStats": [],
    "featureGroupStats": [],
    "updateCommand": "fspec tag-stats"
  }
}"#;
    write_tags(tmp.path(), raw);

    // @step When I dispatch register-tag with tag '@new', category 'Technical Tags', and description 'new desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@new",
            "category": "Technical Tags",
            "description": "new desc"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected success; got error={:?}",
        result.error
    );

    let on_disk = read_tags(tmp.path());

    // @step And spec/tags.json on disk still contains combinationExamples, usageGuidelines, and references with their original payloads
    assert!(on_disk.get("combinationExamples").is_some());
    let ce = &on_disk["combinationExamples"];
    assert_eq!(ce[0]["title"].as_str(), Some("demo"));
    assert!(on_disk.get("usageGuidelines").is_some());
    assert_eq!(
        on_disk["usageGuidelines"]["requiredCombinations"]["title"].as_str(),
        Some("req")
    );
    assert!(on_disk.get("references").is_some());
    assert_eq!(on_disk["references"][0]["title"].as_str(), Some("spec"));

    // @step And spec/tags.json statistics.lastUpdated on disk differs from the original initial timestamp
    let last_updated = on_disk["statistics"]["lastUpdated"]
        .as_str()
        .expect("statistics.lastUpdated string");
    assert_ne!(
        last_updated, "1999-01-01T00:00:00.000Z",
        "statistics.lastUpdated MUST be bumped; got: {last_updated}"
    );
}

#[test]
fn escalates_malformed_tags_json_as_structured_parse_error() {
    // Scenario: Escalates malformed tags.json as a structured parse error

    // @step Given spec/tags.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_tags(tmp.path(), "{ not valid json");
    let before = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();

    // @step When I dispatch register-tag against that project root
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "tag": "@api",
            "category": "Technical Tags",
            "description": "API features"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false; got data={}",
        result.data
    );

    // @step And the error message contains the substring 'Failed to parse tags.json'
    let msg = result.error.as_ref().expect("error msg");
    assert!(
        msg.contains("Failed to parse tags.json"),
        "missing canonical parse-error substring; got: {msg}"
    );

    // @step And spec/tags.json content on disk is unchanged from before the call
    let after = fs::read_to_string(tmp.path().join("spec/tags.json")).unwrap();
    assert_eq!(
        before, after,
        "tags.json must be untouched on parse failure"
    );
}
