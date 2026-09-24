//! Feature: spec/features/coverage-mapping-remediation.feature
//!
//! AUDIT-002 — validates the remediation acceptance criteria by driving the
//! REAL `link-coverage` / `unlink-coverage` / `audit-coverage` command
//! entry points against a fixture project:
//!   (1) re-linking a divergent (whole-file blob) impl mapping to a tight
//!       per-function range passes the audit and leaves no mapping exceeding
//!       a single-function line range;
//!   (2) migrating a dead-path mapping (deleted test + impl files) to a
//!       live Rust port leaves zero file-missing / lines-past-EOF defects.
//!
//! Fixture layout (fresh TempDir per test):
//!   src/thing.rs                       — impl file: fn alpha (lines 1-3), fn beta (lines 4-6)
//!   tests/thing.rs                     — test file whose @step comments match the fixture
//!                                        feature's steps (work-unit type defaults to "story"
//!                                        → link-coverage step validation is enforced)
//!   spec/features/thing.feature        — @AUDIT-002 feature, scenario "Thing does the thing"
//!   spec/features/thing.feature.coverage — sidecar seeded with the pre-remediation mapping

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchResult, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

/// Fixture impl file: two 3-line functions — `alpha` (lines 1-3) and
/// `beta` (lines 4-6). A "whole-file blob" mapping therefore spans 1-6;
/// a single-function range spans at most 3 lines.
const IMPL_SOURCE: &str = "\
fn alpha() {
    let a = 1;
}
fn beta() {
    let b = 2;
}
";

/// Fixture test file. The @step comments match the fixture feature's
/// scenario steps verbatim, so `link-coverage`'s mandatory step validation
/// (story work-unit type — strictest) passes.
const FIXTURE_TEST: &str = "\
// @step Given a thing exists
// @step When the thing is used
// @step Then the thing does the thing
fn t() {
    let thing = 1;
}
";

/// Fixture feature file — models a feature from the AUDIT-002 fix list.
const FIXTURE_FEATURE: &str = "\
@AUDIT-002
Feature: Thing

  Scenario: Thing does the thing
    Given a thing exists
    When the thing is used
    Then the thing does the thing
";

const SCENARIO: &str = "Thing does the thing";

fn req(project_root: &Path, command: &str, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Seed the fixture project (impl + test + feature files) with a fresh
/// coverage sidecar carrying the given pre-remediation mapping.
fn seed_fixture(root: &Path, test_file: &str, test_lines: &str, impl_file: &str, impl_lines: Vec<i64>) {
    write_file(root, "src/thing.rs", IMPL_SOURCE);
    write_file(root, "tests/thing.rs", FIXTURE_TEST);
    write_file(root, "spec/features/thing.feature", FIXTURE_FEATURE);
    let sidecar = json!({
        "scenarios": [{
            "name": SCENARIO,
            "testMappings": [{
                "file": test_file,
                "lines": test_lines,
                "implMappings": [{ "file": impl_file, "lines": impl_lines }]
            }]
        }],
        "stats": {
            "totalScenarios": 1,
            "coveredScenarios": 1,
            "coveragePercent": 100,
            "testFiles": [test_file],
            "implFiles": [impl_file],
            "totalLinesCovered": 0
        }
    });
    write_file(
        root,
        "spec/features/thing.feature.coverage",
        &serde_json::to_string(&sidecar).expect("sidecar is JSON"),
    );
}

fn read_sidecar(root: &Path, feature: &str) -> Value {
    let raw = fs::read_to_string(root.join(format!("spec/features/{feature}.feature.coverage")))
        .expect("read sidecar");
    serde_json::from_str(&raw).expect("sidecar is JSON")
}

/// The single impl mapping of the fixture sidecar's only test mapping.
fn impl_mapping(sidecar: &Value) -> &Value {
    &sidecar["scenarios"][0]["testMappings"][0]["implMappings"][0]
}

/// Inclusive line span of that impl mapping.
fn mapping_span(sidecar: &Value) -> i64 {
    let lines: Vec<i64> = impl_mapping(sidecar)["lines"]
        .as_array()
        .expect("impl lines array")
        .iter()
        .map(|l| l.as_i64().expect("line number"))
        .collect();
    let min = lines.iter().min().expect("at least one line");
    let max = lines.iter().max().expect("at least one line");
    max - min + 1
}

/// Parse the audit-coverage `{output, exitCode}` envelope into
/// (structurally clean?, report text).
fn audit(result: &DispatchResult) -> (bool, String) {
    assert!(
        result.success,
        "audit-coverage must run, got {:?}",
        result.error
    );
    let envelope: Value = serde_json::from_str(&result.data).expect("audit envelope is JSON");
    let clean = envelope["exitCode"].as_i64() == Some(0);
    (clean, envelope["output"].as_str().expect("report text").to_string())
}

/// Walk every test/impl mapping in the sidecar; `check` runs per file ref.
fn for_each_mapped_file<'a>(sidecar: &'a Value, mut check: impl FnMut(&'a str)) {
    for scenario in sidecar["scenarios"].as_array().expect("scenarios array") {
        for mapping in scenario["testMappings"].as_array().expect("test mappings") {
            check(mapping["file"].as_str().expect("test file"));
            for impl_mapping in mapping["implMappings"].as_array().expect("impl mappings") {
                check(impl_mapping["file"].as_str().expect("impl file"));
            }
        }
    }
}

#[test]
fn maintainer_relinks_divergent_scenarios_to_their_implementing_functions() {
    // @step Given a feature in the audit fix list whose scenarios were judged divergent or unverifiable
    // @step And the flagged impl mappings are whole-file blobs (all scenarios mapped to the same large line range)
    let tmp = TempDir::new().expect("tempdir");
    // Pre-remediation state: the scenario's impl mapping is the whole file
    // (lines 1-6 — both `alpha` and `beta`), i.e. the flagged blob.
    seed_fixture(
        tmp.path(),
        "tests/thing.rs",
        "1-6",
        "src/thing.rs",
        (1..=6).collect(),
    );
    let sidecar = read_sidecar(tmp.path(), "thing");
    assert_eq!(
        mapping_span(&sidecar),
        6,
        "pre-remediation mapping must be a whole-file blob (6 of 6 lines)"
    );

    // @step When the maintainer re-links each flagged scenario to the specific function(s) that implement its behavior
    // impl-only mode: the test mapping (tests/thing.rs) already exists, so
    // link-coverage UPDATES the existing src/thing.rs impl mapping in place
    // from the blob to the tight range of `alpha` (the function that
    // implements the scenario) — no second mapping is created.
    let relinked = dispatch_command(
        req(
            tmp.path(),
            "link-coverage",
            json!({
                "featureName": "thing",
                "scenario": SCENARIO,
                "testFile": "tests/thing.rs",
                "implFile": "src/thing.rs",
                "implLines": "1-3"
            }),
        ),
    );
    assert!(
        relinked.success,
        "impl-only re-link to the implementing function must succeed, got {:?}",
        relinked.error
    );
    assert!(
        relinked.data.contains("Updated implementation mapping"),
        "re-link must update the existing mapping in place, got {}",
        relinked.data
    );

    // @step And the maintainer re-runs the semantic audit for that feature
    let (clean, report) = audit(&dispatch_command(
        req(
            tmp.path(),
            "audit-coverage",
            json!({ "featureName": "thing" }),
        ),
    ));

    // @step Then each re-linked scenario is judged aligned on re-audit
    // (clean structural audit + a tight single-function range = aligned)
    assert!(
        clean,
        "audit must be clean after a tight re-link, got: {report}"
    );
    assert!(
        report.contains("All mappings valid"),
        "audit report must confirm all mappings valid, got: {report}"
    );

    // @step And no impl mapping for the feature exceeds a single-function line range
    let sidecar = read_sidecar(tmp.path(), "thing");
    assert_eq!(
        sidecar["scenarios"][0]["testMappings"].as_array().expect("test mappings").len(),
        1,
        "re-link must not create a second test mapping"
    );
    let mapping = impl_mapping(&sidecar);
    assert_eq!(mapping["file"].as_str(), Some("src/thing.rs"));
    let span = mapping_span(&sidecar);
    assert!(
        span <= 3,
        "impl mapping must be a single-function range (fn alpha is 3 lines; got span {span})"
    );
}

#[test]
fn maintainer_migrates_or_retires_dead_path_mappings() {
    // @step Given a set of coverage files whose test or impl mappings reference files that no longer exist
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "src/thing.rs", IMPL_SOURCE);
    write_file(tmp.path(), "tests/thing.rs", FIXTURE_TEST);
    write_file(tmp.path(), "spec/features/thing.feature", FIXTURE_FEATURE);
    // The sidecar still maps to the deleted JS-era files (both dead).
    let sidecar = json!({
        "scenarios": [{
            "name": SCENARIO,
            "testMappings": [{
                "file": "tests/old_thing.js",
                "lines": "1-10",
                "implMappings": [{ "file": "src/old_thing.js", "lines": [1, 2, 3, 4, 5] }]
            }]
        }],
        "stats": {
            "totalScenarios": 1,
            "coveredScenarios": 1,
            "coveragePercent": 100,
            "testFiles": ["tests/old_thing.js"],
            "implFiles": ["src/old_thing.js"],
            "totalLinesCovered": 15
        }
    });
    write_file(
        tmp.path(),
        "spec/features/thing.feature.coverage",
        &serde_json::to_string(&sidecar).expect("sidecar is JSON"),
    );
    let (clean, report) = audit(&dispatch_command(
        req(
            tmp.path(),
            "audit-coverage",
            json!({ "featureName": "thing" }),
        ),
    ));
    assert!(!clean, "pre-migration audit must flag the dead mappings, got: {report}");
    assert!(
        report.contains("Test file not found: tests/old_thing.js"),
        "audit must name the dead test file, got: {report}"
    );
    assert!(
        report.contains("Implementation file not found: src/old_thing.js"),
        "audit must name the dead impl file, got: {report}"
    );

    // @step When the maintainer migrates each feature with a live Rust port to its new paths and line ranges
    // Retire the dead test mapping (and its dead impl mapping), then link
    // the live Rust port.
    let retired = dispatch_command(
        req(
            tmp.path(),
            "unlink-coverage",
            json!({
                "featureName": "thing",
                "scenario": SCENARIO,
                "testFile": "tests/old_thing.js"
            }),
        ),
    );
    assert!(
        retired.success,
        "retiring the dead test mapping must succeed, got {:?}",
        retired.error
    );
    let migrated = dispatch_command(
        req(
            tmp.path(),
            "link-coverage",
            json!({
                "featureName": "thing",
                "scenario": SCENARIO,
                "testFile": "tests/thing.rs",
                "testLines": "1-6",
                "implFile": "src/thing.rs",
                "implLines": "1-3"
            }),
        ),
    );
    assert!(
        migrated.success,
        "migration link to the live Rust port must succeed, got {:?}",
        migrated.error
    );

    // @step And the maintainer retires each coverage file with no live mapping from the coverage set
    // (the migrated sidecar only references live files — nothing dead remains)
    let sidecar = read_sidecar(tmp.path(), "thing");
    for_each_mapped_file(&sidecar, |file| {
        assert!(
            tmp.path().join(file).exists(),
            "mapping must reference a live file: {file}"
        );
    });

    // @step And the maintainer re-runs the structural audit
    let (clean, report) = audit(&dispatch_command(
        req(
            tmp.path(),
            "audit-coverage",
            json!({ "featureName": "thing" }),
        ),
    ));

    // @step Then zero file-missing defects remain for the migrated features
    assert!(
        clean,
        "audit must be clean after migration, got: {report}"
    );
    assert!(
        !report.contains("not found") && !report.contains("missing"),
        "no file-missing defects may remain, got: {report}"
    );

    // @step And zero lines-past-EOF defects remain for the migrated features
    assert!(
        !report.contains("past EOF") && !report.contains("beyond"),
        "no lines-past-EOF defects may remain, got: {report}"
    );
    // The audit report names only file-missing defects, so verify the EOF
    // criterion directly: every mapped line must lie within the mapped file.
    let sidecar = read_sidecar(tmp.path(), "thing");
    for scenario in sidecar["scenarios"].as_array().expect("scenarios array") {
        for mapping in scenario["testMappings"].as_array().expect("test mappings") {
            for impl_mapping in mapping["implMappings"].as_array().expect("impl mappings") {
                let file = impl_mapping["file"].as_str().expect("impl file");
                let file_len = fs::read_to_string(tmp.path().join(file))
                    .expect("read impl file")
                    .lines()
                    .count() as i64;
                for line in impl_mapping["lines"].as_array().expect("impl lines") {
                    assert!(
                        line.as_i64().expect("line") <= file_len,
                        "impl mapping line past EOF in {file}"
                    );
                }
            }
        }
    }
}
