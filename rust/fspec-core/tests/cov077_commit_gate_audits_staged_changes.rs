//! COV-077 — the commit gate runs fspec audit on the staged changes.
//!
//! Feature: spec/features/commit-gate-runs-fspec-audit-on-the-staged-changes.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LOGIN: &str =
    "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

const LOGIN_WITH_LOG_OUT: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n\n  Scenario: Log out\n    Given a user\n    When they log out\n    Then they see the login page\n";

/// The "Log in" @step comments on lines 1-3, then two lines of code.
const TEST_FILE: &str = "// @step Given a user\n// @step When they log in\n// @step Then they see the board\nlet user = user();\nlog_in(&user);\n";

/// The same test with its @step comments moved to lines 5-7.
const MOVED_TEST_FILE: &str = "let user = user();\nlog_in(&user);\nassert!(true);\nassert!(true);\n// @step Given a user\n// @step When they log in\n// @step Then they see the board\n";

const GATE_LINE: &str = "fspec audit --changed || exit 1";

struct Project(tempfile::TempDir);

impl Project {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project(tempfile::tempdir()?);
        project.git(&["init", "-q"])?;
        Ok(project)
    }

    /// A committed project where "Log in" maps tests/login_test.rs:1-3.
    fn with_login() -> Result<Self, Box<dyn std::error::Error>> {
        let project = Project::new()?;
        project.write("spec/features/login.feature", LOGIN)?;
        project.write(
            "spec/features/login.feature.coverage",
            &json!({"scenarios": [{"name": "Log in", "testMappings": [
                {"file": "tests/login_test.rs", "lines": "1-3", "implMappings": []}
            ]}]})
            .to_string(),
        )?;
        project.write("tests/login_test.rs", TEST_FILE)?;
        project.write("README.md", "# Login\n")?;
        project.git(&["add", "-A"])?;
        project.git(&["commit", "-q", "-m", "initial"])?;
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

    fn read(&self, rel: &str) -> std::io::Result<String> {
        std::fs::read_to_string(self.root().join(rel))
    }

    fn git(&self, args: &[&str]) -> TestResult {
        let status = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .current_dir(self.root())
            .status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
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

/// `fspec audit --changed` with `extra` arguments: (exit code, output).
fn audit_changed(root: &Path, extra: Value) -> Result<(Value, String), Box<dyn std::error::Error>> {
    let mut args = json!({"changed": true});
    if let (Some(args), Some(extra)) = (args.as_object_mut(), extra.as_object()) {
        args.extend(extra.clone());
    }
    let result = dispatch(root, "audit", args);
    if !result.success {
        return Err(format!("fspec audit --changed failed: {result:?}").into());
    }
    let envelope: Value = serde_json::from_str(&result.data)?;
    Ok((
        envelope["exitCode"].clone(),
        envelope["output"].as_str().unwrap_or_default().to_string(),
    ))
}

fn coverage_only() -> Value {
    json!({"area": ["coverage"]})
}

/// `fspec init --agent claude`: the result envelope.
fn init(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let result = dispatch(root, "init", json!({"agent": ["claude"]}));
    if !result.success {
        return Err(format!("fspec init failed: {result:?}").into());
    }
    Ok(serde_json::from_str(&result.data)?)
}

#[test]
fn a_commit_that_stages_no_spec_test_or_implementation_file_passes() -> TestResult {
    // @step Given a git repository whose only staged file is README.md
    let project = Project::with_login()?;
    project.write("README.md", "# Login, changed\n")?;
    project.git(&["add", "README.md"])?;
    // @step When fspec audit --changed --json runs
    let (exit, output) = audit_changed(project.root(), json!({"json": true}))?;
    // @step Then it prints {"problems":[],"exitCode":0}
    assert_eq!(output, "{\"problems\":[],\"exitCode\":0}");
    assert_eq!(exit, 0);
    Ok(())
}

#[test]
fn a_staged_edit_that_moves_a_scenarios_steps_out_of_its_mapped_lines_is_reported() -> TestResult {
    // @step Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3
    let project = Project::with_login()?;
    // @step And an edit that moves its @step comments to lines 5-7 is staged
    project.write("tests/login_test.rs", MOVED_TEST_FILE)?;
    project.git(&["add", "tests/login_test.rs"])?;
    // @step When fspec audit --changed runs
    let (exit, output) = audit_changed(project.root(), coverage_only())?;
    // @step Then it reports "coverage drifted login" for tests/login_test.rs:1-3
    assert!(
        output
            .lines()
            .any(|line| line
                .starts_with("coverage drifted login \"Log in\" tests/login_test.rs:1-3: ")),
        "{output}"
    );
    // @step And it exits 1
    assert_eq!(exit, 1, "{output}");
    Ok(())
}

#[test]
fn an_unstaged_edit_neither_blocks_nor_hides_a_problem() -> TestResult {
    // @step Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3 and the staged test file is correct
    let project = Project::with_login()?;
    project.write(
        "tests/login_test.rs",
        &format!("{TEST_FILE}assert!(true);\n"),
    )?;
    project.git(&["add", "tests/login_test.rs"])?;
    // @step And an unstaged edit breaks the test file
    project.write("tests/login_test.rs", MOVED_TEST_FILE)?;
    // @step When fspec audit --changed runs
    let (exit, output) = audit_changed(project.root(), coverage_only())?;
    // @step Then it reports nothing for login
    assert!(!output.contains("login"), "{output}");
    assert!(
        output.contains("No problems found in 1 feature(s)"),
        "{output}"
    );
    // @step And it exits 0
    assert_eq!(exit, 0, "{output}");
    Ok(())
}

#[test]
fn a_staged_scenario_without_a_coverage_entry_is_reported() -> TestResult {
    // @step Given a git repository where spec/features/login.feature is staged with a new scenario "Log out" that has no coverage entry
    let project = Project::with_login()?;
    project.write("spec/features/login.feature", LOGIN_WITH_LOG_OUT)?;
    project.git(&["add", "spec/features/login.feature"])?;
    // @step When fspec audit --changed runs
    let (exit, output) = audit_changed(project.root(), coverage_only())?;
    // @step Then it reports "coverage missing-entry login" naming "Log out"
    assert!(
        output
            .lines()
            .any(|line| line.starts_with("coverage missing-entry login \"Log out\" ")),
        "{output}"
    );
    // @step And it exits 1
    assert_eq!(exit, 1, "{output}");
    Ok(())
}

#[test]
fn a_test_file_staged_as_deleted_is_missing() -> TestResult {
    // @step Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3
    let project = Project::with_login()?;
    // @step And the deletion of tests/login_test.rs is staged while the file is still on disk
    project.git(&["rm", "-q", "--cached", "tests/login_test.rs"])?;
    assert!(project.root().join("tests/login_test.rs").exists());
    // @step When fspec audit --changed runs
    let (exit, output) = audit_changed(project.root(), coverage_only())?;
    // @step Then it reports "coverage missing-test-file login" for tests/login_test.rs:1-3
    assert!(
        output.lines().any(|line| line
            .starts_with("coverage missing-test-file login \"Log in\" tests/login_test.rs:1-3: ")),
        "{output}"
    );
    assert_eq!(exit, 1, "{output}");
    Ok(())
}

#[test]
fn fspec_init_installs_the_pre_commit_hook() -> TestResult {
    // @step Given a git repository without a pre-commit hook
    let project = Project::new()?;
    assert!(!project.root().join(".git/hooks/pre-commit").exists());
    // @step When fspec init runs
    init(project.root())?;
    // @step Then .git/hooks/pre-commit is an executable hook that runs only fspec audit --changed and exits with its exit code
    let hook = project.read(".git/hooks/pre-commit")?;
    let commands: Vec<&str> = hook
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();
    assert_eq!(commands, vec!["exec fspec audit --changed"], "{hook}");
    assert!(hook.starts_with("#!/bin/sh\n"), "{hook}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(project.root().join(".git/hooks/pre-commit"))?
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "{mode:o}");
    }
    // @step And the hook marks itself as written by fspec
    assert!(
        hook.lines()
            .any(|line| line.starts_with("# fspec commit gate: written by fspec ")),
        "{hook}"
    );
    Ok(())
}

#[test]
fn a_pre_commit_hook_fspec_did_not_write_is_left_alone() -> TestResult {
    // @step Given a git repository whose .git/hooks/pre-commit was written by the user
    let project = Project::new()?;
    let user_hook = "#!/bin/sh\nmake lint\n";
    project.write(".git/hooks/pre-commit", user_hook)?;
    // @step When fspec init runs
    let result = init(project.root())?;
    // @step Then the hook is unchanged
    assert_eq!(project.read(".git/hooks/pre-commit")?, user_hook);
    // @step And fspec prints that the commit gate is not installed and the line "fspec audit --changed || exit 1" to add to the hook
    let message = result["commitGate"].as_str().unwrap_or_default();
    assert!(message.contains("commit gate is not installed"), "{result}");
    assert!(message.contains(GATE_LINE), "{result}");
    Ok(())
}

#[test]
fn fspec_sync_version_rewrites_the_hook_fspec_installed_earlier() -> TestResult {
    // @step Given a git repository whose pre-commit hook was written by an earlier fspec version
    let project = Project::new()?;
    init(project.root())?;
    let current = project.read(".git/hooks/pre-commit")?;
    let earlier = "#!/bin/sh\n# fspec commit gate: written by fspec 0.0.1; fspec init and fspec --sync-version keep it current.\nexec fspec audit --changed --json\n";
    project.write(".git/hooks/pre-commit", earlier)?;
    // @step When fspec --sync-version runs
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    runtime.block_on(codelet_fspec_core::commands::sync_version::run(
        &json!({"embeddedVersion": "0.0.1"}).to_string(),
        project.root(),
    ))?;
    // @step Then the hook is rewritten with the current fspec hook
    assert_eq!(project.read(".git/hooks/pre-commit")?, current);
    Ok(())
}
