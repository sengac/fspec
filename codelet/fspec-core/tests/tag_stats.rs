#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/tag-stats-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `tag-stats`
// (RPC-310). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "tag-stats".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, content: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir parent");
    fs::write(&abs, content).expect("write feature");
}

fn write_tags_json(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn tags_json_with(categories: &[(&str, &[&str])]) -> String {
    let mut cats = Vec::new();
    for (name, tag_names) in categories {
        let tags: Vec<Value> = tag_names
            .iter()
            .map(|n| json!({ "name": n, "description": "desc" }))
            .collect();
        cats.push(json!({
            "name": name,
            "description": "x",
            "required": false,
            "tags": tags,
        }));
    }
    serde_json::to_string_pretty(&json!({ "categories": cats })).unwrap()
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn feature_with_tags(tags: &[&str], name: &str) -> String {
    let tag_line = tags.join(" ");
    format!("{tag_line}\nFeature: {name}\n  Scenario: A\n    Given x\n")
}

// ---------- scenarios ----------

#[test]
fn returns_zero_totals_when_no_spec_directory_exists() {
    // Scenario: Returns zero-totals when no spec directory exists and does not auto-create files

    // @step Given an empty project root directory with no spec subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the tag-stats command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result has totalFiles=0, uniqueTags=0, totalOccurrences=0
    assert_eq!(data["totalFiles"].as_u64(), Some(0));
    assert_eq!(data["uniqueTags"].as_u64(), Some(0));
    assert_eq!(data["totalOccurrences"].as_u64(), Some(0));

    // @step Then the result has empty categories, unusedTags, and invalidFiles arrays
    assert_eq!(data["categories"].as_array().map(Vec::len), Some(0));
    assert_eq!(data["unusedTags"].as_array().map(Vec::len), Some(0));
    assert_eq!(data["invalidFiles"].as_array().map(Vec::len), Some(0));

    // @step Then the result has tagsFileFound=false
    assert_eq!(data["tagsFileFound"].as_bool(), Some(false));

    // @step Then spec/tags.json does not exist after the call
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step Then spec/features/ does not exist after the call
    assert!(!tmp.path().join("spec/features").exists());
}

#[test]
fn groups_all_tags_under_unregistered_when_tags_json_missing() {
    // Scenario: Groups all tags under 'Unregistered' when tags.json is missing

    // @step Given spec/features/a.feature has feature-level tags '@critical @auth'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical", "@auth"], "A"),
    );

    // @step Given spec/features/b.feature has feature-level tags '@critical @ui'
    write_feature(
        tmp.path(),
        "spec/features/b.feature",
        &feature_with_tags(&["@critical", "@ui"], "B"),
    );

    // @step Given spec/tags.json does NOT exist
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the result has uniqueTags=3, totalOccurrences=4, tagsFileFound=false
    assert_eq!(data["uniqueTags"].as_u64(), Some(3));
    assert_eq!(data["totalOccurrences"].as_u64(), Some(4));
    assert_eq!(data["tagsFileFound"].as_bool(), Some(false));

    // @step Then the result has exactly one category named 'Unregistered'
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["name"].as_str(), Some("Unregistered"));

    // @step Then the 'Unregistered' category lists tags sorted by count descending: @critical(2), @auth(1), @ui(1)
    let tags = cats[0]["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(tags[0]["count"].as_u64(), Some(2));
    // @auth and @ui both have count 1; allow either order between them
    let names_lower: Vec<&str> = tags[1..]
        .iter()
        .map(|t| t["tag"].as_str().unwrap())
        .collect();
    assert!(names_lower.contains(&"@auth"));
    assert!(names_lower.contains(&"@ui"));
    assert_eq!(tags[1]["count"].as_u64(), Some(1));
    assert_eq!(tags[2]["count"].as_u64(), Some(1));
}

#[test]
fn projects_tags_into_registered_categories_sorted_descending_by_count() {
    // Scenario: Projects tags into registered categories sorted descending by count

    // @step Given spec/tags.json declares Phase Tags=[@critical, @high] and Component Tags=[@cli, @parser] in that order
    let tmp = TempDir::new().expect("tempdir");
    write_tags_json(
        tmp.path(),
        &tags_json_with(&[
            ("Phase Tags", &["@critical", "@high"]),
            ("Component Tags", &["@cli", "@parser"]),
        ]),
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical @cli'
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical", "@cli"], "A"),
    );
    // @step Given spec/features/b.feature has feature-level tags '@critical @high'
    write_feature(
        tmp.path(),
        "spec/features/b.feature",
        &feature_with_tags(&["@critical", "@high"], "B"),
    );
    // @step Given spec/features/c.feature has feature-level tags '@parser'
    write_feature(
        tmp.path(),
        "spec/features/c.feature",
        &feature_with_tags(&["@parser"], "C"),
    );

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let cats = data["categories"].as_array().expect("categories array");

    // @step Then the categories array contains 'Phase Tags' then 'Component Tags' in that order
    assert_eq!(cats.len(), 2);
    assert_eq!(cats[0]["name"].as_str(), Some("Phase Tags"));
    assert_eq!(cats[1]["name"].as_str(), Some("Component Tags"));

    // @step Then the Phase Tags entry lists @critical with count=2 before @high with count=1
    let phase_tags = cats[0]["tags"].as_array().expect("phase tags array");
    assert_eq!(phase_tags[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(phase_tags[0]["count"].as_u64(), Some(2));
    assert_eq!(phase_tags[1]["tag"].as_str(), Some("@high"));
    assert_eq!(phase_tags[1]["count"].as_u64(), Some(1));

    // @step Then the Component Tags entry lists @cli with count=1 and @parser with count=1
    let comp_tags = cats[1]["tags"].as_array().expect("comp tags array");
    assert_eq!(comp_tags.len(), 2);
    let comp_names: Vec<&str> = comp_tags
        .iter()
        .map(|t| t["tag"].as_str().unwrap())
        .collect();
    assert!(comp_names.contains(&"@cli"));
    assert!(comp_names.contains(&"@parser"));

    // @step Then unusedTags is an empty array
    assert_eq!(data["unusedTags"].as_array().map(Vec::len), Some(0));
}

#[test]
fn lists_registered_but_unused_tags_alphabetically_in_unused_tags() {
    // Scenario: Lists registered-but-unused tags alphabetically in unusedTags

    // @step Given spec/tags.json declares Phase Tags=[@critical, @high, @low]
    let tmp = TempDir::new().expect("tempdir");
    write_tags_json(
        tmp.path(),
        &tags_json_with(&[("Phase Tags", &["@critical", "@high", "@low"])]),
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then unusedTags equals ['@high', '@low'] in that alphabetical order
    let unused = data["unusedTags"].as_array().expect("unusedTags array");
    let unused_names: Vec<&str> = unused.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(unused_names, vec!["@high", "@low"]);

    // @step Then the categories array contains exactly one entry 'Phase Tags' with @critical(1)
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["name"].as_str(), Some("Phase Tags"));
    let tags = cats[0]["tags"].as_array().expect("tags");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(tags[0]["count"].as_u64(), Some(1));
}

#[test]
fn collects_unregistered_tags_into_synthetic_unregistered_category() {
    // Scenario: Collects unregistered tags into a synthetic 'Unregistered' category

    // @step Given spec/tags.json declares Phase Tags=[@critical] only
    let tmp = TempDir::new().expect("tempdir");
    write_tags_json(
        tmp.path(),
        &tags_json_with(&[("Phase Tags", &["@critical"])]),
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical @undeclared'
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical", "@undeclared"], "A"),
    );

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the categories array contains 'Phase Tags' then 'Unregistered' in that order
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 2);
    assert_eq!(cats[0]["name"].as_str(), Some("Phase Tags"));
    assert_eq!(cats[1]["name"].as_str(), Some("Unregistered"));

    // @step Then the 'Unregistered' category contains @undeclared with count=1
    let unreg = cats[1]["tags"].as_array().expect("unreg tags");
    assert_eq!(unreg.len(), 1);
    assert_eq!(unreg[0]["tag"].as_str(), Some("@undeclared"));
    assert_eq!(unreg[0]["count"].as_u64(), Some(1));

    // @step Then unusedTags is an empty array because @critical is used
    assert_eq!(data["unusedTags"].as_array().map(Vec::len), Some(0));
}

#[test]
fn records_files_with_malformed_gherkin_in_invalid_files_without_throwing() {
    // Scenario: Records files with malformed Gherkin in invalidFiles without throwing

    // @step Given spec/features/bad.feature contains the bytes 'This is not gherkin at all'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/bad.feature",
        "This is not gherkin at all\n",
    );

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then invalidFiles equals ['spec/features/bad.feature']
    let invalid = data["invalidFiles"].as_array().expect("invalidFiles array");
    let names: Vec<&str> = invalid.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["spec/features/bad.feature"]);

    // @step Then totalFiles=1 and uniqueTags=0
    assert_eq!(data["totalFiles"].as_u64(), Some(1));
    assert_eq!(data["uniqueTags"].as_u64(), Some(0));
}

#[test]
fn counts_only_feature_level_tags_ignoring_scenario_level_tags() {
    // Scenario: Counts only feature-level tags, ignoring scenario-level tags

    // @step Given spec/features/a.feature has '@critical' on the Feature header and '@smoke' on a scenario
    let tmp = TempDir::new().expect("tempdir");
    let body = "@critical\nFeature: Mixed\n\n  @smoke\n  Scenario: A\n    Given x\n";
    write_feature(tmp.path(), "spec/features/a.feature", body);

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then @critical has count=1 across all categories
    let cats = data["categories"].as_array().expect("categories array");
    let mut critical_count: u64 = 0;
    let mut saw_smoke = false;
    for cat in cats {
        let tags = cat["tags"].as_array().expect("tags array");
        for t in tags {
            let name = t["tag"].as_str().unwrap();
            if name == "@critical" {
                critical_count += t["count"].as_u64().unwrap();
            }
            if name == "@smoke" {
                saw_smoke = true;
            }
        }
    }
    assert_eq!(critical_count, 1, "@critical must appear exactly once");

    // @step Then @smoke does NOT appear in any category and is NOT counted
    assert!(!saw_smoke, "scenario-level tag @smoke must not be counted");
    assert_eq!(data["uniqueTags"].as_u64(), Some(1));
    assert_eq!(data["totalOccurrences"].as_u64(), Some(1));
}

#[test]
fn treats_malformed_tags_json_as_missing_without_escalating() {
    // Scenario: Treats malformed tags.json as missing without escalating

    // @step Given spec/tags.json contains the bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_tags_json(tmp.path(), "{ not json");

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "malformed tags.json must be silent: {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result has tagsFileFound=false
    assert_eq!(data["tagsFileFound"].as_bool(), Some(false));

    // @step Then the categories array contains a single 'Unregistered' entry with @critical(1)
    let cats = data["categories"].as_array().expect("categories array");
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0]["name"].as_str(), Some("Unregistered"));
    let tags = cats[0]["tags"].as_array().expect("tags array");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["tag"].as_str(), Some("@critical"));
    assert_eq!(tags[0]["count"].as_u64(), Some(1));
}

#[test]
fn json_format_emits_two_space_indented_payload_with_canonical_field_order() {
    // Scenario: JSON format emits two-space indented payload with canonical declaration-order fields

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step Given spec/tags.json does NOT exist
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch tag-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data parses as JSON with success=true
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then the top-level keys appear in declaration order: success, totalFiles, uniqueTags, totalOccurrences, categories, unusedTags, tagsFileFound, invalidFiles
    // We verify field order by inspecting the byte positions of each key in the
    // pretty-printed output (serde_json::to_string_pretty preserves struct
    // declaration order, unlike json!{} which alphabetises via BTreeMap).
    let positions: Vec<(usize, &str)> = [
        "\"success\"",
        "\"totalFiles\"",
        "\"uniqueTags\"",
        "\"totalOccurrences\"",
        "\"categories\"",
        "\"unusedTags\"",
        "\"tagsFileFound\"",
        "\"invalidFiles\"",
    ]
    .iter()
    .map(|k| (result.data.find(k).unwrap_or_else(|| panic!("missing key {k} in:\n{}", result.data)), *k))
    .collect();
    for w in positions.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "key {} must appear before {}; got positions {} and {}",
            w[0].1,
            w[1].1,
            w[0].0,
            w[1].0
        );
    }

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"success\"")),
        "expected 2-space indent on \"success\"; got:\n{}",
        result.data
    );
}

#[test]
fn text_format_prints_overall_counters_and_missing_tags_json_warning() {
    // Scenario: Text format prints overall counters and missing-tags.json warning

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step Given spec/tags.json does NOT exist
    assert!(!tmp.path().join("spec/tags.json").exists());

    // @step When I dispatch tag-stats with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");
    let data = &result.data;

    // @step Then the DispatchResult.data contains the line 'Tag Usage Statistics'
    assert!(
        data.lines().any(|l| l == "Tag Usage Statistics"),
        "missing 'Tag Usage Statistics' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the line 'Total feature files: 1'
    assert!(
        data.lines().any(|l| l == "Total feature files: 1"),
        "missing 'Total feature files: 1' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the line 'Unique tags used: 1'
    assert!(
        data.lines().any(|l| l == "Unique tags used: 1"),
        "missing 'Unique tags used: 1' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the line 'Total tag occurrences: 1'
    assert!(
        data.lines().any(|l| l == "Total tag occurrences: 1"),
        "missing 'Total tag occurrences: 1' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the substring '⚠ Warning: spec/tags.json not found'
    assert!(
        data.contains("⚠ Warning: spec/tags.json not found"),
        "missing tags.json warning; got:\n{data}"
    );
}

#[test]
fn text_format_prints_invalid_files_warning_with_bulleted_file_list() {
    // Scenario: Text format prints invalid-files warning with bulleted file list

    // @step Given spec/features/bad.feature contains the bytes 'not gherkin'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/bad.feature", "not gherkin\n");

    // @step When I dispatch tag-stats with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");
    let data = &result.data;

    // @step Then the DispatchResult.data contains the substring '⚠ Warning: 1 file(s) with invalid syntax skipped:'
    assert!(
        data.contains("⚠ Warning: 1 file(s) with invalid syntax skipped:"),
        "missing invalid-files warning; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the exact line '  - spec/features/bad.feature'
    assert!(
        data.lines().any(|l| l == "  - spec/features/bad.feature"),
        "missing invalid-file bullet; got:\n{data}"
    );
}

#[test]
fn text_format_lists_unused_registered_tags_alphabetically() {
    // Scenario: Text format lists unused registered tags alphabetically

    // @step Given spec/tags.json declares Phase Tags=[@critical, @high, @low]
    let tmp = TempDir::new().expect("tempdir");
    write_tags_json(
        tmp.path(),
        &tags_json_with(&[("Phase Tags", &["@critical", "@high", "@low"])]),
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    write_feature(
        tmp.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step When I dispatch tag-stats with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "{result:?}");
    let data = &result.data;

    // @step Then the DispatchResult.data contains the line 'Unused Registered Tags'
    assert!(
        data.lines().any(|l| l == "Unused Registered Tags"),
        "missing 'Unused Registered Tags' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the substring '2 registered tag(s) not used in any feature file:'
    assert!(
        data.contains("2 registered tag(s) not used in any feature file:"),
        "missing unused-count line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the exact line '  @high'
    assert!(
        data.lines().any(|l| l == "  @high"),
        "missing '  @high' line; got:\n{data}"
    );

    // @step Then the DispatchResult.data contains the exact line '  @low'
    assert!(
        data.lines().any(|l| l == "  @low"),
        "missing '  @low' line; got:\n{data}"
    );

    // @step Then in the unused list section '@high' appears before '@low'
    let unused_section_idx = data
        .find("registered tag(s) not used")
        .expect("unused section anchor");
    let tail = &data[unused_section_idx..];
    let high = tail.find("@high").expect("@high in unused section");
    let low = tail.find("@low").expect("@low in unused section");
    assert!(
        high < low,
        "expected @high before @low in unused list; got high={high} low={low}\n{tail}"
    );
}

#[test]
fn shared_infrastructure_modules_exist_under_fspec_core() {
    // Scenario: Shared infrastructure modules exist under codelet/fspec-core for reuse

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the function io::feature_glob::glob_feature_files exists and is reused by tag_stats
    let feature_glob_src =
        fs::read_to_string(crate_src.join("io/feature_glob.rs")).expect("io/feature_glob.rs");
    assert!(
        feature_glob_src.contains("pub fn glob_feature_files"),
        "io/feature_glob.rs must declare pub fn glob_feature_files; got:\n{feature_glob_src}"
    );
    let tag_stats_src =
        fs::read_to_string(crate_src.join("commands/tag_stats.rs")).expect("tag_stats.rs");
    assert!(
        tag_stats_src.contains("glob_feature_files"),
        "tag_stats.rs must reuse glob_feature_files; got:\n{tag_stats_src}"
    );

    // @step Then commands/tag_stats.rs delegates to io::feature_glob and inline tags.json reading
    assert!(
        tag_stats_src.contains("TagsData") || tag_stats_src.contains("tags.json"),
        "tag_stats.rs must reference tags.json reading; got:\n{tag_stats_src}"
    );

    // @step Then commands/tag_stats.rs no longer returns FspecCoreError::NotYetPorted
    assert!(
        !tag_stats_src.contains("NotYetPorted"),
        "commands/tag_stats.rs must no longer be a NotYetPorted stub"
    );
}
