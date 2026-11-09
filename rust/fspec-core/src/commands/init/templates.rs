//! Slash-command and agent-documentation generation, including the
//! `<test-command>` / `<quality-check-commands>` config substitution.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::files::HomeEnv;
use super::registry::{
    Agent,
    Category::{Cli, Extension, Ide},
    SlashFormat::Toml,
};
use super::FSPEC_VERSION;
use crate::error::FspecCoreError;

/// Pre-rendered agent-documentation templates, one per distinct prose group.
///
/// The TypeScript `generateAgentDoc` builds the doc by concatenating 22
/// section generators (`src/utils/projectManagementSections/*.ts`), several of
/// which emit AGENT-CONDITIONAL prose *outside* `<system-reminder>` tags and
/// inline `formatSystemReminder` examples that are transformed with a DIFFERENT
/// regex than the doc-level `stripSystemReminders`. Reproducing that pipeline
/// in Rust is brittle, so — mirroring the RPC-200 bootstrap strategy — the
/// byte-exact output is captured once per *prose group* and embedded.
///
/// The 19 agents collapse to exactly 4 groups keyed by
/// `(supportsSystemReminders, category)`; within a group the ONLY variation is
/// the `{{AGENT_NAME}}` (×2) and `{{DOC_TEMPLATE}}` (×2) placeholders:
///   - `sysrem_cli` — claude, antigravity (reminders + meta preserved)
///   - `plain_ide` — cursor, windsurf, kilocode, roo (⚠️-emoji visible form)
///   - `plain_ext` — cline, copilot, amazonq (⚠️-emoji visible form)
///   - `plain_cli` — aider, gemini, qwen, codebuddy, auggie, opencode, codex, factory, crush, codex-cli (plain visible form)
const DOC_SYSREM_CLI: &str = include_str!("../init_doc_sysrem_cli.md");
const DOC_PLAIN_IDE: &str = include_str!("../init_doc_plain_ide.md");
const DOC_PLAIN_EXT: &str = include_str!("../init_doc_plain_ext.md");
const DOC_PLAIN_CLI: &str = include_str!("../init_doc_plain_cli.md");

/// The slash command file for an agent: its path, display path and content.
///
/// Codex / codex-cli write to `<home>/.codex/prompts/fspec.md`, the home
/// resolved from the injected platform and environment (BUG-533: USERPROFILE
/// on Windows, HOME elsewhere; never the project). All others write under the
/// project root at `agent.slash_command_path`.
pub(super) fn slash_command_file(
    project_root: &Path,
    agent: &Agent,
    home: &HomeEnv,
) -> Result<(PathBuf, String, String), FspecCoreError> {
    let is_codex = is_codex(agent);
    let commands_dir = if is_codex {
        home.require_home_dir()?.join(".codex").join("prompts")
    } else {
        project_root.join(agent.slash_command_path)
    };
    let filename = if agent.slash_command_format == Toml {
        "fspec.toml"
    } else {
        "fspec.md"
    };
    let display = if is_codex {
        "~/.codex/prompts/fspec.md".to_string()
    } else {
        format!("{}{}", agent.slash_command_path, filename)
    };
    Ok((
        commands_dir.join(filename),
        display,
        generate_slash_command_content(agent),
    ))
}

/// Codex and codex-cli keep their prompt in the home directory.
pub(super) fn is_codex(agent: &Agent) -> bool {
    agent.id == "codex" || agent.id == "codex-cli"
}

/// Generate slash command content. TOML agents get an inline literal; markdown
/// agents get the embedded header section (parity with getHeaderSection()).
fn generate_slash_command_content(agent: &Agent) -> String {
    if agent.slash_command_format == Toml {
        return format!(
            "[command]\nname = \"fspec - Load Project Context\"\n\
description = \"Load fspec workflow and ACDD methodology\"\n\n\
# fspec Command - Load Full Context\n\n\
Run these commands to load fspec context:\n\n\
1. fspec --sync-version {FSPEC_VERSION}\n2. fspec bootstrap\n\n\
Then read the comprehensive guide at spec/{} for full ACDD workflow.\n",
            agent.doc_template
        );
    }

    // Markdown header section — port of getHeaderSection().
    format!(
        "# fspec Command - Kanban-Based Project Management\n\n\
IMMEDIATELY - run this command (fspec is already installed locally, so just run it):\n\n\
fspec --sync-version {FSPEC_VERSION}\n\n\
IF the output from this command is an error, asking you to run the \
\"fspec configure-tools\" command, RUN THAT FIRST BY FOLLOWING THE INSTRUCTIONS YOU WERE GIVEN.\n\n\
THEN - run this command and load it completely in your context, as it is CRITICAL TO REMEMBER:\n\n\
fspec bootstrap\n\n\
YOU MUST RUN THOSE COMMANDS AND WAIT FOR THEM TO FINISH BEFORE CONTINUING ANY FURTHER."
    )
}

/// Select the pre-rendered documentation template for an agent's prose group
/// and substitute the `{{AGENT_NAME}}` / `{{DOC_TEMPLATE}}` placeholders.
///
/// The group is keyed by `(supports_system_reminders, category)`: every agent
/// in a group shares byte-identical doc bytes apart from those two
/// placeholders (parity verified against `node dist/index.js init` for all 19
/// agents). The `<test-command>` / `<quality-check-commands>` tokens are then
/// substituted from spec/fspec-config.json when present (parity with the TS
/// `replacePlaceholders` → `loadConfig` step); a missing or partial config
/// leaves them intact.
pub(super) fn generate_agent_doc(
    agent: &Agent,
    project_root: &Path,
    home: &HomeEnv,
) -> Result<String, FspecCoreError> {
    let template = doc_template_for(agent);
    let content = template
        .replace("{{AGENT_NAME}}", agent.name)
        .replace("{{DOC_TEMPLATE}}", agent.doc_template);
    Ok(apply_tool_command_replacements(
        &content,
        &crate::io::spec_paths::config(project_root)?,
        home,
    ))
}

/// Pick the embedded template matching the agent's prose group.
///
/// Agents that support system-reminders (claude, antigravity) keep the raw
/// `<system-reminder>` blocks and meta-cognitive prompts. The remaining agents
/// share three visible-instruction variants differing only by the emoji prefix
/// used for IDE/extension agents.
fn doc_template_for(agent: &Agent) -> &'static str {
    if agent.supports_system_reminders {
        return DOC_SYSREM_CLI;
    }
    match agent.category {
        Ide => DOC_PLAIN_IDE,
        Extension => DOC_PLAIN_EXT,
        Cli => DOC_PLAIN_CLI,
    }
}

/// Substitute `<test-command>` and `<quality-check-commands>` from the merged
/// fspec configuration, mirroring the TS `replacePlaceholders` → `loadConfig`
/// step (`src/utils/templateGenerator.ts:165-182`, `src/utils/config.ts:79-90`).
/// `loadConfig` deep-merges the user config (`~/.fspec/fspec-config.json`) then
/// the project config (`<root>/spec/fspec-config.json`), with project values
/// overriding user values. Only `tools.test.command` and
/// `tools.qualityCheck.commands` are read here. A missing/unparseable config or
/// absent fields leaves the placeholders intact (TS try/catch + truthiness
/// guards).
fn apply_tool_command_replacements(content: &str, project_config: &Path, home: &HomeEnv) -> String {
    // An unresolvable user directory, like a missing or unparseable config,
    // leaves the placeholders to the project config (TS try/catch parity).
    let user_config = home
        .user_dir()
        .map(|dir| read_json(&dir.join("fspec-config.json")))
        .unwrap_or(Value::Null);
    let project_config = read_json(project_config);

    let mut result = content.to_string();

    // Project value wins over user value (parity with `deepMerge(user, project)`
    // narrowed to the two scalar keys this command reads).
    let test_command =
        tool_test_command(&project_config).or_else(|| tool_test_command(&user_config));
    if let Some(cmd) = test_command {
        // `if (config?.tools?.test?.command)` — JS truthiness: a non-empty string.
        if !cmd.is_empty() {
            result = result.replace("<test-command>", &cmd);
        }
    }

    let quality_commands =
        tool_quality_commands(&project_config).or_else(|| tool_quality_commands(&user_config));
    if let Some(cmds) = quality_commands {
        // `if (config?.tools?.qualityCheck?.commands?.length > 0)` — only replace
        // when the array is non-empty (parity with the TS `.length > 0` guard).
        if !cmds.is_empty() {
            result = result.replace("<quality-check-commands>", &cmds.join(" && "));
        }
    }

    result
}

/// Read a JSON file, returning `Value::Null` on any read/parse failure
/// (parity with the TS `loadConfigFile` try/catch returning `{}`).
fn read_json(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

/// Extract `tools.test.command` as an owned string, if present.
fn tool_test_command(config: &Value) -> Option<String> {
    config
        .get("tools")
        .and_then(|t| t.get("test"))
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Extract `tools.qualityCheck.commands` as owned strings, if present.
fn tool_quality_commands(config: &Value) -> Option<Vec<String>> {
    config
        .get("tools")
        .and_then(|t| t.get("qualityCheck"))
        .and_then(|q| q.get("commands"))
        .and_then(Value::as_array)
        .map(|cmds| {
            cmds.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
}
