//! AUDIT-004 — audit sessions walk the judgment problems step by step.
//!
//! Feature: spec/features/audit-sessions-walk-the-judgment-problems-step-by-step.feature

use std::path::{Path, PathBuf};

use codelet_fspec_core::io::gherkin::parse_feature_lenient;
use codelet_fspec_core::io::gherkin_format::format_feature;
use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TAGS: &str = r#"{"categories":[
  {"name":"Component Tags","description":"","required":true,"tags":[{"name":"@comp","description":"x"}]},
  {"name":"Feature Group Tags","description":"","required":true,"tags":[{"name":"@grp","description":"x"}]},
  {"name":"Technical Tags","description":"","required":false,"tags":[{"name":"@AUTH-001","description":"x"}]}
]}"#;

const STEPS: [&str; 3] = [
    "    // @step Given a user",
    "    // @step When they log in",
    "    // @step Then they see the board",
];

/// The canonical form of a feature source.
fn canonical(body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let feature = parse_feature_lenient(body).map_err(|error| format!("{error:?}"))?;
    Ok(format_feature(&feature, body))
}

/// A feature named `title` with one scenario "Log in" (the login steps).
fn one_scenario(tags: &str, title: &str) -> String {
    format!("{tags}\nFeature: {title}\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n")
}

/// A temporary project; every audit session it started is reset on drop.
struct Project {
    dir: tempfile::TempDir,
    scopes: std::cell::RefCell<Vec<Value>>,
}

impl Drop for Project {
    fn drop(&mut self) {
        for scope in self.scopes.borrow().iter() {
            let mut args = scope.clone();
            args["reset"] = json!(true);
            let _ = self.run("audit", &args);
        }
        let _ = self.run("reverse", &json!({"reset": true}));
    }
}

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project {
            dir: tempfile::tempdir()?,
            scopes: std::cell::RefCell::new(Vec::new()),
        };
        project.write("spec/tags.json", TAGS)?;
        project.write(
            "spec/work-units.json",
            &json!({"version": "0.7.1", "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
                "workUnits": {"AUTH-001": {"id": "AUTH-001", "title": "Login", "type": "story",
                    "status": "implementing", "createdAt": "2026-11-01T00:00:00.000Z",
                    "updatedAt": "2026-11-01T00:00:00.000Z"}},
                "states": {"backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-001"],
                    "validating": [], "done": [], "blocked": []}})
            .to_string(),
        )?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, rel: &str, body: &str) -> std::io::Result<()> {
        let path = self.root().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    }

    fn run(&self, command: &str, args: &Value) -> DispatchResult {
        dispatch_command(DispatchRequest {
            command: command.to_string(),
            args_json: args.to_string(),
            project_root: self.root().to_path_buf(),
        })
    }

    /// `fspec audit` with `args`: (exit code, output, problems).
    fn audit(&self, args: Value) -> Result<(u64, String, Vec<Value>), Box<dyn std::error::Error>> {
        if args.get("start").is_some() {
            let mut scope = args.clone();
            if let Some(object) = scope.as_object_mut() {
                object.remove("start");
            }
            self.scopes.borrow_mut().push(scope);
        }
        let result = self.run("audit", &args);
        if !result.success {
            return Err(format!("audit failed: {result:?}").into());
        }
        let envelope: Value = serde_json::from_str(&result.data)?;
        Ok((
            envelope["exitCode"].as_u64().unwrap_or(99),
            envelope["output"].as_str().unwrap_or_default().to_string(),
            envelope["problems"].as_array().cloned().unwrap_or_default(),
        ))
    }

    /// Feature `name` (one scenario "Log in", canonical) mapping
    /// `test_file` at `lines`, with an implementation mapping of
    /// src/<name>.rs:1-3.
    fn feature(&self, name: &str, tags: &str, test_file: &str, lines: &str) -> TestResult {
        let title = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
        self.write(
            &format!("spec/features/{name}.feature"),
            &canonical(&one_scenario(tags, &title))?,
        )?;
        self.write(
            &format!("src/{name}.rs"),
            &format!("pub fn {name}() {{\n    run();\n}}\n"),
        )?;
        self.write(
            &format!("spec/features/{name}.feature.coverage"),
            &json!({"scenarios": [{"name": "Log in", "testMappings": [
                {"file": test_file, "lines": lines, "implMappings": [
                    {"file": format!("src/{name}.rs"), "lines": [1, 2, 3],
                     "firstLine": format!("pub fn {name}() {{"), "lastLine": "}"}]}
            ]}]})
            .to_string(),
        )?;
        Ok(())
    }

    /// `test_file` with the login @step block at lines 30-32 and 40-42 of
    /// a 45-line file.
    fn twice(&self, test_file: &str) -> std::io::Result<()> {
        let mut text: Vec<String> = (1..=45).map(|n| format!("// line {n}")).collect();
        for start in [30, 40] {
            for (offset, step) in STEPS.iter().enumerate() {
                text[start - 1 + offset] = step.to_string();
            }
        }
        self.write(test_file, &format!("{}\n", text.join("\n")))
    }

    /// Feature `name` whose "Log in" mapping of `test_file`:10-20 drifted
    /// and whose block occurs twice in the file.
    fn drifted_twice(&self, name: &str, test_file: &str) -> TestResult {
        self.twice(test_file)?;
        self.feature(name, "@comp @grp", test_file, "10-20")
    }
}

#[test]
fn a_problem_that_needs_judgment_names_the_audit_session_as_its_command() -> TestResult {
    // @step Given the @step block of "Log in" occurs twice in tests/login_test.rs and its mapping of tests/login_test.rs:10-20 drifted
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;

    // @step When fspec audit --feature login runs
    let (exit, output, _) = project.audit(json!({"feature": ["login"]}))?;
    let (_, _, problems) = project.audit(json!({"feature": ["login"], "json": true}))?;

    // @step Then the "coverage drifted login" line ends in "-> fspec audit --start --feature login"
    assert_eq!(exit, 1, "{output}");
    assert!(
        output.lines().any(|line| line
            .starts_with("coverage drifted login \"Log in\" tests/login_test.rs:10-20: ")
            && line.ends_with(" -> fspec audit --start --feature login")),
        "{output}"
    );

    // @step And the problem's fix kind in the JSON report is "guided"
    let drifted = problems
        .iter()
        .find(|problem| problem["code"] == "drifted")
        .ok_or("no drifted problem")?;
    assert_eq!(drifted["fix"]["kind"], "guided", "{drifted}");
    assert_eq!(
        drifted["fix"]["command"],
        "fspec audit --start --feature login"
    );
    Ok(())
}

#[test]
fn starting_a_session_over_a_twice_found_block_creates_one_step_naming_both_candidates(
) -> TestResult {
    // @step Given the @step block of "Log in" occurs twice in tests/login_test.rs and its mapping of tests/login_test.rs:10-20 drifted
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;

    // @step When fspec audit --start --feature login runs
    let (exit, output, _) = project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step Then a session with 1 step is created
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("1 step(s)"), "{output}");
    assert!(output.contains("Step 1 of 1"), "{output}");

    // @step And step 1 names both candidate ranges and the link-coverage command with --replace
    assert!(output.contains("tests/login_test.rs:30-32"), "{output}");
    assert!(output.contains("tests/login_test.rs:40-42"), "{output}");
    assert!(
        output.contains("fspec link-coverage login --scenario \"Log in\" --test-file tests/login_test.rs --test-lines <lines> --replace"),
        "{output}"
    );
    Ok(())
}

#[test]
fn continuing_while_the_problem_persists_stays_on_the_step() -> TestResult {
    // @step Given an audit session for feature "login" on step 1 of 1 whose mapping still drifts
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;
    project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step When fspec audit --continue --feature login runs
    let (exit, output, _) = project.audit(json!({"continue": true, "feature": ["login"]}))?;

    // @step Then it says the problem persists and exits 1
    assert_eq!(exit, 1, "{output}");
    assert!(output.contains("persists"), "{output}");

    // @step And fspec audit --status --feature login still shows step 1 of 1
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["login"]}))?;
    assert!(status.contains("Step 1 of 1"), "{status}");
    Ok(())
}

#[test]
fn continuing_after_the_repair_advances_and_completing_deletes_the_session() -> TestResult {
    // @step Given an audit session for feature "login" on step 1 of 1
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;
    project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step And the mapping was relinked to the first candidate range
    let linked = project.run(
        "link-coverage",
        &json!({"featureName": "login", "scenario": "Log in", "testFile": "tests/login_test.rs",
                "testLines": "30-32", "replace": true}),
    );
    assert!(linked.success, "{linked:?}");

    // @step When fspec audit --continue --feature login runs
    let (exit, output, _) = project.audit(json!({"continue": true, "feature": ["login"]}))?;
    assert_eq!(exit, 0, "{output}");

    // @step Then the session has every step done
    assert!(output.contains("Every step is done"), "{output}");

    // @step And fspec audit --complete --feature login deletes the session
    let (exit, output, _) = project.audit(json!({"complete": true, "feature": ["login"]}))?;
    assert_eq!(exit, 0, "{output}");
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["login"]}))?;
    assert!(status.contains("No audit session"), "{status}");
    Ok(())
}

#[test]
fn completing_before_every_step_is_done_is_refused() -> TestResult {
    // @step Given an audit session for feature "login" on step 1 of 1 whose mapping still drifts
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;
    project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step When fspec audit --complete --feature login runs
    let result = project.run("audit", &json!({"complete": true, "feature": ["login"]}));

    // @step Then it fails saying not every step is done
    assert!(!result.success, "{result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("not every step is done"),
        "{result:?}"
    );

    // @step And the session still exists
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["login"]}))?;
    assert!(status.contains("Step 1 of 1"), "{status}");
    Ok(())
}

#[test]
fn resetting_deletes_the_session() -> TestResult {
    // @step Given an audit session for feature "login" on step 1 of 1
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;
    project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step When fspec audit --reset --feature login runs
    let (exit, output, _) = project.audit(json!({"reset": true, "feature": ["login"]}))?;
    assert_eq!(exit, 0, "{output}");

    // @step Then the session no longer exists
    let continued = project.run("audit", &json!({"continue": true, "feature": ["login"]}));
    assert!(!continued.success, "{continued:?}");

    // @step And fspec audit --status --feature login says there is no audit session for the scope
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["login"]}))?;
    assert!(
        status.contains("No audit session for --feature login"),
        "{status}"
    );
    Ok(())
}

#[test]
fn a_scope_without_guided_problems_creates_no_session() -> TestResult {
    // @step Given feature "login" audits clean
    let project = Project::new()?;
    project.write("tests/login_test.rs", &format!("{}\n", STEPS.join("\n")))?;
    project.feature("login", "@comp @grp", "tests/login_test.rs", "1-3")?;
    assert_eq!(project.audit(json!({"feature": ["login"]}))?.0, 0);

    // @step When fspec audit --start --feature login runs
    let (exit, output, _) = project.audit(json!({"start": true, "feature": ["login"]}))?;

    // @step Then it says there is nothing to walk and no session is created
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("nothing to walk"), "{output}");
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["login"]}))?;
    assert!(status.contains("No audit session"), "{status}");
    Ok(())
}

/// AUTH-001's feature "login" with two scenarios, both mapped, and the
/// configured test command "cargo test".
fn meaning_project() -> Result<Project, Box<dyn std::error::Error>> {
    let project = Project::new()?;
    project.write(
        "spec/features/login.feature",
        &canonical("@AUTH-001 @comp @grp\nFeature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n\n  Scenario: Log out\n    Given a user\n    When they log out\n    Then they see the login page\n")?,
    )?;
    project.write(
        "tests/login_test.rs",
        &format!(
            "{}\n    // @step Given a user\n    // @step When they log out\n    // @step Then they see the login page\n",
            STEPS.join("\n")
        ),
    )?;
    project.write(
        "src/login.rs",
        "pub fn log_in() {\n    run();\n}\npub fn log_out() {\n    stop();\n}\n",
    )?;
    project.write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [
            {"name": "Log in", "testMappings": [{"file": "tests/login_test.rs", "lines": "1-3", "implMappings": [
                {"file": "src/login.rs", "lines": [1, 2, 3], "firstLine": "pub fn log_in() {", "lastLine": "}"}]}]},
            {"name": "Log out", "testMappings": [{"file": "tests/login_test.rs", "lines": "4-6", "implMappings": [
                {"file": "src/login.rs", "lines": [4, 5, 6], "firstLine": "pub fn log_out() {", "lastLine": "}"}]}]}
        ]})
        .to_string(),
    )?;
    Ok(project)
}

#[test]
fn the_meaning_area_adds_one_review_step_per_scenario_with_its_mappings_and_the_test_command(
) -> TestResult {
    // @step Given work unit AUTH-001 tags feature "login" whose two live scenarios map tests and implementation lines
    let project = meaning_project()?;

    // @step And spec/fspec-config.json configures tools.test.command "cargo test"
    project.write(
        "spec/fspec-config.json",
        r#"{"tools": {"test": {"command": "cargo test"}}}"#,
    )?;

    // @step When fspec audit --start --area meaning --work-unit AUTH-001 runs
    let (exit, output, _) =
        project.audit(json!({"start": true, "area": ["meaning"], "workUnitId": "AUTH-001"}))?;

    // @step Then a session with 2 review steps is created
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("2 step(s)"), "{output}");
    assert!(
        output.contains("Step 1 of 2: meaning login \"Log in\""),
        "{output}"
    );

    // @step And step 1 prints the scenario's test mapping, its implementation mappings and "cargo test"
    assert!(output.contains("tests/login_test.rs:1-3"), "{output}");
    assert!(output.contains("src/login.rs:1-3"), "{output}");
    assert!(output.contains("cargo test"), "{output}");

    // @step And step 1 tells the agent to break the implementation lines in a scratch git worktree, run the test, confirm it fails for the missing behaviour and remove the worktree
    for needle in [
        "git worktree add",
        "remove or stub",
        "fails because the behaviour is missing",
        "not because the build broke",
        "git worktree remove",
    ] {
        assert!(output.contains(needle), "{needle}: {output}");
    }
    Ok(())
}

#[test]
fn continuing_a_review_step_records_the_attestation() -> TestResult {
    // @step Given an audit session over the meaning area of AUTH-001 on step 1 of 2
    let project = meaning_project()?;
    project.write(
        "spec/fspec-config.json",
        r#"{"tools": {"test": {"command": "cargo test"}}}"#,
    )?;
    let scope = json!({"area": ["meaning"], "workUnitId": "AUTH-001"});
    let mut start = scope.clone();
    start["start"] = json!(true);
    project.audit(start)?;

    // @step When fspec audit --continue --area meaning --work-unit AUTH-001 runs
    let mut next = scope.clone();
    next["continue"] = json!(true);
    let (exit, output, _) = project.audit(next)?;
    assert_eq!(exit, 0, "{output}");

    // @step Then the session is on step 2 of 2
    assert!(output.contains("Step 2 of 2"), "{output}");

    // @step And step 1 records when it was attested
    let mut status = scope;
    status["status"] = json!(true);
    let (_, status, _) = project.audit(status)?;
    let first = status
        .lines()
        .find(|line| line.contains("1. meaning login \"Log in\""))
        .ok_or(format!("no step 1 line: {status}"))?;
    assert!(first.contains("attested 20"), "{status}");
    Ok(())
}

#[test]
fn the_spec_area_adds_one_stale_text_review_step_per_feature() -> TestResult {
    // @step Given features "login" and "admin" audit clean
    let project = Project::new()?;
    project.write("tests/login_test.rs", &format!("{}\n", STEPS.join("\n")))?;
    project.write("tests/admin_test.rs", &format!("{}\n", STEPS.join("\n")))?;
    project.feature("login", "@comp @grp", "tests/login_test.rs", "1-3")?;
    project.feature("admin", "@comp @grp", "tests/admin_test.rs", "1-3")?;
    assert_eq!(project.audit(json!({"feature": ["login", "admin"]}))?.0, 0);

    // @step When fspec audit --start --area spec --feature login --feature admin runs
    let (exit, output, _) =
        project.audit(json!({"start": true, "area": ["spec"], "feature": ["login", "admin"]}))?;

    // @step Then a session with 2 review steps is created, one per feature, naming the feature file and its implementation files
    assert_eq!(exit, 0, "{output}");
    assert!(output.contains("2 step(s)"), "{output}");
    let (_, status, _) =
        project.audit(json!({"status": true, "area": ["spec"], "feature": ["login", "admin"]}))?;
    assert!(status.contains("stale-text admin"), "{status}");
    assert!(status.contains("stale-text login"), "{status}");
    assert!(output.contains("spec/features/admin.feature"), "{output}");
    assert!(output.contains("src/admin.rs"), "{output}");
    Ok(())
}

#[test]
fn sessions_of_different_scopes_are_separate() -> TestResult {
    // @step Given features "login" and "admin" each have a twice-found drifted @step block
    let project = Project::new()?;
    project.drifted_twice("login", "tests/login_test.rs")?;
    project.drifted_twice("admin", "tests/admin_test.rs")?;

    // @step When fspec audit --start --feature login and fspec audit --start --feature admin run in one project
    let (login_exit, login, _) = project.audit(json!({"start": true, "feature": ["login"]}))?;
    let (admin_exit, admin, _) = project.audit(json!({"start": true, "feature": ["admin"]}))?;

    // @step Then each scope has its own session file
    assert_eq!((login_exit, admin_exit), (0, 0), "{login}\n{admin}");
    assert!(login.contains("coverage drifted login"), "{login}");
    assert!(admin.contains("coverage drifted admin"), "{admin}");

    // @step And fspec audit --status --feature admin shows the admin step
    let (_, status, _) = project.audit(json!({"status": true, "feature": ["admin"]}))?;
    assert!(status.contains("coverage drifted admin"), "{status}");
    assert!(!status.contains("coverage drifted login"), "{status}");
    Ok(())
}

#[test]
fn the_meaning_area_without_start_is_an_error() -> TestResult {
    let project = Project::new()?;
    // @step When fspec audit --area meaning --feature login runs
    let result = project.run("audit", &json!({"area": ["meaning"], "feature": ["login"]}));

    // @step Then it fails saying the meaning area is accepted only with --start
    assert!(!result.success, "{result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("the meaning area is accepted only with --start"),
        "{result:?}"
    );
    Ok(())
}

#[test]
fn the_validating_reminder_offers_the_meaning_session() -> TestResult {
    // @step Given work unit AUTH-001 moves from implementing to validating
    let project = meaning_project()?;
    let result = project.run(
        "update-work-unit-status",
        &json!({"workUnitId": "AUTH-001", "status": "validating", "skipTemporalValidation": true}),
    );

    // @step When the move succeeds
    assert!(result.success, "{result:?}");

    // @step Then its reminder offers "fspec audit --start --area meaning --work-unit AUTH-001"
    assert!(
        result
            .data
            .contains("fspec audit --start --area meaning --work-unit AUTH-001"),
        "{}",
        result.data
    );
    Ok(())
}

#[test]
fn fspec_reverse_keeps_its_session_file_on_the_shared_session_module() -> TestResult {
    // @step Given a project with no reverse session
    let project = Project::new()?;
    project.write("Cargo.toml", "[package]\nname = \"x\"\n")?;
    let canonical_root = std::fs::canonicalize(project.root())?;
    let digest = Sha256::digest(canonical_root.to_string_lossy().as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let expected: PathBuf = std::env::temp_dir().join(format!("fspec-reverse-{}.json", &hex[..12]));
    let _ = std::fs::remove_file(&expected);

    // @step When fspec reverse starts a session
    let result = project.run("reverse", &json!({}));
    assert!(result.success, "{result:?}");

    // @step Then the session file is fspec-reverse-<first 12 hex of sha256 of the canonical project root>.json in the temp directory
    assert!(expected.exists(), "{}", expected.display());
    assert_eq!(
        codelet_fspec_core::types::reverse_session::session_path(project.root()),
        expected
    );
    Ok(())
}
