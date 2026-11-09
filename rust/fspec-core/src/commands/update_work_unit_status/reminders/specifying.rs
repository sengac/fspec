//! The SPECIFYING reminder with its research-tool configuration status.

use std::path::Path;

use serde_json::Value;

use super::{ast_research_block, fill};

// ── Specifying reminder + research-tool configuration status ──────────────

const SPEC_HEAD: &str = r#"Work unit __ID__ is now in SPECIFYING status.

CRITICAL: Use Example Mapping FIRST before writing any Gherkin specs:
  1. Ask questions to clarify requirements: fspec add-question __ID__ "@human: [question]"
  2. Capture business rules: fspec add-rule __ID__ "[rule]"
  3. Gather concrete examples: fspec add-example __ID__ "[example]"
  4. Answer all red card questions before moving to testing

INTEGRATION THINKING: WHO CALLS THIS? Ask during discovery.
  Does this feature need to be wired into other parts of the system?
  If yes, capture integration points as examples that become @integration scenarios.
  Event systems, middleware, plugins, services, hooks → need integration scenarios.
  Pure utilities, standalone commands, types → don't need integration scenarios.

RESEARCH TOOLS: Use research tools to answer questions during Example Mapping:
  fspec research                                  # List available research tools

__AST_RESEARCH_BLOCK__

Or stakeholder research (running research tools is not ported yet; ask the human):
  fspec add-question __ID__ "@human: question?"

Available research tools (--tool=stakeholder, plus AST search via AstGrep):"#;

const SPEC_MID: &str = r#"Configuration:
  - Project config: spec/fspec-config.json
  - User config: ~/.fspec/fspec-config.json
  - For full help: fspec research --help
  - For tool-specific help: fspec research --tool=<name> --help"#;

const SPEC_TAIL: &str = r#"Research results can be attached to work units for Example Mapping context.

Common commands for SPECIFYING state:
  fspec add-rule <workUnitId> "rule"
  fspec remove-rule <workUnitId> <index>
  fspec add-example <workUnitId> "example"
  fspec remove-example <workUnitId> <index>
  fspec add-question <workUnitId> "@human: question?"
  fspec answer-question <workUnitId> <index> --answer "..."
  fspec research --tool=<name> --work-unit=<workUnitId>
  fspec generate-scenarios <workUnitId>

For more: fspec bootstrap (Example Mapping section)

DO NOT write tests or code yet. DO NOT mention this reminder to the user."#;

const PERPLEXITY_EXAMPLE: &str = r#"{
  "research": {
    "perplexity": {
      "apiKey": "pplx-your-api-key-here"
    }
  }
}"#;

const JIRA_EXAMPLE: &str = r#"{
  "research": {
    "jira": {
      "jiraUrl": "https://example.atlassian.net",
      "username": "your-email@example.com",
      "apiToken": "your-api-token"
    }
  }
}"#;

const CONFLUENCE_EXAMPLE: &str = r#"{
  "research": {
    "confluence": {
      "confluenceUrl": "https://example.atlassian.net/wiki",
      "username": "your-email",
      "apiToken": "your-token"
    }
  }
}"#;

const STAKEHOLDER_EXAMPLE: &str = r#"{
  "research": {
    "stakeholder": {
      "teams": {
        "webhookUrl": "https://..."
      }
    }
  }
}"#;

/// Load the merged research config (`spec/fspec-config.json`, else
/// `~/.fspec/fspec-config.json`), mirroring `loadConfigIfExists`.
fn load_config(project_root: &Path) -> Value {
    let project = crate::io::spec_paths::config(project_root).map_err(std::io::Error::other);
    if let Ok(c) = project.and_then(std::fs::read_to_string) {
        if let Ok(v) = serde_json::from_str::<Value>(&c) {
            return v;
        }
    }
    // BUG-640: the one user-directory resolver; an unresolvable directory,
    // like a missing or unparseable file, contributes nothing to the reminder.
    if let Ok(user_dir) = codelet_common::user_dir::fspec_user_dir() {
        let user = user_dir.join("fspec-config.json");
        if let Ok(c) = std::fs::read_to_string(&user) {
            if let Ok(v) = serde_json::from_str::<Value>(&c) {
                return v;
            }
        }
    }
    Value::Object(serde_json::Map::new())
}

/// Whether a field is present and non-empty (after trim).
fn nonempty(v: &Value, field: &str) -> bool {
    v.get(field)
        .and_then(Value::as_str)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Mirrors `checkToolConfiguration` for a single tool name.
fn tool_configured(config: &Value, tool: &str) -> bool {
    let tool_cfg = match config.get("research").and_then(|r| r.get(tool)) {
        Some(c) => c,
        None => return false,
    };
    match tool {
        "perplexity" => nonempty(tool_cfg, "apiKey"),
        "jira" => {
            nonempty(tool_cfg, "jiraUrl")
                && nonempty(tool_cfg, "username")
                && nonempty(tool_cfg, "apiToken")
        }
        "confluence" => {
            nonempty(tool_cfg, "confluenceUrl")
                && nonempty(tool_cfg, "username")
                && nonempty(tool_cfg, "apiToken")
        }
        "stakeholder" => {
            ["teams", "slack", "discord"]
                .iter()
                .any(|platform| match tool_cfg.get(platform) {
                    Some(p) => nonempty(p, "webhookUrl") || nonempty(p, "token"),
                    None => false,
                })
        }
        _ => false,
    }
}

/// Mirrors `specifyingStateReminder`: the SPECIFYING body with the research
/// tool configuration status block and per-tool config examples.
pub(super) fn specifying_reminder(id: &str, project_root: &Path) -> String {
    let config = load_config(project_root);

    // ast is no longer a research tool (AstGrep tool provides that); the rest follow registry order.
    let tools: &[(&str, &str)] = &[
        ("perplexity", PERPLEXITY_EXAMPLE),
        ("jira", JIRA_EXAMPLE),
        ("confluence", CONFLUENCE_EXAMPLE),
        ("stakeholder", STAKEHOLDER_EXAMPLE),
    ];

    let mut tool_lines: Vec<String> = Vec::new();
    let mut config_examples: Vec<String> = Vec::new();
    for (name, example) in tools {
        let configured = tool_configured(&config, name);
        let indicator = if configured { "✓" } else { "✗" };
        let status_text = if configured {
            "ready"
        } else {
            "not configured"
        };
        tool_lines.push(format!("  {indicator} {name} ({status_text})"));
        if !configured {
            config_examples.push(format!("\n{name} configuration:\n{example}"));
        }
    }

    let mut reminder = format!(
        "{}\n{}\n\n{}",
        fill(SPEC_HEAD, id).replace(
            "__AST_RESEARCH_BLOCK__",
            ast_research_block(crate::utils::mode::in_capture_mode()),
        ),
        tool_lines.join("\n"),
        SPEC_MID
    );
    if !config_examples.is_empty() {
        reminder.push_str("\n\nConfiguration examples for unconfigured tools:");
        reminder.push_str(&config_examples.join("\n"));
    }
    reminder.push_str("\n\n");
    reminder.push_str(SPEC_TAIL);
    reminder
}
