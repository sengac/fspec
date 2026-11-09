//! RES-010 — `fspec research` shows each tool's ✓/✗ status, resolved from
//! the environment, the user config file and the project config, with
//! `export` setup guidance for unconfigured tools.
//!
//! Feature: spec/features/tool-discovery-and-status-display.feature
//!
//! Both front doors (`fspec research` and the dispatcher) call the same
//! `research::run`, so the listing is checked through `dispatch_command`.
//! The research environment variables and `FSPEC_USER_DIR` are process
//! globals: every test holds `ENV_LOCK`, clears every research variable and
//! points `FSPEC_USER_DIR` at its own temporary user directory first, so no
//! test sees another test's settings or the developer's real environment.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Every environment variable the research config resolution reads.
const RESEARCH_VARS: [&str; 8] = [
    "PERPLEXITY_API_KEY",
    "PERPLEXITY_MODEL",
    "JIRA_URL",
    "JIRA_TOKEN",
    "CONFLUENCE_URL",
    "CONFLUENCE_TOKEN",
    "TEAMS_WEBHOOK_URL",
    "SLACK_WEBHOOK_URL",
];

/// One test's isolated environment: the lock, a project root and a user
/// directory that `FSPEC_USER_DIR` names. No research variable is set.
struct Isolated {
    _guard: MutexGuard<'static, ()>,
    project: tempfile::TempDir,
    user: tempfile::TempDir,
}

fn isolated() -> Result<Isolated, Box<dyn std::error::Error>> {
    let guard = ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    for var in RESEARCH_VARS {
        std::env::remove_var(var);
    }
    let project = tempfile::tempdir()?;
    let user = tempfile::tempdir()?;
    std::env::set_var("FSPEC_USER_DIR", user.path());
    Ok(Isolated {
        _guard: guard,
        project,
        user,
    })
}

fn write_config(path: &Path, config: &Value) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

/// `fspec research` (list mode) in `project`: the `tools` array.
fn list_tools(project: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "research".to_string(),
        args_json: json!({}).to_string(),
        project_root: project.to_path_buf(),
    });
    if !result.success {
        return Err(format!("research failed: {result:?}").into());
    }
    let data: Value = serde_json::from_str(&result.data)?;
    Ok(data["tools"].as_array().cloned().unwrap_or_default())
}

fn tool<'a>(tools: &'a [Value], name: &str) -> Result<&'a Value, String> {
    tools
        .iter()
        .find(|tool| tool["name"].as_str() == Some(name))
        .ok_or_else(|| format!("{name} is not listed: {tools:?}"))
}

#[test]
fn an_environment_variable_marks_the_tool_configured() -> TestResult {
    let env = isolated()?;
    // @step Given I have PERPLEXITY_API_KEY environment variable set
    std::env::set_var("PERPLEXITY_API_KEY", "pplx-from-env");

    // @step When I run "fspec research"
    let tools = list_tools(env.project.path())?;

    // @step Then the output should list Perplexity tool
    let perplexity = tool(&tools, "perplexity")?;
    // @step And the status indicator should be "✓"
    assert_eq!(perplexity["statusIndicator"], "✓", "{perplexity}");
    assert_eq!(perplexity["configured"], true, "{perplexity}");
    assert!(perplexity.get("configGuidance").is_none(), "{perplexity}");
    std::env::remove_var("PERPLEXITY_API_KEY");
    Ok(())
}

#[test]
fn an_unconfigured_tool_shows_export_setup_guidance() -> TestResult {
    let env = isolated()?;
    // @step Given I have no JIRA_URL or JIRA_TOKEN configured
    // (isolated() cleared both variables; no config file names jira)

    // @step When I run "fspec research"
    let tools = list_tools(env.project.path())?;

    // @step Then the output should list Jira tool
    let jira = tool(&tools, "jira")?;
    // @step And the status indicator should be "✗"
    assert_eq!(jira["statusIndicator"], "✗", "{jira}");
    assert_eq!(jira["configured"], false, "{jira}");
    // @step And the output should show setup guidance
    let guidance = jira["configGuidance"]
        .as_str()
        .ok_or_else(|| format!("jira has no setup guidance: {jira}"))?;
    let lines: Vec<&str> = guidance.lines().collect();
    // @step And the setup guidance should include the line export JIRA_URL="..."
    assert!(lines.contains(&"export JIRA_URL=\"...\""), "{guidance}");
    // @step And the setup guidance should include the line export JIRA_TOKEN="..."
    assert!(lines.contains(&"export JIRA_TOKEN=\"...\""), "{guidance}");
    Ok(())
}

#[test]
fn the_user_config_file_marks_the_tool_configured() -> TestResult {
    let env = isolated()?;
    // @step Given I have Confluence configuration in ~/.fspec/fspec-config.json
    write_config(
        &env.user.path().join("fspec-config.json"),
        &json!({"research": {"confluence": {
            "url": "https://wiki.example.test",
            "token": "user-token"
        }}}),
    )?;
    // Without the user file the same project lists Confluence unconfigured.
    assert!(!env.project.path().join("spec/fspec-config.json").exists());

    // @step When I run "fspec research"
    let tools = list_tools(env.project.path())?;

    // @step Then the output should list Confluence tool
    let confluence = tool(&tools, "confluence")?;
    // @step And the status indicator should be "✓"
    assert_eq!(confluence["statusIndicator"], "✓", "{confluence}");
    assert_eq!(confluence["configured"], true, "{confluence}");

    // The user layer is what configured it: an empty user directory leaves
    // Confluence unconfigured.
    std::fs::remove_file(env.user.path().join("fspec-config.json"))?;
    let tools = list_tools(env.project.path())?;
    assert_eq!(tool(&tools, "confluence")?["statusIndicator"], "✗");
    Ok(())
}

#[test]
fn every_tool_is_listed_with_a_description_and_a_status_indicator() -> TestResult {
    let env = isolated()?;
    // @step Given I have mixed tool configurations (some configured, some not)
    std::env::set_var("PERPLEXITY_API_KEY", "pplx-from-env");
    write_config(
        &env.user.path().join("fspec-config.json"),
        &json!({"research": {"confluence": {"url": "https://wiki.example.test", "token": "t"}}}),
    )?;
    write_config(
        &env.project.path().join("spec/fspec-config.json"),
        &json!({"research": {"stakeholder": {"teamsWebhook": "https://hook.example.test"}}}),
    )?;

    // @step When I run "fspec research"
    let tools = list_tools(env.project.path())?;
    std::env::remove_var("PERPLEXITY_API_KEY");

    // @step Then the output should list all registered research tools
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["confluence", "jira", "perplexity", "stakeholder"]);
    for entry in &tools {
        // @step And each tool should show a description
        let description = entry["description"].as_str().unwrap_or_default();
        assert!(!description.is_empty(), "{entry}");
        // @step And each tool should show a status indicator (✓ or ✗)
        let configured = entry["configured"] == true;
        let expected = if configured { "✓" } else { "✗" };
        assert_eq!(entry["statusIndicator"], expected, "{entry}");
        // @step And only unconfigured tools should show setup guidance
        assert_eq!(
            entry.get("configGuidance").is_some(),
            !configured,
            "{entry}"
        );
    }
    let configured: Vec<&str> = tools
        .iter()
        .filter(|t| t["configured"] == true)
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(configured, ["perplexity", "confluence", "stakeholder"]);
    Ok(())
}
