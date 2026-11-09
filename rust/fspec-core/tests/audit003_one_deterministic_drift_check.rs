//! AUDIT-003 — fspec audit is the one deterministic drift check.
//!
//! Feature: spec/features/fspec-audit-is-the-one-deterministic-drift-check.feature

use std::path::Path;

use codelet_fspec_core::io::gherkin::parse_feature_lenient;
use codelet_fspec_core::io::gherkin_format::format_feature;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TAGS: &str = r#"{"categories":[
  {"name":"Component Tags","description":"","required":true,"tags":[{"name":"@comp","description":"x"}]},
  {"name":"Feature Group Tags","description":"","required":true,"tags":[{"name":"@grp","description":"x"}]},
  {"name":"Technical Tags","description":"","required":false,"tags":[]}
]}"#;

const LOGIN: &str = "@comp @grp\nFeature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const STEPS: [&str; 3] = [
    "    // @step Given a user",
    "    // @step When they log in",
    "    // @step Then they see the board",
];

/// The formatter's canonical form of a feature source.
fn canonical(body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let feature = parse_feature_lenient(body).map_err(|error| format!("{error:?}"))?;
    Ok(format_feature(&feature, body))
}

/// A temporary project with registered tags.
struct Project(tempfile::TempDir);

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.write("spec/tags.json", TAGS)?;
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

    /// Feature `name` (canonical form of `body`) whose scenario "Log in"
    /// maps `test_file` at `lines`.
    fn feature(
        &self,
        name: &str,
        body: &str,
        test_file: &str,
        lines: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write(&format!("spec/features/{name}.feature"), &canonical(body)?)?;
        self.write(
            &format!("spec/features/{name}.feature.coverage"),
            &json!({"scenarios": [{"name": "Log in", "testMappings": [
                {"file": test_file, "lines": lines, "implMappings": []}
            ]}]})
            .to_string(),
        )?;
        Ok(())
    }

    /// The clean login feature, its test on lines 1-3 of tests/login_test.rs.
    fn clean_login(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.write("tests/login_test.rs", &format!("{}\n", STEPS.join("\n")))?;
        self.feature("login", LOGIN, "tests/login_test.rs", "1-3")
    }
}

/// The result of `fspec audit` with `args`.
struct Audit {
    exit_code: Value,
    output: String,
    problems: Vec<Value>,
}

fn audit(root: &Path, args: Value) -> Result<Audit, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "audit".to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    });
    if !result.success {
        return Err(format!("audit failed: {result:?}").into());
    }
    let envelope: Value = serde_json::from_str(&result.data)?;
    Ok(Audit {
        exit_code: envelope["exitCode"].clone(),
        output: envelope["output"].as_str().unwrap_or_default().to_string(),
        problems: envelope["problems"].as_array().cloned().unwrap_or_default(),
    })
}

#[test]
fn a_drifted_test_mapping_is_reported_with_the_command_that_relinks_it() -> TestResult {
    // @step Given scenario "Log in" maps tests/login_test.rs:10-20 but its @step comments sit on lines 14-24
    let project = Project::new()?;
    let mut lines: Vec<String> = (1..=24).map(|n| format!("// filler {n}")).collect();
    lines[13] = STEPS[0].to_string();
    lines[18] = STEPS[1].to_string();
    lines[23] = STEPS[2].to_string();
    project.write("tests/login_test.rs", &format!("{}\n", lines.join("\n")))?;
    project.feature("login", LOGIN, "tests/login_test.rs", "10-20")?;
    // @step When fspec audit runs
    let result = audit(project.root(), json!({}))?;
    // @step Then it prints a "coverage drifted login" line for tests/login_test.rs:10-20 ending in the fspec audit --fix command
    let expected = "coverage drifted login \"Log in\" tests/login_test.rs:10-20: no @step comment for 'Then they see the board' in the mapped lines -> fspec audit --fix --feature login";
    assert!(
        result.output.lines().any(|line| line == expected),
        "{}",
        result.output
    );
    // @step And it exits 1
    assert_eq!(result.exit_code, 1, "{}", result.output);
    Ok(())
}

#[test]
fn a_coverage_file_without_its_feature_is_reported() -> TestResult {
    // @step Given spec/features/old.feature.coverage exists without old.feature
    let project = Project::new()?;
    project.clean_login()?;
    project.write("spec/features/old.feature.coverage", r#"{"scenarios":[]}"#)?;
    // @step When fspec audit runs
    let result = audit(project.root(), json!({}))?;
    // @step Then it prints a "spec orphan-coverage old" line
    assert!(
        result.output.lines().any(|line| line
            .starts_with("spec orphan-coverage old spec/features/old.feature.coverage: ")),
        "{}",
        result.output
    );
    // @step And it exits 1
    assert_eq!(result.exit_code, 1, "{}", result.output);
    Ok(())
}

#[test]
fn a_feature_with_no_scenarios_is_reported() -> TestResult {
    // @step Given a feature "empty" with no scenarios
    let project = Project::new()?;
    project.clean_login()?;
    project.write(
        "spec/features/empty.feature",
        &canonical("@comp @grp\nFeature: Empty\n")?,
    )?;
    // @step When fspec audit runs
    let result = audit(project.root(), json!({}))?;
    // @step Then it prints a "spec no-scenarios empty" line
    assert!(
        result
            .output
            .lines()
            .any(|line| line.starts_with("spec no-scenarios empty: ")),
        "{}",
        result.output
    );
    assert_eq!(result.exit_code, 1, "{}", result.output);
    Ok(())
}

#[test]
fn step_text_inside_a_string_literal_does_not_cover_a_step() -> TestResult {
    // @step Given a test file whose only "@step Then they see the board" text is inside a string literal
    let project = Project::new()?;
    project.write(
        "tests/login_test.rs",
        &format!(
            "fn log_in() {{\n{}\n{}\n    let text = \"// @step Then they see the board\";\n}}\n",
            STEPS[0], STEPS[1]
        ),
    )?;
    project.feature("login", LOGIN, "tests/login_test.rs", "1-5")?;
    // @step When fspec audit runs
    let result = audit(project.root(), json!({}))?;
    // @step Then it reports the step "Then they see the board" as missing from the mapped lines
    assert!(
        result.output.lines().any(|line| line.starts_with(
            "coverage drifted login \"Log in\" tests/login_test.rs:1-5: no @step comment for 'Then they see the board' in the mapped lines"
        )),
        "{}",
        result.output
    );
    assert_eq!(result.exit_code, 1, "{}", result.output);
    Ok(())
}

#[test]
fn the_area_option_limits_the_areas_checked() -> TestResult {
    // @step Given a project whose only problem is a feature file that needs formatting
    let project = Project::new()?;
    project.clean_login()?;
    project.write(
        "spec/features/login.feature",
        "@comp @grp\nFeature: Login\n\n\n  Scenario: Log in\n        Given a user\n    When they log in\n    Then they see the board\n",
    )?;
    let full = audit(project.root(), json!({}))?;
    let codes: Vec<String> = full
        .problems
        .iter()
        .map(|problem| format!("{} {}", problem["area"], problem["code"]))
        .collect();
    assert_eq!(
        codes,
        ["\"format\" \"needs-formatting\""],
        "{}",
        full.output
    );
    // @step When fspec audit runs with --area tags
    let result = audit(project.root(), json!({"area": ["tags"]}))?;
    // @step Then it prints no problems
    assert!(result.problems.is_empty(), "{}", result.output);
    // @step And it exits 0
    assert_eq!(result.exit_code, 0, "{}", result.output);
    Ok(())
}

#[test]
fn a_clean_project_prints_an_empty_problem_list_as_json() -> TestResult {
    // @step Given a project with no problems
    let project = Project::new()?;
    project.clean_login()?;
    // @step When fspec audit runs with --json
    let result = audit(project.root(), json!({"json": true}))?;
    // @step Then it prints {"problems":[],"exitCode":0}
    assert_eq!(result.output, r#"{"problems":[],"exitCode":0}"#);
    assert_eq!(result.exit_code, 0);
    Ok(())
}

#[test]
fn the_feature_option_limits_the_scope() -> TestResult {
    // @step Given features "login" and "other" both have problems
    let project = Project::new()?;
    project.feature("login", LOGIN, "tests/gone_test.rs", "1-3")?;
    project.write(
        "spec/features/other.feature",
        &canonical("@comp @grp\nFeature: Other\n")?,
    )?;
    let full = audit(project.root(), json!({}))?;
    assert!(
        full.problems.iter().any(|p| p["feature"] == "other"),
        "{}",
        full.output
    );
    // @step When fspec audit runs with --feature login
    let result = audit(project.root(), json!({"feature": ["login"]}))?;
    // @step Then it reports only problems of "login"
    assert!(!result.problems.is_empty(), "{}", result.output);
    assert!(
        result.problems.iter().all(|p| p["feature"] == "login"),
        "{}",
        result.output
    );
    Ok(())
}

#[test]
fn a_test_file_mapped_by_two_features_is_reported_for_each() -> TestResult {
    // @step Given features "login" and "admin" both map tests/shared_test.rs
    let project = Project::new()?;
    project.write("tests/shared_test.rs", &format!("{}\n", STEPS.join("\n")))?;
    project.feature("login", LOGIN, "tests/shared_test.rs", "1-3")?;
    project.feature(
        "admin",
        &LOGIN.replace("Feature: Login", "Feature: Admin"),
        "tests/shared_test.rs",
        "1-3",
    )?;
    // @step When fspec audit runs
    let result = audit(project.root(), json!({}))?;
    // @step Then it prints a "coverage shared-test-file" line for each feature naming the other
    for (feature, other) in [("login", "admin"), ("admin", "login")] {
        let prefix = format!(
            "coverage shared-test-file {feature} tests/shared_test.rs: also mapped by spec/features/{other}.feature -> "
        );
        assert!(
            result.output.lines().any(|line| line.starts_with(&prefix)),
            "{prefix}\n{}",
            result.output
        );
    }
    assert_eq!(result.exit_code, 1, "{}", result.output);
    Ok(())
}
