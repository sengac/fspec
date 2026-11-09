//! BUG-507 — update-feature-text edits non-step feature text.
//!
//! Feature: spec/features/update-feature-text-edits-non-step-feature-text.feature

use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "\
Feature: Login
  \"\"\"
  Users sign in with a password.
  \"\"\"

  # RULES:
  #   1. Passwords are checked by the old service

  Scenario Outline: Log in
    Given a user <name>
    When they open the board
    Then they see the board

    Examples:
      | name  |
      | alice |
";

fn project(body: &str) -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("spec/features/login.feature");
    std::fs::create_dir_all(root.path().join("spec/features"))?;
    std::fs::write(&path, body)?;
    Ok((root, path))
}

fn replace(root: &Path, find: &str, with: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-feature-text".to_string(),
        args_json: json!({"feature": "login", "find": find, "replace": with}).to_string(),
        project_root: root.to_path_buf(),
    })
}

#[test]
fn description_comment_and_table_text_are_replaced() -> TestResult {
    // @step Given a feature with a description, a comment rule and an Examples table
    let (root, path) = project(FEATURE)?;
    // @step When I replace a description sentence, a comment rule and an Examples cell with update-feature-text
    for (find, with) in [
        ("sign in with a password", "sign in with a passkey"),
        ("checked by the old service", "checked by the auth service"),
        ("| alice |", "| bob   |"),
    ] {
        let result = replace(root.path(), find, with);
        assert!(result.success, "{find}: {result:?}");
    }
    // @step Then each text is replaced
    let after = std::fs::read_to_string(&path)?;
    assert!(after.contains("Users sign in with a passkey."), "{after}");
    assert!(after.contains("checked by the auth service"), "{after}");
    assert!(
        after.contains("| bob   |") && !after.contains("alice"),
        "{after}"
    );
    // @step And the scenarios and steps are unchanged
    let steps = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| {
                let line = line.trim_start();
                ["Scenario", "Given", "When", "Then"]
                    .iter()
                    .any(|keyword| line.starts_with(keyword))
            })
            .map(str::to_string)
            .collect()
    };
    assert_eq!(steps(&after), steps(FEATURE));
    Ok(())
}

#[test]
fn text_that_occurs_more_than_once_is_refused() -> TestResult {
    // @step Given a feature in which the text "the board" occurs twice
    let (root, path) = project(FEATURE)?;
    // @step When I replace "the board" with update-feature-text
    let result = replace(root.path(), "the board", "the dashboard");
    // @step Then update-feature-text fails naming 2 matches
    assert!(!result.success, "{result:?}");
    assert!(format!("{result:?}").contains("2 matches"), "{result:?}");
    // @step And the file is unchanged
    assert_eq!(std::fs::read_to_string(&path)?, FEATURE);
    Ok(())
}

#[test]
fn text_that_is_not_in_the_feature_is_refused() -> TestResult {
    // @step Given a feature without the text "nowhere"
    let (root, path) = project(FEATURE)?;
    // @step When I replace "nowhere" with update-feature-text
    let result = replace(root.path(), "nowhere", "somewhere");
    // @step Then update-feature-text fails saying the text was not found
    assert!(!result.success, "{result:?}");
    assert!(format!("{result:?}").contains("not found"), "{result:?}");
    assert_eq!(std::fs::read_to_string(&path)?, FEATURE);
    Ok(())
}

#[test]
fn step_lines_and_scenario_headers_are_refused() -> TestResult {
    // @step Given a feature with the step "Given a user" in scenario "Log in"
    let (root, path) = project(FEATURE)?;
    // @step When I replace "Given a user" and then "Scenario: Log in" with update-feature-text
    let step = replace(root.path(), "Given a user", "Given an admin");
    let header = replace(
        root.path(),
        "Scenario Outline: Log in",
        "Scenario Outline: Sign in",
    );
    // @step Then both replacements fail pointing at update-step and update-scenario
    assert!(!step.success, "{step:?}");
    assert!(format!("{step:?}").contains("update-step"), "{step:?}");
    assert!(!header.success, "{header:?}");
    assert!(
        format!("{header:?}").contains("update-scenario"),
        "{header:?}"
    );
    // @step And the file is unchanged
    assert_eq!(std::fs::read_to_string(&path)?, FEATURE);
    Ok(())
}

#[test]
fn a_replacement_that_breaks_the_gherkin_is_refused() -> TestResult {
    // @step Given a feature titled "Login"
    let (root, path) = project(FEATURE)?;
    // @step When I replace "Feature: Login" with text that drops the Feature keyword
    let result = replace(root.path(), "Feature: Login", "Login");
    // @step Then update-feature-text fails saying the result is not valid Gherkin
    assert!(!result.success, "{result:?}");
    assert!(
        format!("{result:?}").contains("not valid Gherkin"),
        "{result:?}"
    );
    // @step And the file is unchanged
    assert_eq!(std::fs::read_to_string(&path)?, FEATURE);
    Ok(())
}

#[test]
fn the_listed_stale_feature_text_is_fixed() -> TestResult {
    // @step Given the stale texts listed on BUG-507
    let stale = [
        (
            "rpc022-slash-command-wiring.feature",
            "| /role                             | ClearRole",
        ),
        ("rpc024-multi-session-store.feature", "pub fn cycle_session"),
        (
            "delete-epic-cli-subcommand.feature",
            "prefixed with 'Error:'",
        ),
        (
            "optimized-compaction-trigger.feature",
            "disable_autocompact flag is set",
        ),
        (
            "clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature",
            "force_inject_fallback_dag",
        ),
        (
            "agentview-esc-exit-confirmation-dialog.feature",
            "renders three flat buttons",
        ),
        (
            "codex-tool-input-streaming.feature",
            "Only read_file, list_dir and grep_files may dispatch",
        ),
        (
            "per-model-context-window-and-max-output-configuration.feature",
            "session_set_model gains optional context_window",
        ),
        (
            "provider-settings-list-mode-keybinds-shape.feature",
            "remove Rust-only keybinds (r/R, wrap-around, PageUp/PageDown/Home/End)",
        ),
    ];
    let features = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("repository root")?
        .join("spec/features");
    // @step When the repository's feature files are read
    let mut remaining = Vec::new();
    for (file, text) in stale {
        let content = std::fs::read_to_string(features.join(file))?;
        // @step Then none of those stale texts remains
        if content.contains(text) {
            remaining.push(format!("{file}: {text}"));
        }
    }
    assert!(remaining.is_empty(), "stale text remains: {remaining:#?}");
    Ok(())
}
