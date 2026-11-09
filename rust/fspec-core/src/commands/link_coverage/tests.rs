#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn args_parse_full() {
    let a: LinkCoverageArgs = serde_json::from_str(
        r#"{"featureName":"user-login","scenario":"Login","testFile":"t.ts","testLines":"1-5","implFile":"i.ts","implLines":"10-12","skipValidation":true,"skipStepValidation":false}"#,
    )
    .unwrap();
    assert_eq!(a.feature_name.as_deref(), Some("user-login"));
    assert_eq!(a.scenario.as_deref(), Some("Login"));
    assert_eq!(a.test_lines.as_deref(), Some("1-5"));
    assert!(a.skip_validation);
    assert!(!a.skip_step_validation);
}

#[test]
fn parse_impl_lines_ranges_and_lists() {
    assert_eq!(parse_impl_lines("10-12"), vec![10, 11, 12]);
    assert_eq!(parse_impl_lines("10,11,12"), vec![10, 11, 12]);
    assert_eq!(parse_impl_lines("89-90,316-317"), vec![89, 90, 316, 317]);
}

#[test]
fn extract_work_unit_id_finds_tag() {
    assert_eq!(
        extract_work_unit_id("@AUTH-001\nFeature: X"),
        Some("AUTH-001".to_string())
    );
    assert_eq!(extract_work_unit_id("@wip\nFeature: X"), None);
}

#[test]
fn update_stats_initializes_missing_stats() {
    let mut cov = json!({
        "scenarios": [ { "name": "Login", "testMappings": [
            { "file": "t.ts", "lines": "1-10", "implMappings": [ { "file": "i.ts", "lines": [1,2,3] } ] }
        ] } ]
    });
    update_stats(&mut cov);
    let stats = cov.get("stats").unwrap();
    assert_eq!(stats.get("coveredScenarios").unwrap(), 1);
    assert_eq!(stats.get("coveragePercent").unwrap(), 100);
    assert_eq!(stats.get("totalLinesCovered").unwrap(), 13);
}

#[test]
fn add_test_mapping_appends_and_labels() {
    let mut entry = json!({ "name": "Login", "testMappings": [] });
    let msg = add_test_mapping(&mut entry, "t.ts", "45-62");
    assert!(msg.contains("Linked test mapping"));
    let msg2 = add_test_mapping(&mut entry, "t.ts", "70-80");
    assert!(msg2.contains("Added second test mapping"));
}

// Feature: spec/features/link-coverage-crashes-when-coverage-file-stats-are-missing.feature
// BUG-091: a coverage file without a `stats` object links cleanly and gains
// a valid stats object.
#[test]
fn dispatch_links_a_coverage_file_without_a_stats_object() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    let write = |rel: &str, body: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    // @step Given I have a feature file "test-missing-stats.feature"
    write(
        "spec/features/test-missing-stats.feature",
        "Feature: Test Missing Stats\n\n  Scenario: Test Scenario\n    Given a precondition\n    When an action happens\n    Then an outcome follows\n",
    );
    // @step And I have a coverage file "test-missing-stats.feature.coverage" without a stats object
    write(
        "spec/features/test-missing-stats.feature.coverage",
        r#"{ "scenarios": [ { "name": "Test Scenario", "testMappings": [] } ] }"#,
    );
    // @step And I have a test file "test-missing-stats.test.ts"
    write(
        "test-missing-stats.test.ts",
        "// @step Given a precondition\n// @step When an action happens\n// @step Then an outcome follows\ntest('t', () => {\n  // 4\n  // 5\n  // 6\n  // 7\n  // 8\n});\n",
    );
    // @step When I run "fspec link-coverage test-missing-stats.feature --scenario 'Test Scenario' --test-file test-missing-stats.test.ts --test-lines 1-10"
    let result = crate::dispatch_command(crate::DispatchRequest {
        command: "link-coverage".to_string(),
        args_json: json!({
            "featureName": "test-missing-stats.feature",
            "scenario": "Test Scenario",
            "testFile": "test-missing-stats.test.ts",
            "testLines": "1-10"
        })
        .to_string(),
        project_root: root.to_path_buf(),
    });
    // @step Then the exit code should be 0
    assert!(result.success, "link-coverage failed: {:?}", result.error);
    // @step And the output should contain "Linked test mapping"
    let data: Value = serde_json::from_str(&result.data).unwrap();
    let message = data["message"].as_str().unwrap();
    assert!(
        message.contains("Linked test mapping"),
        "message: {message}"
    );
    // @step And the coverage file should contain a valid stats object
    let raw =
        std::fs::read_to_string(root.join("spec/features/test-missing-stats.feature.coverage"))
            .unwrap();
    let cov: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        cov["stats"],
        json!({
            "totalScenarios": 1,
            "coveredScenarios": 1,
            "coveragePercent": 100,
            "testFiles": ["test-missing-stats.test.ts"],
            "implFiles": [],
            "totalLinesCovered": 10
        })
    );
}
