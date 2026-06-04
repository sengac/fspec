// Feature: spec/features/list-features-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-features`
// (RPC-245). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Red phase: list_features::run is still a NotYetPorted stub, so every
// dispatcher call below returns success=false with the NotYetPorted error
// message. The success-path assertions therefore fail until the impl lands
// in Phase C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-features".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, content: &str) {
    let path = project_root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&path, content).expect("write feature file");
}

fn mk_features_dir(project_root: &Path) {
    let features = project_root.join("spec/features");
    fs::create_dir_all(&features).expect("mkdir spec/features");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// A minimal valid feature body with N scenarios, tagged with the provided
/// `@`-prefixed tags (one tag per line above `Feature:`).
fn feature_body(name: &str, tags: &[&str], scenarios: usize) -> String {
    let mut s = String::new();
    for t in tags {
        s.push_str(t);
        s.push('\n');
    }
    s.push_str(&format!("Feature: {name}\n"));
    for i in 0..scenarios {
        s.push_str(&format!(
            "\n  Scenario: scenario {i}\n    Given a precondition\n    When something happens\n    Then expect outcome\n",
        ));
    }
    s
}

// ---------- scenarios ----------

#[test]
fn escalates_structured_error_when_spec_features_does_not_exist() {
    // Scenario: Escalates a structured error when spec/features/ does not exist

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the list-features command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Directory not found: spec/features/'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Directory not found: spec/features/"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn returns_empty_features_list_when_spec_features_is_empty() {
    // Scenario: Returns an empty features list when spec/features/ exists but contains no .feature files

    // @step Given a project root containing an empty spec/features/ directory
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());

    // @step When I dispatch the list-features command with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true with an empty features array
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["features"].as_array().map(Vec::len),
        Some(0),
        "expected empty features array, got {}",
        result.data
    );
}

#[test]
fn aggregates_feature_names_scenario_counts_and_tags_sorted_by_file() {
    // Scenario: Aggregates feature names, scenario counts, and tags sorted by file path

    // @step Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 3 scenarios
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        &feature_body("User Authentication", &["@critical", "@auth"], 3),
    );

    // @step Given spec/features/billing.feature exists with name 'Billing', tags '@billing' and 1 scenario
    write_feature(
        tmp.path(),
        "spec/features/billing.feature",
        &feature_body("Billing", &["@billing"], 1),
    );

    // @step When I dispatch list-features with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["features"].as_array().expect("features array");

    // @step Then the features array contains exactly two entries in order spec/features/auth.feature then spec/features/billing.feature
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");
    assert_eq!(arr[0]["file"].as_str(), Some("spec/features/auth.feature"));
    assert_eq!(arr[1]["file"].as_str(), Some("spec/features/billing.feature"));

    // @step Then the auth entry has scenarioCount=3 and tags exactly ['@critical', '@auth']
    assert_eq!(arr[0]["scenarioCount"].as_u64(), Some(3));
    assert_eq!(
        arr[0]["tags"].as_array().map(|a| {
            a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
        }),
        Some(vec!["@critical", "@auth"])
    );

    // @step Then the billing entry has scenarioCount=1 and tags exactly ['@billing']
    assert_eq!(arr[1]["scenarioCount"].as_u64(), Some(1));
    assert_eq!(
        arr[1]["tags"].as_array().map(|a| {
            a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
        }),
        Some(vec!["@billing"])
    );
}

#[test]
fn filters_features_by_exact_tag_match_with_leading_at() {
    // Scenario: Filters features by exact tag match including the leading '@'

    // @step Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        &feature_body("Auth", &["@critical"], 1),
    );

    // @step Given spec/features/billing.feature exists with tag '@billing' and 1 scenario
    write_feature(
        tmp.path(),
        "spec/features/billing.feature",
        &feature_body("Billing", &["@billing"], 1),
    );

    // @step When I dispatch list-features with format='json' and tag='@critical'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "tag": "@critical" }),
    ));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["features"].as_array().expect("features array");

    // @step Then the features array contains exactly one entry whose file is spec/features/auth.feature
    assert_eq!(arr.len(), 1, "expected 1 entry, got {arr:?}");
    assert_eq!(arr[0]["file"].as_str(), Some("spec/features/auth.feature"));
}

#[test]
fn silently_skips_files_that_fail_to_parse_without_escalating() {
    // Scenario: Silently skips files that fail to parse without escalating

    // @step Given spec/features/valid-feature.feature contains a parseable feature with 2 scenarios
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/valid-feature.feature",
        &feature_body("Valid Feature", &[], 2),
    );

    // @step Given spec/features/broken.feature contains the malformed bytes 'not a feature file'
    write_feature(
        tmp.path(),
        "spec/features/broken.feature",
        "not a feature file",
    );

    // @step When I dispatch list-features with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "broken.feature must be silently skipped: {result:?}"
    );

    // @step Then the features array contains exactly one entry whose file is spec/features/valid-feature.feature
    let data = parse_data(&result.data);
    let arr = data["features"].as_array().expect("features array");
    assert_eq!(arr.len(), 1, "expected 1 entry, got {arr:?}");
    assert_eq!(
        arr[0]["file"].as_str(),
        Some("spec/features/valid-feature.feature")
    );
}

#[test]
fn sorts_features_alphabetically_by_file_path_regardless_of_glob_order() {
    // Scenario: Sorts features alphabetically by file path regardless of glob order

    // @step Given spec/features/zebra.feature, spec/features/alpha.feature and spec/features/mango.feature each contain one scenario
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    // Write in non-alphabetical order so the impl can't accidentally rely on
    // creation order.
    write_feature(
        tmp.path(),
        "spec/features/zebra.feature",
        &feature_body("Zebra", &[], 1),
    );
    write_feature(
        tmp.path(),
        "spec/features/alpha.feature",
        &feature_body("Alpha", &[], 1),
    );
    write_feature(
        tmp.path(),
        "spec/features/mango.feature",
        &feature_body("Mango", &[], 1),
    );

    // @step When I dispatch list-features with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let arr = data["features"].as_array().expect("features array");

    // @step Then the features array file values are in order spec/features/alpha.feature, spec/features/mango.feature, spec/features/zebra.feature
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["file"].as_str(), Some("spec/features/alpha.feature"));
    assert_eq!(arr[1]["file"].as_str(), Some("spec/features/mango.feature"));
    assert_eq!(arr[2]["file"].as_str(), Some("spec/features/zebra.feature"));
}

#[test]
fn text_format_prints_sentinel_for_empty_results() {
    // Scenario: Text format prints sentinel for empty results

    // @step Given a project root containing an empty spec/features/ directory
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());

    // @step When I dispatch list-features with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data is exactly the string 'No feature files found in spec/features/'
    assert_eq!(
        result.data, "No feature files found in spec/features/",
        "expected exact sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn text_format_renders_populated_listing_with_header_line_and_summary() {
    // Scenario: Text format renders a populated listing with header line and unfiltered summary

    // @step Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 2 scenarios
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        &feature_body("User Authentication", &["@critical", "@auth"], 2),
    );

    // @step When I dispatch list-features with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line '  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]"),
        "missing exact feature listing line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line 'Found 1 feature files'
    assert!(
        result.data.lines().any(|l| l == "Found 1 feature files"),
        "missing exact 'Found 1 feature files' summary; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_with_tag_filter_uses_matching_summary_phrasing() {
    // Scenario: Text format with a tag filter uses the matching summary phrasing

    // @step Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        &feature_body("Auth", &["@critical"], 1),
    );

    // @step When I dispatch list-features with format='text' and tag='@critical'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "text", "tag": "@critical" }),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line 'Found 1 feature files matching @critical'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Found 1 feature files matching @critical"),
        "missing exact matching-summary line; got:\n{}",
        result.data
    );
}

#[test]
fn json_format_emits_two_space_indented_payload_with_canonical_field_set() {
    // Scenario: JSON format emits a 2-space indented payload with the canonical field set

    // @step Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical' and 2 scenarios
    let tmp = TempDir::new().expect("tempdir");
    mk_features_dir(tmp.path());
    write_feature(
        tmp.path(),
        "spec/features/auth.feature",
        &feature_body("User Authentication", &["@critical"], 2),
    );

    // @step When I dispatch list-features with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has a 'features' array of length 1
    let data = parse_data(&result.data);
    let arr = data["features"].as_array().expect("features array");
    assert_eq!(arr.len(), 1);

    // @step Then the first features entry contains fields file='spec/features/auth.feature', name='User Authentication', scenarioCount=2, tags=['@critical']
    let entry = &arr[0];
    assert_eq!(entry["file"].as_str(), Some("spec/features/auth.feature"));
    assert_eq!(entry["name"].as_str(), Some("User Authentication"));
    assert_eq!(entry["scenarioCount"].as_u64(), Some(2));
    assert_eq!(
        entry["tags"].as_array().map(|a| {
            a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
        }),
        Some(vec!["@critical"])
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    // serde_json::to_string_pretty produces 2-space indent by default. We
    // verify the indentation pattern by walking the nested structure:
    //   level 1: `  "features"` (2 spaces — root field)
    //   level 2: `    {`        (4 spaces — array entry open brace)
    //   level 3: `      "file"` (6 spaces — nested field)
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"features\"")),
        "expected a line starting with two-space indent + \"features\"; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected a four-space-indented `{{` line opening the features entry; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l.starts_with("      \"file\"")),
        "expected a line starting with six-space indent + \"file\"; got:\n{}",
        result.data
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under fspec-core for reuse by other gherkin-aware commands

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the module io::feature_glob::glob_feature_files exists and is publicly accessible from the crate root
    let feature_glob_path: PathBuf = crate_src.join("io/feature_glob.rs");
    assert!(
        feature_glob_path.exists(),
        "io/feature_glob.rs must exist; missing: {}",
        feature_glob_path.display()
    );
    let feature_glob_src = fs::read_to_string(&feature_glob_path)
        .expect("io/feature_glob.rs readable");
    assert!(
        feature_glob_src.contains("pub fn glob_feature_files"),
        "io/feature_glob.rs must declare `pub fn glob_feature_files`; got:\n{feature_glob_src}"
    );

    // @step Then the error::FspecCoreError enum declares a DirectoryNotFound variant whose Display contains the substring 'Directory not found'
    let error_src = fs::read_to_string(crate_src.join("error.rs"))
        .expect("error.rs readable");
    assert!(
        error_src.contains("DirectoryNotFound"),
        "error.rs must declare a DirectoryNotFound variant; got:\n{error_src}"
    );
    assert!(
        error_src.contains("Directory not found"),
        "error.rs DirectoryNotFound Display must contain 'Directory not found'; got:\n{error_src}"
    );

    // @step Then list_features::run delegates to these shared modules rather than embedding its own filesystem-walk logic
    let list_src = fs::read_to_string(crate_src.join("commands/list_features.rs"))
        .expect("commands/list_features.rs readable");
    assert!(
        list_src.contains("glob_feature_files"),
        "commands/list_features.rs must delegate to io::feature_glob::glob_feature_files; got:\n{list_src}"
    );
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "commands/list_features.rs must no longer be a NotYetPorted stub"
    );
}
