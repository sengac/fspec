//! BUG-608: fspec audit reports a coverage file whose stats disagree with
//! its scenarios, and --fix rewrites the stats.
//!
//! Feature: spec/features/coverage-stats-match-their-scenarios.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SIDECAR: &str = "spec/features/stats.feature.coverage";
const TEST_FILE: &str = "tests/stats_test.rs";

struct Project(tempfile::TempDir);

impl Project {
    /// A feature with `scenarios` one-step scenarios and a test file holding
    /// the @step comments of the first `tested` of them.
    fn new(scenarios: usize, tested: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        let mut feature = String::from("Feature: Stats\n");
        for n in 1..=scenarios {
            feature.push_str(&format!("\n  Scenario: Case {n}\n    Given step {n}\n"));
        }
        project.write("spec/features/stats.feature", &feature)?;
        let steps: String = (1..=tested)
            .map(|n| format!("// @step Given step {n}\n"))
            .collect();
        project.write(TEST_FILE, &steps)?;
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

    fn sidecar(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&std::fs::read_to_string(
            self.root().join(SIDECAR),
        )?)?)
    }

    /// A coverage file for `scenarios` scenarios whose first `tested` map
    /// their own @step line, with `stats` (or no stats block for `None`).
    fn coverage(&self, scenarios: usize, tested: usize, stats: Option<Value>) -> TestResult {
        let entries: Vec<Value> = (1..=scenarios)
            .map(|n| {
                let mappings = if n <= tested {
                    json!([{"file": TEST_FILE, "lines": format!("{n}-{n}"), "implMappings": []}])
                } else {
                    json!([])
                };
                json!({"name": format!("Case {n}"), "testMappings": mappings})
            })
            .collect();
        let mut coverage = json!({"scenarios": entries});
        if let Some(stats) = stats {
            coverage["stats"] = stats;
        }
        self.write(SIDECAR, &serde_json::to_string_pretty(&coverage)?)?;
        Ok(())
    }
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn succeed(root: &Path, command: &str, args: Value) -> TestResult {
    let result = dispatch(root, command, args);
    if !result.success {
        return Err(format!("{command} failed: {result:?}").into());
    }
    Ok(())
}

/// The output of `fspec audit --area coverage --feature stats`.
fn audit(root: &Path, fix: bool) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch(
        root,
        "audit",
        json!({"feature": ["stats"], "area": ["coverage"], "fix": fix}),
    );
    if !result.success {
        return Err(format!("fspec audit failed: {result:?}").into());
    }
    let envelope: Value = serde_json::from_str(&result.data)?;
    Ok(envelope["output"]
        .as_str()
        .ok_or("audit output is not text")?
        .to_string())
}

/// The audit's stats-mismatch line, if any.
fn stats_problem(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.starts_with("coverage stats-mismatch stats "))
        .map(str::to_string)
}

fn stats(total: u64, covered: u64, percent: u64, test_files: Value, lines: u64) -> Value {
    json!({
        "totalScenarios": total,
        "coveredScenarios": covered,
        "coveragePercent": percent,
        "testFiles": test_files,
        "implFiles": [],
        "totalLinesCovered": lines
    })
}

#[test]
fn stats_that_disagree_with_the_scenarios_are_reported_field_by_field() -> TestResult {
    // @step Given a coverage file with 14 scenarios, 11 of them with a test mapping
    let project = Project::new(14, 11)?;
    // @step And its stats say totalScenarios 8, coveredScenarios 0 and coveragePercent 0
    project.coverage(14, 11, Some(stats(8, 0, 0, json!([TEST_FILE]), 11)))?;

    // @step When fspec audit checks the coverage area
    let output = audit(project.root(), false)?;

    // @step Then it reports coverage stats-mismatch for the feature
    let problem = stats_problem(&output).ok_or(format!("no stats problem in:\n{output}"))?;
    // @step And the problem names totalScenarios 8 -> 14, coveredScenarios 0 -> 11 and coveragePercent 0 -> 79
    for field in [
        "totalScenarios 8 -> 14",
        "coveredScenarios 0 -> 11",
        "coveragePercent 0 -> 79",
    ] {
        assert!(problem.contains(field), "{field} missing in: {problem}");
    }
    assert!(!problem.contains("testFiles"), "{problem}");
    // @step And the problem's repair is fspec audit --fix
    assert!(
        problem.ends_with("-> fspec audit --fix --feature stats"),
        "{problem}"
    );
    Ok(())
}

#[test]
fn stats_that_list_a_test_file_no_mapping_uses_are_reported() -> TestResult {
    // @step Given a coverage file whose stats list a test file that none of its mappings name
    let project = Project::new(2, 2)?;
    let stored = json!([TEST_FILE, "tests/removed_test.rs"]);
    project.coverage(2, 2, Some(stats(2, 2, 100, stored, 2)))?;

    // @step When fspec audit checks the coverage area
    let output = audit(project.root(), false)?;

    // @step Then it reports coverage stats-mismatch naming testFiles with the stored and the recomputed list
    let problem = stats_problem(&output).ok_or(format!("no stats problem in:\n{output}"))?;
    let expected =
        format!(r#"testFiles ["{TEST_FILE}","tests/removed_test.rs"] -> ["{TEST_FILE}"]"#);
    assert!(
        problem.contains(&expected),
        "{expected} missing in: {problem}"
    );
    Ok(())
}

#[test]
fn a_coverage_file_without_a_stats_block_is_not_reported() -> TestResult {
    // @step Given a coverage file that has scenarios but no stats block
    let project = Project::new(2, 1)?;
    project.coverage(2, 1, None)?;

    // @step When fspec audit checks the coverage area
    let output = audit(project.root(), false)?;

    // @step Then it reports no coverage stats-mismatch
    assert_eq!(stats_problem(&output), None, "{output}");
    Ok(())
}

#[test]
fn the_repair_rewrites_only_the_stats_block() -> TestResult {
    // @step Given a coverage file whose stats disagree with its scenarios
    let project = Project::new(3, 2)?;
    project.coverage(3, 2, Some(stats(1, 0, 0, json!([]), 0)))?;
    let before = project.sidecar()?;

    // @step When fspec audit --fix runs for the feature
    audit(project.root(), true)?;

    // @step Then the stats block holds the recomputed values
    let after = project.sidecar()?;
    assert_eq!(after["stats"], stats(3, 2, 67, json!([TEST_FILE]), 2));
    // @step And the scenarios array is unchanged
    assert_eq!(after["scenarios"], before["scenarios"]);
    // @step And a second audit reports no stats-mismatch
    let output = audit(project.root(), false)?;
    assert_eq!(stats_problem(&output), None, "{output}");
    Ok(())
}

#[test]
fn coverage_files_fspec_writes_pass_the_check() -> TestResult {
    // @step Given a feature whose coverage file was written by add-scenario, generate-coverage, link-coverage and unlink-coverage in turn
    let project = Project::new(1, 1)?;
    project.write("src/stats.rs", "pub fn stats() -> u64 {\n    1\n}\n")?;
    let root = project.root();
    let feature = json!({"featureName": "stats"});
    let link = json!({"featureName": "stats", "scenario": "Case 1", "testFile": TEST_FILE,
                      "testLines": "1-1", "implFile": "src/stats.rs", "implLines": "1-3"});
    let unlink = json!({"featureName": "stats", "scenario": "Case 1", "testFile": TEST_FILE,
                        "implFile": "src/stats.rs"});
    let steps: [(&str, Value); 4] = [
        ("generate-coverage", feature),
        (
            "add-scenario",
            json!({"feature": "stats", "scenario": "Case 2"}),
        ),
        ("link-coverage", link),
        ("unlink-coverage", unlink),
    ];

    // @step When fspec audit checks the coverage area after each command
    let mut problems = Vec::new();
    for (command, args) in steps {
        succeed(root, command, args)?;
        if let Some(problem) = stats_problem(&audit(root, false)?) {
            problems.push(format!("after {command}: {problem}"));
        }
    }

    // @step Then it never reports coverage stats-mismatch
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    Ok(())
}

/// BUG-639: the audit's unreadable-coverage line, if any.
fn unreadable_problem(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.starts_with("coverage unreadable-coverage stats "))
        .map(str::to_string)
}

#[test]
fn stats_stored_as_counts_make_the_file_unreadable_and_the_repair_is_fspec_audit_fix() -> TestResult
{
    // @step Given a coverage file whose stats store testFiles and implFiles as counts
    let project = Project::new(2, 2)?;
    project.coverage(
        2,
        2,
        Some(
            json!({"totalScenarios": 2, "coveredScenarios": 2, "coveragePercent": 100,
                    "testFiles": 1, "implFiles": 2, "totalLinesCovered": 2}),
        ),
    )?;
    // @step When fspec audit checks the coverage area
    let output = audit(project.root(), false)?;
    // @step Then it reports coverage unreadable-coverage with the exact parse error
    let problem = unreadable_problem(&output).ok_or(output.clone())?;
    assert!(
        problem.contains(&format!(
            "{SIDECAR} does not match the coverage format: invalid type: integer `1`, expected a sequence at line "
        )),
        "{problem}"
    );
    // @step And the problem's repair is fspec audit --fix for the feature
    assert!(
        problem.ends_with("-> fspec audit --fix --feature stats"),
        "{problem}"
    );
    // @step And after fspec audit --fix the file reads and the audit reports no coverage problem
    audit(project.root(), true)?;
    let output = audit(project.root(), false)?;
    assert!(
        !output.lines().any(|line| line.starts_with("coverage ")),
        "{output}"
    );
    assert_eq!(project.sidecar()?["stats"]["testFiles"], json!([TEST_FILE]));
    Ok(())
}
