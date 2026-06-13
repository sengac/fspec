#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_closure,
    clippy::redundant_closure_for_method_calls,
    clippy::uninlined_format_args
)]
// Feature: spec/features/show-coverage-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `show-coverage`
// (RPC-300). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "show-coverage".to_string(),
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

/// Build a coverage file body with a custom scenarios array (raw JSON string)
/// and a computed stats object derived from the inputs.
fn make_coverage(scenarios_json: &str, stats_json: &str) -> String {
    format!(
        r#"{{
  "scenarios": {sc},
  "stats": {st}
}}"#,
        sc = scenarios_json,
        st = stats_json
    )
}

fn coverage_fully_covered(n_total: usize, n_covered: usize) -> String {
    let mut scenarios = Vec::new();
    for i in 0..n_covered {
        scenarios.push(format!(
            r#"{{
      "name": "S{i}",
      "testMappings": [
        {{ "file": "tests/t{i}.ts", "lines": "1-10",
          "implMappings": [ {{ "file": "src/s{i}.ts", "lines": "1-5" }} ]
        }}
      ]
    }}"#
        ));
    }
    for i in n_covered..n_total {
        scenarios.push(format!(r#"{{ "name": "S{i}", "testMappings": [] }}"#));
    }
    let percent = if n_total == 0 {
        0
    } else {
        ((n_covered as f64) / (n_total as f64) * 100.0).round() as u32
    };
    let stats = format!(
        r#"{{
    "totalScenarios": {n_total},
    "coveredScenarios": {n_covered},
    "coveragePercent": {percent},
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }}"#
    );
    make_coverage(
        &format!("[\n    {}\n  ]", scenarios.join(",\n    ")),
        &stats,
    )
}

/// Write referenced test+impl files for `n_covered` scenarios so warnings don't fire.
fn write_referenced_files(root: &Path, n_covered: usize) {
    for i in 0..n_covered {
        write_file(root, &format!("tests/t{i}.ts"), "// t\n");
        write_file(root, &format!("src/s{i}.ts"), "// s\n");
    }
}

// ───────── Per-feature mode scenarios ─────────

#[test]
fn scenario_bare_feature_name_resolves_and_renders_markdown() {
    // Scenario: Bare feature name resolves to spec/features/<name>.feature.coverage and renders markdown report

    // @step Given a temp project root contains spec/features/user-login.feature.coverage with 5 scenarios, 4 of which have testMappings with implMappings and 1 with no testMappings
    let tmp = TempDir::new().expect("tempdir");
    write_referenced_files(tmp.path(), 4);
    let body = coverage_fully_covered(5, 4);
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        &body,
    );

    // @step When I dispatch show-coverage with featureName='user-login' (no format)
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "user-login"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered string starts with the line '# Coverage Report: user-login.feature'
    assert!(
        result
            .data
            .starts_with("# Coverage Report: user-login.feature"),
        "rendered must start with title; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '**Coverage**: 80% (4/5 scenarios)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "**Coverage**: 80% (4/5 scenarios)"),
        "rendered must contain coverage line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_trailing_feature_extension_tolerated() {
    // Scenario: Trailing .feature on the bare name is tolerated

    // @step Given a temp project root contains spec/features/user-login.feature.coverage with 5 scenarios, 4 of which have testMappings with implMappings and 1 with no testMappings
    let tmp = TempDir::new().expect("tempdir");
    write_referenced_files(tmp.path(), 4);
    let body = coverage_fully_covered(5, 4);
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        &body,
    );

    // @step When I dispatch show-coverage with featureName='user-login.feature'
    let result_with_ext = dispatch_command(req(
        tmp.path(),
        json!({"featureName": "user-login.feature"}),
    ));
    let result_bare = dispatch_command(req(tmp.path(), json!({"featureName": "user-login"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result_with_ext.success, "got {result_with_ext:?}");

    // @step And the rendered string is byte-equal to the call made with featureName='user-login'
    assert_eq!(result_with_ext.data, result_bare.data);
}

#[test]
fn scenario_missing_coverage_file_io_error() {
    // Scenario: Missing coverage file is reported as an Io error with TS-parity message

    // @step Given a temp project root contains spec/features/ but no missing.feature.coverage
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");

    // @step When I dispatch show-coverage with featureName='missing'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "missing"})));

    // @step Then the call returns Err(FspecCoreError::Io)
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();

    // @step And the error's message contains 'Coverage file not found: missing.feature.coverage'
    assert!(
        err.contains("Coverage file not found: missing.feature.coverage"),
        "error must mention not-found; got: {err}"
    );

    // @step And the error's message contains "Suggestion: Run 'fspec create-feature' to create the feature with coverage tracking"
    assert!(
        err.contains(
            "Suggestion: Run 'fspec create-feature' to create the feature with coverage tracking"
        ),
        "error must include create-feature suggestion; got: {err}"
    );
}

#[test]
fn scenario_invalid_json_invalid_args_error() {
    // Scenario: Invalid JSON in the coverage file is reported as an InvalidArgs error

    // @step Given a temp project root contains spec/features/broken.feature.coverage with the bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/broken.feature.coverage",
        "{ not json",
    );

    // @step When I dispatch show-coverage with featureName='broken'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "broken"})));

    // @step Then the call returns Err(FspecCoreError::InvalidArgs)
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();

    // @step And the error's message starts with 'Invalid JSON in coverage file: broken.feature.coverage'
    assert!(
        err.contains("Invalid JSON in coverage file: broken.feature.coverage"),
        "error must reference invalid JSON; got: {err}"
    );

    // @step And the error's message contains 'Parse error:'
    assert!(
        err.contains("Parse error:"),
        "error must include Parse error sub-line; got: {err}"
    );

    // @step And the error's message contains 'Suggestion: Validate the JSON or recreate the file'
    assert!(
        err.contains("Suggestion: Validate the JSON or recreate the file"),
        "error must include validate-suggestion; got: {err}"
    );
}

#[test]
fn scenario_markdown_summary_section_order() {
    // Scenario: Markdown summary section is emitted in the exact TS order

    // @step Given a temp project root contains spec/features/feat.feature.coverage with 3 scenarios all fully covered (testMappings with implMappings)
    let tmp = TempDir::new().expect("tempdir");
    write_referenced_files(tmp.path(), 3);
    let body = coverage_fully_covered(3, 3);
    write_file(tmp.path(), "spec/features/feat.feature.coverage", &body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string contains a '## Summary' section
    assert!(
        result.data.contains("## Summary"),
        "must contain Summary section; got:\n{}",
        result.data
    );

    // @step And immediately under '## Summary' the lines appear in this order:
    // 'Total Scenarios', 'Covered', 'Uncovered', 'Test Files', 'Implementation Files', 'Test Lines', 'Implementation Lines', 'Total Lines'
    let summary_idx = result.data.find("## Summary").expect("Summary section");
    let after = &result.data[summary_idx..];
    let p_total = after.find("Total Scenarios").expect("Total Scenarios");
    let p_covered = after.find("Covered").expect("Covered");
    let p_uncovered = after.find("Uncovered").expect("Uncovered");
    let p_test_files = after.find("Test Files").expect("Test Files");
    let p_impl_files = after
        .find("Implementation Files")
        .expect("Implementation Files");
    let p_test_lines = after.find("Test Lines").expect("Test Lines");
    let p_impl_lines = after
        .find("Implementation Lines")
        .expect("Implementation Lines");
    let p_total_lines = after.find("Total Lines").expect("Total Lines");
    assert!(p_total < p_covered);
    assert!(p_covered < p_uncovered);
    assert!(p_uncovered < p_test_files);
    assert!(p_test_files < p_impl_files);
    assert!(p_impl_files < p_test_lines);
    assert!(p_test_lines < p_impl_lines);
    assert!(p_impl_lines < p_total_lines);
}

#[test]
fn scenario_per_scenario_fully_covered_label() {
    // Scenario: Per-scenario block uses ✅ / FULLY COVERED label for scenarios with implMappings

    // @step Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Login' has 1 testMapping with 1 implMapping referencing existing files
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/login.test.ts", "// t\n");
    write_file(tmp.path(), "src/login.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        { "file": "tests/login.test.ts", "lines": "1-10",
          "implMappings": [ { "file": "src/login.ts", "lines": "1-5" } ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
    "testFiles": ["tests/login.test.ts"], "implFiles": ["src/login.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));

    // @step Then the rendered string contains the line '### ✅ Login (FULLY COVERED)'
    assert!(result.success, "got {result:?}");
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "### ✅ Login (FULLY COVERED)"),
        "must contain Fully Covered scenario block; got:\n{}",
        result.data
    );

    // @step And the rendered string contains a '- **Test**: ' bullet for that scenario
    assert!(
        result.data.contains("- **Test**: "),
        "must contain Test bullet; got:\n{}",
        result.data
    );

    // @step And the rendered string contains a '- **Implementation**: ' bullet for that scenario
    assert!(
        result.data.contains("- **Implementation**: "),
        "must contain Implementation bullet; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_per_scenario_partially_covered_label() {
    // Scenario: Per-scenario block uses ⚠️ / PARTIALLY COVERED label when testMapping has no implMappings

    // @step Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Logout' has 1 testMapping but ZERO implMappings
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/logout.test.ts", "// t\n");
    let body = r#"{
  "scenarios": [
    {
      "name": "Logout",
      "testMappings": [
        { "file": "tests/logout.test.ts", "lines": "1-10", "implMappings": [] }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
    "testFiles": ["tests/logout.test.ts"], "implFiles": [],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains the line '### ⚠️ Logout (PARTIALLY COVERED)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "### ⚠️ Logout (PARTIALLY COVERED)"),
        "must contain Partially Covered block; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '- **Implementation**: ⚠️  No implementation mappings'
    assert!(
        result
            .data
            .contains("- **Implementation**: ⚠️  No implementation mappings"),
        "must contain no-impl-mappings bullet; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_per_scenario_uncovered_label_and_gaps_section() {
    // Scenario: Per-scenario block uses ❌ / UNCOVERED label and the file gains a Coverage Gaps section

    // @step Given a temp project root contains spec/features/feat.feature.coverage where scenario 'Reset' has ZERO testMappings
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "scenarios": [
    { "name": "Reset", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
    "testFiles": [], "implFiles": [], "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains the line '### ❌ Reset (UNCOVERED)'
    assert!(
        result.data.lines().any(|l| l == "### ❌ Reset (UNCOVERED)"),
        "must contain Uncovered block; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '- No test mappings'
    assert!(
        result.data.contains("- No test mappings"),
        "must contain no-test-mappings bullet; got:\n{}",
        result.data
    );

    // @step And the rendered string contains a '## ⚠️  Coverage Gaps' section preceded by a '---' separator
    assert!(
        result.data.contains("---") && result.data.contains("## ⚠️  Coverage Gaps"),
        "must contain Coverage Gaps section with separator; got:\n{}",
        result.data
    );

    // @step And the gaps section contains the bullet '- Reset'
    let gaps_idx = result
        .data
        .find("## ⚠️  Coverage Gaps")
        .expect("gaps section");
    let after = &result.data[gaps_idx..];
    assert!(
        after.contains("- Reset"),
        "gaps section must list Reset; got:\n{}",
        after
    );
}

#[test]
fn scenario_missing_file_warnings_section() {
    // Scenario: Missing referenced test or impl files are surfaced as Warnings section but command still succeeds

    // @step Given a temp project root contains spec/features/feat.feature.coverage referencing src/__tests__/deleted.test.ts as a testMapping file
    // @step And the file src/__tests__/deleted.test.ts does NOT exist on disk
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "scenarios": [
    {
      "name": "Gone",
      "testMappings": [
        { "file": "src/__tests__/deleted.test.ts", "lines": "1-10", "implMappings": [] }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
    "testFiles": ["src/__tests__/deleted.test.ts"], "implFiles": [],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string contains a '## Warnings' section
    assert!(
        result.data.contains("## Warnings"),
        "must contain Warnings section; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '⚠️  File not found: src/__tests__/deleted.test.ts'
    assert!(
        result
            .data
            .contains("⚠️  File not found: src/__tests__/deleted.test.ts"),
        "must contain file-not-found warning; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_legacy_coverage_no_stats_calculated_silently() {
    // Scenario: Legacy coverage file without a stats key has stats calculated silently from scenarios

    // @step Given a temp project root contains spec/features/legacy.feature.coverage whose top-level JSON object omits the 'stats' key but has 4 scenarios all with testMappings
    let tmp = TempDir::new().expect("tempdir");
    write_referenced_files(tmp.path(), 4);
    let body = r#"{
  "scenarios": [
    { "name": "S0", "testMappings": [{ "file": "tests/t0.ts", "lines": "1-10", "implMappings": [{"file":"src/s0.ts","lines":"1-5"}] }] },
    { "name": "S1", "testMappings": [{ "file": "tests/t1.ts", "lines": "1-10", "implMappings": [{"file":"src/s1.ts","lines":"1-5"}] }] },
    { "name": "S2", "testMappings": [{ "file": "tests/t2.ts", "lines": "1-10", "implMappings": [{"file":"src/s2.ts","lines":"1-5"}] }] },
    { "name": "S3", "testMappings": [{ "file": "tests/t3.ts", "lines": "1-10", "implMappings": [{"file":"src/s3.ts","lines":"1-5"}] }] }
  ]
}"#;
    write_file(tmp.path(), "spec/features/legacy.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='legacy'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "legacy"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string contains the line '**Coverage**: 100% (4/4 scenarios)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "**Coverage**: 100% (4/4 scenarios)"),
        "must contain 100% coverage line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_legacy_stats_dedup_and_math_round() {
    // Scenario: Legacy stats calculation deduplicates test and impl files and rounds coveragePercent with Math.round semantics

    // @step Given a temp project root contains spec/features/legacy.feature.coverage with no stats key and 2 scenarios where scenario A has a testMapping referencing test1.ts and implMapping referencing impl1.ts and scenario B has no testMappings
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "test1.ts", "// t\n");
    write_file(tmp.path(), "impl1.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    { "name": "A", "testMappings": [{ "file": "test1.ts", "lines": "1-10", "implMappings": [{"file":"impl1.ts","lines":"1-5"}] }] },
    { "name": "B", "testMappings": [] }
  ]
}"#;
    write_file(tmp.path(), "spec/features/legacy.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='legacy' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"featureName": "legacy", "format": "json"}),
    ));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    let v: Value = serde_json::from_str(&result.data).expect("must be JSON");

    // @step And the rendered JSON's stats.testFiles equals ['test1.ts']
    let test_files = v["stats"]["testFiles"].as_array().expect("testFiles array");
    assert_eq!(test_files.len(), 1);
    assert_eq!(test_files[0].as_str(), Some("test1.ts"));

    // @step And the rendered JSON's stats.implFiles equals ['impl1.ts']
    let impl_files = v["stats"]["implFiles"].as_array().expect("implFiles array");
    assert_eq!(impl_files.len(), 1);
    assert_eq!(impl_files[0].as_str(), Some("impl1.ts"));

    // @step And the rendered JSON's stats.coveragePercent equals 50
    assert_eq!(v["stats"]["coveragePercent"].as_i64(), Some(50));
}

#[test]
fn scenario_line_counting_test_and_impl_ranges() {
    // Scenario: Line counting accumulates test ranges and both array-and-string impl ranges

    // @step Given a temp project root contains spec/features/feat.feature.coverage with one scenario whose testMapping.lines='45-62' and implMappings contain one with lines=[10,11,12,23,24] and one with lines='1-149'
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/t.ts", "// t\n");
    write_file(tmp.path(), "src/a.ts", "// s\n");
    write_file(tmp.path(), "src/b.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    {
      "name": "S",
      "testMappings": [
        { "file": "tests/t.ts", "lines": "45-62",
          "implMappings": [
            { "file": "src/a.ts", "lines": [10, 11, 12, 23, 24] },
            { "file": "src/b.ts", "lines": "1-149" }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
    "testFiles": ["tests/t.ts"], "implFiles": ["src/a.ts", "src/b.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat'
    let result = dispatch_command(req(tmp.path(), json!({"featureName": "feat"})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered markdown contains the line '- Test Lines: 18'
    assert!(
        result.data.lines().any(|l| l == "- Test Lines: 18"),
        "test lines must be 18 (62-45+1); got:\n{}",
        result.data
    );

    // @step And the rendered markdown contains the line '- Implementation Lines: 154'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "- Implementation Lines: 154"),
        "impl lines must be 154 (5 + 149); got:\n{}",
        result.data
    );

    // @step And the rendered markdown contains the line '- Total Lines: 172'
    assert!(
        result.data.lines().any(|l| l == "- Total Lines: 172"),
        "total lines must be 172; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_single_file_json_key_order() {
    // Scenario: JSON format for single-file mode emits 2-space-indented object with keys in declaration order

    // @step Given a temp project root contains spec/features/feat.feature.coverage with 2 scenarios and a stats key
    let tmp = TempDir::new().expect("tempdir");
    write_referenced_files(tmp.path(), 2);
    let body = coverage_fully_covered(2, 2);
    write_file(tmp.path(), "spec/features/feat.feature.coverage", &body);

    // @step When I dispatch show-coverage with featureName='feat' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"featureName": "feat", "format": "json"}),
    ));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string parses as JSON
    let v: Value = serde_json::from_str(&result.data).expect("must be JSON");

    // @step And the rendered JSON's top-level keys in declaration order are 'fileName', 'scenarios', 'stats', 'warnings'
    let obj = v.as_object().expect("root is object");
    let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    assert!(
        keys.starts_with(&["fileName", "scenarios", "stats"]),
        "first three keys must be fileName, scenarios, stats; got: {keys:?}"
    );

    // @step And the rendered JSON's fileName equals 'feat.feature'
    assert_eq!(v["fileName"].as_str(), Some("feat.feature"));

    // @step And each entry in the rendered JSON's scenarios array has an appended 'coverageStatus' field whose value is one of 'fully-covered', 'partially-covered', 'uncovered'
    for s in v["scenarios"].as_array().expect("scenarios array") {
        let status = s["coverageStatus"].as_str().expect("coverageStatus");
        assert!(
            matches!(status, "fully-covered" | "partially-covered" | "uncovered"),
            "coverageStatus must be one of the three values; got: {status}"
        );
    }

    // @step And the rendered string uses 2-space indentation
    assert!(
        result.data.contains("\n  \"fileName\""),
        "must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_single_file_json_omits_warnings_when_no_missing_files() {
    // Scenario: Single-file JSON omits warnings field when no files are missing

    // @step Given a temp project root contains spec/features/feat.feature.coverage with one scenario referencing only files that exist on disk
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/t.ts", "// t\n");
    write_file(tmp.path(), "src/s.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    {
      "name": "S",
      "testMappings": [
        { "file": "tests/t.ts", "lines": "1-10",
          "implMappings": [{"file":"src/s.ts","lines":"1-5"}] }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
    "testFiles": ["tests/t.ts"], "implFiles": ["src/s.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/feat.feature.coverage", body);

    // @step When I dispatch show-coverage with featureName='feat' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"featureName": "feat", "format": "json"}),
    ));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered JSON's warnings field is null or omitted
    let v: Value = serde_json::from_str(&result.data).expect("JSON");
    let w = &v["warnings"];
    assert!(
        w.is_null() || w == &Value::Null,
        "warnings must be null when no missing files; got: {w}"
    );
}

// ───────── Project-wide mode scenarios ─────────

fn write_project_wide_workspace(tmp: &Path) {
    // a.feature.coverage: 2 fully covered scenarios
    write_referenced_files(tmp, 2);
    write_file(tmp, "tests/a0.ts", "// t\n");
    write_file(tmp, "tests/a1.ts", "// t\n");
    write_file(tmp, "src/a0.ts", "// s\n");
    write_file(tmp, "src/a1.ts", "// s\n");
    let a_body = r#"{
  "scenarios": [
    { "name": "A0", "testMappings": [{ "file": "tests/a0.ts", "lines": "1-10", "implMappings": [{"file":"src/a0.ts","lines":"1-5"}] }] },
    { "name": "A1", "testMappings": [{ "file": "tests/a1.ts", "lines": "1-10", "implMappings": [{"file":"src/a1.ts","lines":"1-5"}] }] }
  ],
  "stats": {
    "totalScenarios": 2, "coveredScenarios": 2, "coveragePercent": 100,
    "testFiles": ["tests/a0.ts", "tests/a1.ts"], "implFiles": ["src/a0.ts", "src/a1.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp, "spec/features/a.feature.coverage", a_body);
    // b.feature.coverage: 2 scenarios, 1 fully covered + 1 uncovered
    write_file(tmp, "tests/b0.ts", "// t\n");
    write_file(tmp, "src/b0.ts", "// s\n");
    let b_body = r#"{
  "scenarios": [
    { "name": "B0", "testMappings": [{ "file": "tests/b0.ts", "lines": "1-10", "implMappings": [{"file":"src/b0.ts","lines":"1-5"}] }] },
    { "name": "B1", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50,
    "testFiles": ["tests/b0.ts"], "implFiles": ["src/b0.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp, "spec/features/b.feature.coverage", b_body);
}

#[test]
fn scenario_project_wide_aggregates_totals() {
    // Scenario: Project-wide mode aggregates totals and prints '# Project Coverage Report'

    // @step Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios both fully covered AND spec/features/b.feature.coverage with 2 scenarios, 1 fully covered and 1 uncovered
    let tmp = TempDir::new().expect("tempdir");
    write_project_wide_workspace(tmp.path());

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string starts with the line '# Project Coverage Report'
    assert!(
        result.data.starts_with("# Project Coverage Report"),
        "must start with Project Coverage Report; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '**Overall Coverage**: 75% (3/4 scenarios)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "**Overall Coverage**: 75% (3/4 scenarios)"),
        "must contain overall coverage line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_project_summary_section_order() {
    // Scenario: Project Summary section appears in TS field order

    // @step Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios both fully covered AND spec/features/b.feature.coverage with 2 scenarios, 1 fully covered and 1 uncovered
    let tmp = TempDir::new().expect("tempdir");
    write_project_wide_workspace(tmp.path());

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains a '## Project Summary' section
    assert!(
        result.data.contains("## Project Summary"),
        "must contain Project Summary; got:\n{}",
        result.data
    );

    // @step And inside Project Summary the lines appear in this order: 'Total Features: 2', 'Total Scenarios: 4', 'Covered: 3', 'Uncovered: 1'
    let ps = result
        .data
        .find("## Project Summary")
        .expect("Project Summary");
    let after = &result.data[ps..];
    let p_features = after.find("Total Features: 2").expect("Total Features: 2");
    let p_scenarios = after
        .find("Total Scenarios: 4")
        .expect("Total Scenarios: 4");
    let p_covered = after.find("Covered: 3").expect("Covered: 3");
    let p_uncovered = after.find("Uncovered: 1").expect("Uncovered: 1");
    assert!(p_features < p_scenarios);
    assert!(p_scenarios < p_covered);
    assert!(p_covered < p_uncovered);
}

#[test]
fn scenario_features_overview_100_percent_uses_check() {
    // Scenario: Features Overview uses ✅ for 100% coverage features

    // @step Given a temp project root contains spec/features/full.feature.coverage where the stats.coveragePercent equals 100
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/f.ts", "// t\n");
    write_file(tmp.path(), "src/f.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    { "name": "F", "testMappings": [{ "file": "tests/f.ts", "lines": "1-10", "implMappings": [{"file":"src/f.ts","lines":"1-5"}] }] }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
    "testFiles": ["tests/f.ts"], "implFiles": ["src/f.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/full.feature.coverage", body);

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains the line '- full.feature: 100% (.*) ✅' (regex match)
    let line = result
        .data
        .lines()
        .find(|l| l.starts_with("- full.feature: 100%"))
        .unwrap_or_else(|| panic!("expected Features Overview entry; got:\n{}", result.data));
    assert!(
        line.ends_with("✅"),
        "100% line must end with ✅; got: {line}"
    );
}

#[test]
fn scenario_features_overview_50_percent_uses_warning() {
    // Scenario: Features Overview uses ⚠️ for ≥50% but <100% coverage features

    // @step Given a temp project root contains spec/features/half.feature.coverage where the stats.coveragePercent equals 50
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/h.ts", "// t\n");
    write_file(tmp.path(), "src/h.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    { "name": "H0", "testMappings": [{ "file": "tests/h.ts", "lines": "1-10", "implMappings": [{"file":"src/h.ts","lines":"1-5"}] }] },
    { "name": "H1", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50,
    "testFiles": ["tests/h.ts"], "implFiles": ["src/h.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/half.feature.coverage", body);

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains the substring 'half.feature: 50%'
    assert!(
        result.data.contains("half.feature: 50%"),
        "must contain half.feature line; got:\n{}",
        result.data
    );

    // @step And the line containing 'half.feature: 50%' ends with the ⚠️ symbol
    let line = result
        .data
        .lines()
        .find(|l| l.contains("half.feature: 50%"))
        .expect("half.feature line");
    assert!(
        line.ends_with("⚠️"),
        "half.feature line must end with ⚠️; got: {line}"
    );
}

#[test]
fn scenario_features_overview_zero_percent_uses_cross() {
    // Scenario: Features Overview uses ❌ for <50% coverage features including 0%

    // @step Given a temp project root contains spec/features/none.feature.coverage where the stats.coveragePercent equals 0
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "scenarios": [
    { "name": "N", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 0, "coveragePercent": 0,
    "testFiles": [], "implFiles": [], "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/none.feature.coverage", body);

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains the substring 'none.feature: 0%'
    assert!(
        result.data.contains("none.feature: 0%"),
        "must contain none.feature line; got:\n{}",
        result.data
    );

    // @step And the line containing 'none.feature: 0%' ends with the ❌ symbol
    let line = result
        .data
        .lines()
        .find(|l| l.contains("none.feature: 0%"))
        .expect("none.feature line");
    assert!(
        line.ends_with("❌"),
        "0% line must end with ❌; got: {line}"
    );
}

#[test]
fn scenario_project_wide_detailed_coverage_section() {
    // Scenario: Project-wide mode emits Detailed Coverage by Feature with per-feature scenario list

    // @step Given a temp project root contains spec/features/a.feature.coverage with scenarios 'X' (covered) and 'Y' (uncovered)
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/x.ts", "// t\n");
    write_file(tmp.path(), "src/x.ts", "// s\n");
    let body = r#"{
  "scenarios": [
    { "name": "X", "testMappings": [{ "file": "tests/x.ts", "lines": "1-10", "implMappings": [{"file":"src/x.ts","lines":"1-5"}] }] },
    { "name": "Y", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50,
    "testFiles": ["tests/x.ts"], "implFiles": ["src/x.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/a.feature.coverage", body);

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "got {result:?}");

    // @step Then the rendered string contains a '---' separator followed by '## Detailed Coverage by Feature'
    assert!(
        result.data.contains("---") && result.data.contains("## Detailed Coverage by Feature"),
        "must contain separator + Detailed Coverage section; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '### a.feature'
    assert!(
        result.data.lines().any(|l| l == "### a.feature"),
        "must contain ### a.feature header; got:\n{}",
        result.data
    );

    // @step And the rendered string contains a per-scenario bullet '- ✅ X'
    assert!(
        result.data.lines().any(|l| l == "- ✅ X"),
        "must contain per-scenario covered bullet; got:\n{}",
        result.data
    );

    // @step And the rendered string contains a per-scenario bullet '- ❌ Y'
    assert!(
        result.data.lines().any(|l| l == "- ❌ Y"),
        "must contain per-scenario uncovered bullet; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_project_wide_silently_skips_invalid_json() {
    // Scenario: Project-wide mode silently skips coverage files whose JSON fails to parse

    // @step Given a temp project root contains spec/features/good.feature.coverage with 1 fully covered scenario AND spec/features/bad.feature.coverage with the bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_file(tmp.path(), "tests/g.ts", "// t\n");
    write_file(tmp.path(), "src/g.ts", "// s\n");
    let good = r#"{
  "scenarios": [
    { "name": "G", "testMappings": [{ "file": "tests/g.ts", "lines": "1-10", "implMappings": [{"file":"src/g.ts","lines":"1-5"}] }] }
  ],
  "stats": {
    "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100,
    "testFiles": ["tests/g.ts"], "implFiles": ["src/g.ts"],
    "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/good.feature.coverage", good);
    write_file(
        tmp.path(),
        "spec/features/bad.feature.coverage",
        "{ not json",
    );

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string contains 'good.feature'
    assert!(
        result.data.contains("good.feature"),
        "must contain good.feature; got:\n{}",
        result.data
    );

    // @step And the rendered string does NOT contain 'bad.feature'
    assert!(
        !result.data.contains("bad.feature"),
        "must NOT contain bad.feature; got:\n{}",
        result.data
    );

    // @step And the rendered string contains the line '**Overall Coverage**: 100% (1/1 scenarios)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "**Overall Coverage**: 100% (1/1 scenarios)"),
        "must show 100% overall; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_project_wide_missing_features_dir_is_error() {
    // Scenario: Project-wide mode errors when spec/features directory does not exist

    // @step Given a temp project root with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the call returns Err(FspecCoreError::Io)
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();

    // @step And the error's message contains 'Features directory not found: spec/features/'
    assert!(
        err.contains("Features directory not found: spec/features/"),
        "error must mention missing features dir; got: {err}"
    );

    // @step And the error's message contains "Suggestion: Run 'fspec create-feature' to create your first feature"
    assert!(
        err.contains("Suggestion: Run 'fspec create-feature' to create your first feature"),
        "error must include create-feature suggestion; got: {err}"
    );
}

#[test]
fn scenario_project_wide_empty_features_dir_is_error() {
    // Scenario: Project-wide mode errors when spec/features exists but contains no .feature.coverage files

    // @step Given a temp project root contains spec/features/ but no *.feature.coverage files
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the call returns Err(FspecCoreError::Io)
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();

    // @step And the error's message contains 'No coverage files found in spec/features/'
    assert!(
        err.contains("No coverage files found in spec/features/"),
        "error must say no coverage files; got: {err}"
    );

    // @step And the error's message contains "Suggestion: Run 'fspec create-feature' to create features with coverage tracking"
    assert!(
        err.contains(
            "Suggestion: Run 'fspec create-feature' to create features with coverage tracking"
        ),
        "error must include create-feature suggestion; got: {err}"
    );
}

#[test]
fn scenario_project_wide_zero_scenarios_no_nan() {
    // Scenario: Project-wide mode with zero scenarios across all features renders Overall Coverage 0% without NaN

    // @step Given a temp project root contains spec/features/empty.feature.coverage whose scenarios array is empty
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "scenarios": [],
  "stats": {
    "totalScenarios": 0, "coveredScenarios": 0, "coveragePercent": 0,
    "testFiles": [], "implFiles": [], "totalLinesCovered": 0
  }
}"#;
    write_file(tmp.path(), "spec/features/empty.feature.coverage", body);

    // @step When I dispatch show-coverage with no featureName
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string contains the line '**Overall Coverage**: 0% (0/0 scenarios)'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "**Overall Coverage**: 0% (0/0 scenarios)"),
        "must contain 0% overall with no NaN; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_project_wide_json_key_order() {
    // Scenario: Project-wide JSON format emits 2-space-indented object with declaration-order root keys

    // @step Given a temp project root contains spec/features/a.feature.coverage with 2 scenarios and spec/features/b.feature.coverage with 1 scenario
    let tmp = TempDir::new().expect("tempdir");
    write_project_wide_workspace(tmp.path());

    // @step When I dispatch show-coverage with no featureName and format='json'
    let result = dispatch_command(req(tmp.path(), json!({"format": "json"})));

    // @step Then the call returns Ok(rendered_string)
    assert!(result.success, "got {result:?}");

    // @step And the rendered string parses as JSON
    let v: Value = serde_json::from_str(&result.data).expect("must be JSON");

    // @step And the rendered JSON's top-level keys in declaration order are 'aggregated', 'features'
    let obj = v.as_object().expect("root is object");
    let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    assert_eq!(
        keys,
        vec!["aggregated", "features"],
        "top-level keys must be aggregated, features in order; got: {keys:?}"
    );

    // @step And the rendered JSON's aggregated field has keys in declaration order: 'totalFeatures', 'totalScenarios', 'coveredScenarios', 'coveragePercent'
    let agg = v["aggregated"].as_object().expect("aggregated is object");
    let agg_keys: Vec<&str> = agg.keys().map(|s| s.as_str()).collect();
    assert_eq!(
        agg_keys,
        vec![
            "totalFeatures",
            "totalScenarios",
            "coveredScenarios",
            "coveragePercent"
        ],
        "aggregated keys order; got: {agg_keys:?}"
    );

    // @step And the rendered JSON's features field is an array of objects with declaration-order keys 'fileName', 'coverage'
    let features = v["features"].as_array().expect("features array");
    assert!(!features.is_empty(), "features must be non-empty");
    let f0 = features[0].as_object().expect("feature[0] is object");
    let f0_keys: Vec<&str> = f0.keys().map(|s| s.as_str()).collect();
    assert!(
        f0_keys.starts_with(&["fileName", "coverage"]),
        "feature entry keys must start with fileName, coverage; got: {f0_keys:?}"
    );

    // @step And the rendered string uses 2-space indentation
    assert!(
        result.data.contains("\n  \"aggregated\""),
        "must use 2-space indentation; got:\n{}",
        result.data
    );
}

// ───────── Two-front-doors / shared infrastructure ─────────

#[test]
fn scenario_invalid_args_json_returns_invalid_args_error() {
    // Scenario: args_json that fails to parse returns FspecCoreError::InvalidArgs

    // @step Given an arbitrary project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I call show_coverage::run with args_json='{ not valid json'
    let request = DispatchRequest {
        command: "show-coverage".to_string(),
        args_json: "{ not valid json".to_string(),
        project_root: tmp.path().to_path_buf(),
    };
    let result = dispatch_command(request);

    // @step Then the call returns Err(FspecCoreError::InvalidArgs)
    assert!(
        !result.success,
        "expected failure on malformed args_json; got {result:?}"
    );

    // @step And the error's command field equals 'show-coverage'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("show-coverage"),
        "error must mention command name show-coverage; got: {err}"
    );
}

#[test]
fn scenario_coverage_types_module_publicly_accessible() {
    // Scenario: Coverage sidecar types module is publicly accessible from the crate root

    // @step Given the codelet/fspec-core crate is built

    // @step When I inspect codelet/fspec-core/src/types/
    let types_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/types");
    assert!(types_dir.exists(), "src/types must exist");
    let coverage_rs = types_dir.join("coverage.rs");

    // @step Then the module types::coverage exists and exposes CoverageFile, CoverageScenario, TestMapping, ImplMapping, CoverageStats
    assert!(
        coverage_rs.exists(),
        "src/types/coverage.rs must exist; expected at: {}",
        coverage_rs.display()
    );
    let body = fs::read_to_string(&coverage_rs).expect("coverage.rs readable");
    for sym in [
        "CoverageFile",
        "CoverageScenario",
        "TestMapping",
        "ImplMapping",
        "CoverageStats",
    ] {
        assert!(
            body.contains(sym),
            "coverage.rs must define {sym}; got:\n{body}"
        );
    }

    // @step And the CoverageFile struct uses #[serde(rename_all = "camelCase")] and preserves unknown fields via a flattened extra map
    assert!(
        body.contains("rename_all = \"camelCase\""),
        "coverage.rs must use camelCase serde rename_all; got:\n{body}"
    );
    assert!(
        body.contains("serde(flatten") || body.contains("flatten"),
        "coverage.rs must preserve unknown fields via flatten; got:\n{body}"
    );

    // @step And commands/show_coverage.rs no longer returns FspecCoreError::NotYetPorted
    let show_cov_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/show_coverage.rs");
    let show_cov_src = fs::read_to_string(&show_cov_path).expect("show_coverage.rs readable");
    assert!(
        !show_cov_src.contains("NotYetPorted"),
        "show_coverage.rs must no longer return NotYetPorted; got:\n{show_cov_src}"
    );
}
