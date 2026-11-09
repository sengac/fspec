//! AUDIT-005 — workflow gates call the audit core.
//!
//! Feature: spec/features/workflow-gates-call-the-audit-core.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN: &str = "@AUTH-001\nFeature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const STEPS: [&str; 3] = [
    "    // @step Given a user",
    "    // @step When they log in",
    "    // @step Then they see the board",
];

const IMPL: &str = "pub fn log_in() {\n    board();\n}\n";

/// A temporary project holding work unit AUTH-001.
struct Project(tempfile::TempDir);

impl Project {
    /// AUTH-001 (a story) at `status`, with `linked` features if any.
    fn new(status: &str, linked: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        let mut states = serde_json::Map::new();
        for state in [
            "backlog",
            "specifying",
            "testing",
            "implementing",
            "validating",
            "done",
            "blocked",
        ] {
            let ids: Vec<&str> = if state == status {
                vec!["AUTH-001"]
            } else {
                vec![]
            };
            states.insert(state.to_string(), json!(ids));
        }
        let mut unit = json!({
            "id": "AUTH-001", "title": "Login", "type": "story", "status": status,
            "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
        });
        if !linked.is_empty() {
            unit["linkedFeatures"] = json!(linked);
        }
        project.write(
            "spec/work-units.json",
            &json!({
                "version": "0.7.1",
                "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
                "workUnits": {"AUTH-001": unit},
                "states": states,
            })
            .to_string(),
        )?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn write(&self, rel: &str, body: &str) -> std::io::Result<()> {
        let path = self.root().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }

    /// tests/login_test.rs with the three @step comments on `lines` of a
    /// file of `length` lines.
    fn test_file(&self, lines: [usize; 3], length: usize) -> std::io::Result<()> {
        let mut text: Vec<String> = (1..=length).map(|n| format!("// line {n}")).collect();
        for (step, line) in STEPS.iter().zip(lines) {
            text[line - 1] = step.to_string();
        }
        self.write("tests/login_test.rs", &format!("{}\n", text.join("\n")))
    }

    /// The login feature and its coverage: `scenarios` is the coverage
    /// scenarios array.
    fn login(&self, scenarios: Value) -> std::io::Result<()> {
        self.write("spec/features/login.feature", LOGIN)?;
        self.write("src/login.rs", IMPL)?;
        self.write(
            "spec/features/login.feature.coverage",
            &json!({ "scenarios": scenarios }).to_string(),
        )
    }

    /// AUTH-001 moved to `status`.
    fn move_to(&self, status: &str) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: "update-work-unit-status".to_string(),
            args_json: json!({
                "workUnitId": "AUTH-001", "status": status, "skipTemporalValidation": true
            })
            .to_string(),
            project_root: self.root().to_path_buf(),
        })
    }

    fn status(&self) -> Result<String, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(self.root().join("spec/work-units.json"))?;
        let data: Value = serde_json::from_str(&raw)?;
        Ok(data["workUnits"]["AUTH-001"]["status"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// The problem lines `fspec audit` prints for `args`.
    fn audit_lines(&self, args: Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let result = dispatch_command(DispatchRequest {
            command: "audit".to_string(),
            args_json: args.to_string(),
            project_root: self.root().to_path_buf(),
        });
        if !result.success {
            return Err(format!("audit failed: {result:?}").into());
        }
        let envelope: Value = serde_json::from_str(&result.data)?;
        Ok(problem_lines(
            envelope["output"].as_str().unwrap_or_default(),
        ))
    }
}

/// The `<area> <code> <feature> ...` lines of a text.
fn problem_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.starts_with("coverage ") || line.starts_with("spec "))
        .map(str::to_string)
        .collect()
}

/// The error of a failed move.
fn failure(result: &DispatchResult) -> Result<String, Box<dyn std::error::Error>> {
    if result.success {
        return Err(format!("the move succeeded: {result:?}").into());
    }
    Ok(result.error.clone().unwrap_or_default())
}

/// A test mapping of tests/login_test.rs at `lines`, with the recorded
/// implementation mapping of src/login.rs:1-3 when `implemented`.
fn mapping(lines: &str, implemented: bool) -> Value {
    let impl_mappings = if implemented {
        json!([{"file": "src/login.rs", "lines": [1, 2, 3],
                "firstLine": "pub fn log_in() {", "lastLine": "}"}])
    } else {
        json!([])
    };
    json!({"file": "tests/login_test.rs", "lines": lines, "implMappings": impl_mappings})
}

#[test]
fn a_drifted_test_mapping_blocks_the_move_to_validating_with_fspec_audits_problem_lines(
) -> TestResult {
    // @step Given AUTH-001 is implementing and its feature "login" maps scenario "Log in" to tests/login_test.rs:10-20
    let project = Project::new("implementing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("10-20", true)]}]))?;

    // @step And the @step comments of "Log in" sit on lines 30-40
    project.test_file([30, 35, 40], 40)?;

    // @step When AUTH-001 is moved to validating
    let result = project.move_to("validating");

    // @step Then the move fails with the "coverage drifted login" line and its fix command
    let error = failure(&result)?;
    let lines = problem_lines(&error);
    assert!(
        lines.iter().any(|line| line
            .starts_with("coverage drifted login \"Log in\" tests/login_test.rs:10-20: ")
            && line.ends_with(" -> fspec audit --fix --feature login")),
        "{error}"
    );

    // @step And the error holds exactly the problem lines that fspec audit --work-unit AUTH-001 --area coverage prints
    let audited = project.audit_lines(json!({"workUnitId": "AUTH-001", "area": ["coverage"]}))?;
    assert!(!audited.is_empty());
    assert_eq!(lines, audited, "{error}");

    // @step And AUTH-001 stays implementing
    assert_eq!(project.status()?, "implementing");
    Ok(())
}

#[test]
fn step_text_outside_the_mapped_lines_blocks_the_move_to_implementing() -> TestResult {
    // @step Given AUTH-001 is testing and its test file holds the steps of "Log in" only outside the mapped lines
    let project = Project::new("testing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("1-3", false)]}]))?;
    project.test_file([7, 8, 9], 9)?;

    // @step When AUTH-001 is moved to implementing
    let result = project.move_to("implementing");

    // @step Then the move fails with the "coverage drifted login" line
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line
                .starts_with("coverage drifted login \"Log in\" tests/login_test.rs:1-3: ")),
        "{error}"
    );
    assert_eq!(project.status()?, "testing");
    Ok(())
}

#[test]
fn an_uncovered_scenario_blocks_the_move_to_implementing() -> TestResult {
    // @step Given AUTH-001 is testing and its scenario "Log in" has no test mappings
    let project = Project::new("testing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": []}]))?;

    // @step When AUTH-001 is moved to implementing
    let result = project.move_to("implementing");

    // @step Then the move fails with the "coverage uncovered login" line for "Log in"
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line.starts_with("coverage uncovered login \"Log in\": ")),
        "{error}"
    );
    assert_eq!(project.status()?, "testing");
    Ok(())
}

#[test]
fn a_scenario_without_an_implementation_mapping_blocks_the_move_to_validating() -> TestResult {
    // @step Given AUTH-001 is implementing and its scenario "Log in" maps its exact @step block but no implementation lines
    let project = Project::new("implementing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("1-3", false)]}]))?;
    project.test_file([1, 2, 3], 3)?;
    assert!(project
        .audit_lines(json!({"workUnitId": "AUTH-001", "area": ["coverage"]}))?
        .is_empty());

    // @step When AUTH-001 is moved to validating
    let result = project.move_to("validating");

    // @step Then the move fails naming "Log in" as a scenario without an implementation mapping
    let error = failure(&result)?;
    assert!(
        problem_lines(&error).iter().any(|line| line
            .starts_with("coverage no-impl-mapping login \"Log in\" tests/login_test.rs:1-3: ")),
        "{error}"
    );
    assert_eq!(project.status()?, "implementing");
    Ok(())
}

#[test]
fn a_coverage_entry_for_a_deleted_scenario_blocks_the_move_to_done() -> TestResult {
    // @step Given AUTH-001 is validating and its coverage file keeps an entry for the deleted scenario "Log out"
    let project = Project::new("validating", &[])?;
    project.login(json!([
        {"name": "Log in", "testMappings": [mapping("1-3", true)]},
        {"name": "Log out", "testMappings": []}
    ]))?;
    project.test_file([1, 2, 3], 3)?;

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move fails with the "coverage stale-entry login" line for "Log out"
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line.starts_with("coverage stale-entry login \"Log out\" ")),
        "{error}"
    );
    assert_eq!(project.status()?, "validating");
    Ok(())
}

#[test]
fn a_feature_without_scenarios_blocks_the_move_to_done() -> TestResult {
    // @step Given AUTH-001 is validating and its linked feature "empty" has no scenarios
    let project = Project::new("validating", &["empty"])?;
    project.write("spec/features/empty.feature", "Feature: Empty\n")?;
    project.write(
        "spec/features/empty.feature.coverage",
        &json!({"scenarios": []}).to_string(),
    )?;

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move fails with the "spec no-scenarios empty" line
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line.starts_with("spec no-scenarios empty: ")),
        "{error}"
    );
    assert_eq!(project.status()?, "validating");
    Ok(())
}

#[test]
fn a_clean_audit_with_every_scenario_mapped_moves_the_card_to_done() -> TestResult {
    // @step Given AUTH-001 is validating and fspec audit finds no problem in its features
    let project = Project::new("validating", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("1-3", true)]}]))?;
    project.test_file([1, 2, 3], 3)?;
    assert!(project
        .audit_lines(json!({"workUnitId": "AUTH-001", "area": ["coverage", "spec"]}))?
        .is_empty());

    // @step And every scenario maps its exact @step block and implementation lines
    let coverage: Value = serde_json::from_str(&std::fs::read_to_string(
        project.root().join("spec/features/login.feature.coverage"),
    )?)?;
    assert_eq!(
        coverage["scenarios"][0]["testMappings"][0]["implMappings"][0]["file"],
        "src/login.rs"
    );

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move succeeds and AUTH-001 is done
    assert!(result.success, "{result:?}");
    assert_eq!(project.status()?, "done");
    Ok(())
}

#[test]
fn a_scenario_mapping_its_exact_step_block_moves_the_card_to_implementing() -> TestResult {
    // @step Given AUTH-001 is testing and its scenario "Log in" maps its exact @step block
    let project = Project::new("testing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("1-3", false)]}]))?;
    project.test_file([1, 2, 3], 3)?;

    // @step When AUTH-001 is moved to implementing
    let result = project.move_to("implementing");

    // @step Then the move succeeds and AUTH-001 is implementing
    assert!(result.success, "{result:?}");
    assert_eq!(project.status()?, "implementing");
    Ok(())
}

#[test]
fn a_test_mapping_without_the_then_step_blocks_the_move_to_validating_and_names_the_missing_step(
) -> TestResult {
    // @step Given AUTH-001 is implementing and the mapped lines of "Log in" hold the Given and When @step comments but not the Then one
    let project = Project::new("implementing", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": [mapping("1-3", true)]}]))?;
    project.write(
        "tests/login_test.rs",
        &format!("{}\n{}\n    // assert the board\n", STEPS[0], STEPS[1]),
    )?;

    // @step When AUTH-001 is moved to validating
    let result = project.move_to("validating");

    // @step Then the move fails with the "coverage drifted login" line naming the step 'Then they see the board'
    let error = failure(&result)?;
    assert!(
        problem_lines(&error).iter().any(|line| line
            .starts_with("coverage drifted login \"Log in\" tests/login_test.rs:1-3: ")
            && line.contains("no @step comment for 'Then they see the board' in the mapped lines")),
        "{error}"
    );

    // @step And AUTH-001 stays implementing
    assert_eq!(project.status()?, "implementing");
    Ok(())
}

#[test]
fn an_uncovered_scenario_of_a_feature_found_by_its_tag_blocks_the_move_to_done() -> TestResult {
    // @step Given AUTH-001 is validating, links no features, and the feature "login" tagged @AUTH-001 has the scenario "Log in" with no test mappings
    let project = Project::new("validating", &[])?;
    project.login(json!([{"name": "Log in", "testMappings": []}]))?;

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move fails with the "coverage uncovered login" line for "Log in"
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line.starts_with("coverage uncovered login \"Log in\": ")),
        "{error}"
    );

    // @step And AUTH-001 stays validating
    assert_eq!(project.status()?, "validating");
    Ok(())
}

#[test]
fn a_feature_without_a_coverage_file_blocks_the_move_to_done() -> TestResult {
    // @step Given AUTH-001 is validating and its feature "login" has no coverage file
    let project = Project::new("validating", &[])?;
    project.write("spec/features/login.feature", LOGIN)?;
    assert!(!project
        .root()
        .join("spec/features/login.feature.coverage")
        .exists());

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move fails with the "coverage uncovered login" line for "Log in"
    let error = failure(&result)?;
    assert!(
        problem_lines(&error)
            .iter()
            .any(|line| line.starts_with("coverage uncovered login \"Log in\": ")),
        "{error}"
    );

    // @step And AUTH-001 stays validating
    assert_eq!(project.status()?, "validating");
    Ok(())
}

#[test]
fn linked_features_replace_the_features_found_by_the_work_units_tag() -> TestResult {
    // @step Given AUTH-001 is validating and links only the feature "custom", whose scenario maps its exact @step block and implementation lines
    let project = Project::new("validating", &["custom"])?;
    project.write(
        "spec/features/custom.feature",
        &LOGIN.replace("@AUTH-001\nFeature: Login", "Feature: Custom"),
    )?;
    project.write(
        "spec/features/custom.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": [mapping("1-3", true)]}]})
            .to_string(),
    )?;
    project.write("src/login.rs", IMPL)?;
    project.test_file([1, 2, 3], 3)?;

    // @step And the feature "login" is tagged @AUTH-001 and its scenario "Log in" has no test mappings
    project.write("spec/features/login.feature", LOGIN)?;
    project.write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": []}]}).to_string(),
    )?;
    let tag_scope = project.audit_lines(json!({"feature": ["login"], "area": ["coverage"]}))?;
    assert!(
        tag_scope
            .iter()
            .any(|line| line.starts_with("coverage uncovered login \"Log in\": ")),
        "{tag_scope:?}"
    );

    // @step When AUTH-001 is moved to done
    let result = project.move_to("done");

    // @step Then the move succeeds and AUTH-001 is done, because "login" is not audited
    assert!(result.success, "{result:?}");
    assert_eq!(project.status()?, "done");
    Ok(())
}
