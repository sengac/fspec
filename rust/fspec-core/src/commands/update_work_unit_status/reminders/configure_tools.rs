//! configure-tools checks printed on entering validating.

use std::path::Path;

use serde_json::Value;

use crate::commands::review::{format_agent_output, get_agent_config};

/// Current UTC year as a string (parity with `new Date().getFullYear()`).
fn current_year() -> String {
    crate::io::time::iso8601_now()
        .get(0..4)
        .unwrap_or("1970")
        .to_string()
}

/// Mirrors `checkTestCommand`: agent-formatted reminder describing the test
/// command (or its absence).
pub(crate) fn check_test_command(project_root: &Path) -> String {
    let agent = get_agent_config(project_root);
    let config_path = crate::io::spec_paths::config(project_root).map_err(std::io::Error::other);
    let year = current_year();

    let full = format!(
        r#"NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools {year}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)

Example:
  fspec configure-tools --test-command "npm test"
  fspec configure-tools --test-command "pytest"
  fspec configure-tools --test-command "cargo test"
"#
    );
    let short = format!(
        r#"NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools {year}"

Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)
"#
    );

    let content = match config_path.and_then(std::fs::read_to_string) {
        Err(_) => full,
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(cfg) => match cfg
                .get("tools")
                .and_then(|t| t.get("test"))
                .and_then(|t| t.get("command"))
                .and_then(Value::as_str)
            {
                Some(cmd) if !cmd.is_empty() => {
                    format!("RUN TESTS\n\nRun tests: {cmd}\n")
                }
                _ => short,
            },
            Err(_) => short,
        },
    };
    format_agent_output(&agent, &content)
}

/// Mirrors `checkQualityCommands`: agent-formatted reminder describing the
/// quality-check commands (or their absence).
pub(crate) fn check_quality_commands(project_root: &Path) -> String {
    let agent = get_agent_config(project_root);
    let config_path = crate::io::spec_paths::config(project_root).map_err(std::io::Error::other);

    let content = match config_path.and_then(std::fs::read_to_string) {
        Err(_) => "No quality check commands configured".to_string(),
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(cfg) => match cfg
                .get("tools")
                .and_then(|t| t.get("qualityCheck"))
                .and_then(|q| q.get("commands"))
                .and_then(Value::as_array)
            {
                Some(cmds) => {
                    let chained = cmds
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" && ");
                    format!("RUN QUALITY CHECKS\n\nRun quality checks: {chained}\n")
                }
                None => "No quality check commands configured".to_string(),
            },
            Err(_) => "No quality check commands configured".to_string(),
        },
    };
    format_agent_output(&agent, &content)
}
