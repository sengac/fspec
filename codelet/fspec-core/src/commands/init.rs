//! `init` — Rust port of `src/commands/init.ts` (RPC-239).
//!
//! Scaffolds fspec into a project for one or more AI agents using BLOCKING
//! `std::fs` (no `.await`, no child processes, no network) so it runs under
//! `poll_sync_future`. Both front doors call this single `run`:
//!   - LLM tool call JSON → dispatch_command → init::run
//!   - Shell argv → clap → codelet/fspec/src/init.rs → init::run
//!
//! Per-agent doc generation selects one of four pre-rendered, byte-exact
//! templates (captured from `node dist/index.js init`, keyed by the agent's
//! `(supports_system_reminders, category)` prose group) and substitutes the
//! `{{AGENT_NAME}}` / `{{DOC_TEMPLATE}}` placeholders plus the optional
//! `<test-command>` / `<quality-check-commands>` config tokens. This mirrors
//! the RPC-200 bootstrap "embed the byte-exact output" strategy because the TS
//! pipeline (22 section generators with interleaved agent-conditional prose and
//! two different system-reminder transforms) cannot be reproduced verbatim by a
//! single in-Rust transform.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;

/// fspec version embedded into the slash command template
/// (parity with `getVersion()` reading package.json — pinned to 0.9.3).
const FSPEC_VERSION: &str = "0.9.3";

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
const DOC_SYSREM_CLI: &str = include_str!("init_doc_sysrem_cli.md");
const DOC_PLAIN_IDE: &str = include_str!("init_doc_plain_ide.md");
const DOC_PLAIN_EXT: &str = include_str!("init_doc_plain_ext.md");
const DOC_PLAIN_CLI: &str = include_str!("init_doc_plain_cli.md");

/// Slash command format for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlashFormat {
    Markdown,
    Toml,
}

/// Agent category, mirroring the TS `AgentConfig.category`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Ide,
    Cli,
    Extension,
}

/// Inlined agent registry entry — port of `AGENT_REGISTRY`
/// (`src/utils/agentRegistry.ts`).
#[derive(Debug, Clone, Copy)]
struct Agent {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    slash_command_path: &'static str,
    slash_command_format: SlashFormat,
    supports_system_reminders: bool,
    doc_template: &'static str,
    category: Category,
    /// Relative directories that, when present in a project root, indicate
    /// this agent is already in use (port of `AgentConfig.detectionPaths`).
    detection_paths: &'static [&'static str],
    /// Whether the agent is offered in the interactive selector (port of
    /// `AgentConfig.available`). All current agents are available.
    available: bool,
}

use Category::{Cli, Extension, Ide};
use SlashFormat::{Markdown, Toml};

/// Inlined agent registry table (19 agents). Order matches the TS source.
const AGENT_REGISTRY: &[Agent] = &[
    Agent { id: "claude", name: "Claude Code", description: "Anthropic CLI (nested slash commands, system-reminders)", slash_command_path: ".claude/commands/", slash_command_format: Markdown, supports_system_reminders: true, doc_template: "CLAUDE.md", category: Cli, detection_paths: &[".claude/", ".claude/commands/"], available: true },
    Agent { id: "cursor", name: "Cursor", description: "IDE with AI assistant (flat slash commands)", slash_command_path: ".cursor/commands/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "CURSOR.md", category: Ide, detection_paths: &[".cursor/", ".cursor/commands/"], available: true },
    Agent { id: "cline", name: "Cline", description: "VS Code extension (no system-reminders)", slash_command_path: ".cline/commands/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "CLINE.md", category: Extension, detection_paths: &[".cline/", ".continue/"], available: true },
    Agent { id: "aider", name: "Aider", description: "CLI agent (no meta-cognitive prompts)", slash_command_path: ".aider/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "AIDER.md", category: Cli, detection_paths: &[".aider/"], available: true },
    Agent { id: "windsurf", name: "Windsurf", description: "IDE-based agent", slash_command_path: ".windsurf/workflows/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "WINDSURF.md", category: Ide, detection_paths: &[".windsurf/"], available: true },
    Agent { id: "copilot", name: "GitHub Copilot", description: "GitHub AI assistant", slash_command_path: ".github/prompts/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "COPILOT.md", category: Extension, detection_paths: &[".github/prompts/"], available: true },
    Agent { id: "gemini", name: "Gemini CLI", description: "Google CLI agent (TOML format)", slash_command_path: ".gemini/commands/", slash_command_format: Toml, supports_system_reminders: false, doc_template: "GEMINI.md", category: Cli, detection_paths: &[".gemini/"], available: true },
    Agent { id: "qwen", name: "Qwen Code", description: "Qwen CLI agent (TOML format)", slash_command_path: ".qwen/commands/", slash_command_format: Toml, supports_system_reminders: false, doc_template: "QWEN.md", category: Cli, detection_paths: &[".qwen/"], available: true },
    Agent { id: "kilocode", name: "Kilo Code", description: "IDE agent with rules-based pattern", slash_command_path: ".kilocode/rules/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "KILOCODE.md", category: Ide, detection_paths: &[".kilocode/"], available: true },
    Agent { id: "roo", name: "Roo Code", description: "IDE agent with rules pattern", slash_command_path: ".roo/rules/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "ROO.md", category: Ide, detection_paths: &[".roo/"], available: true },
    Agent { id: "codebuddy", name: "CodeBuddy", description: "CLI-based agent", slash_command_path: ".codebuddy/commands/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "CODEBUDDY.md", category: Cli, detection_paths: &[".codebuddy/"], available: true },
    Agent { id: "amazonq", name: "Amazon Q", description: "AWS AI assistant", slash_command_path: ".amazonq/prompts/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "AMAZONQ.md", category: Extension, detection_paths: &[".amazonq/"], available: true },
    Agent { id: "auggie", name: "Auggie", description: "Augment CLI agent", slash_command_path: ".auggie/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "AUGGIE.md", category: Cli, detection_paths: &[".auggie/"], available: true },
    Agent { id: "opencode", name: "OpenCode", description: "CLI agent", slash_command_path: ".opencode/command/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "OPENCODE.md", category: Cli, detection_paths: &[".opencode/"], available: true },
    Agent { id: "codex", name: "Codex", description: "OpenAI Codex agent", slash_command_path: ".codex/prompts/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "AGENTS.md", category: Cli, detection_paths: &[".codex/"], available: true },
    Agent { id: "factory", name: "Factory Droid", description: "Factory agent", slash_command_path: ".factory/commands/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "FACTORY.md", category: Cli, detection_paths: &[".factory/"], available: true },
    Agent { id: "crush", name: "Crush", description: "Crush agent", slash_command_path: ".crush/commands/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "CRUSH.md", category: Cli, detection_paths: &[".crush/"], available: true },
    Agent { id: "codex-cli", name: "Codex CLI", description: "Codex command-line interface", slash_command_path: ".codex/prompts/", slash_command_format: Markdown, supports_system_reminders: false, doc_template: "AGENTS.md", category: Cli, detection_paths: &[".codex-cli/"], available: true },
    Agent { id: "antigravity", name: "Antigravity", description: "Google Deepmind Agentic AI", slash_command_path: ".antigravity/commands/", slash_command_format: Markdown, supports_system_reminders: true, doc_template: "ANTIGRAVITY.md", category: Cli, detection_paths: &[".antigravity/"], available: true },
];

fn get_agent_by_id(id: &str) -> Option<&'static Agent> {
    AGENT_REGISTRY.iter().find(|a| a.id == id)
}

/// Public, terminal-free summary of an available agent, used by the CLI
/// bridge to build the interactive selector list without duplicating the
/// inlined registry. Mirrors the fields the TS `AgentSelector` reads from
/// `AgentConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// All agents offered in the interactive selector (port of
/// `getAvailableAgents()` — `AGENT_REGISTRY.filter(a => a.available)`),
/// preserving registry order.
pub fn available_agents() -> Vec<AgentInfo> {
    AGENT_REGISTRY
        .iter()
        .filter(|a| a.available)
        .map(|a| AgentInfo {
            id: a.id.to_string(),
            name: a.name.to_string(),
            description: a.description.to_string(),
        })
        .collect()
}

/// Detect agents already in use under `project_root` by probing each agent's
/// `detection_paths` (port of `detectAgents` — `src/utils/agentDetection.ts`).
/// Returns the matching agent ids in registry order; an agent is recorded at
/// most once even if multiple of its detection paths exist.
pub fn detect_agents(project_root: &Path) -> Vec<String> {
    AGENT_REGISTRY
        .iter()
        .filter(|a| {
            a.detection_paths
                .iter()
                .any(|rel| project_root.join(rel).exists())
        })
        .map(|a| a.id.to_string())
        .collect()
}

/// Agent-specific activation message (port of getActivationMessage,
/// `src/utils/activationMessage.ts`).
fn activation_message(agent: &Agent) -> String {
    match agent.id {
        "claude" => "Run /fspec in Claude Code to activate".to_string(),
        "codex" => "Run /prompts:fspec in Codex to activate".to_string(),
        "codex-cli" => "Run /prompts:fspec in Codex CLI to activate".to_string(),
        "cursor" => "Open .cursor/commands/ in Cursor to activate".to_string(),
        "aider" => "Add .aider/ to your Aider configuration to activate".to_string(),
        _ => match agent.category {
            Ide | Extension => {
                format!("Open {} in {} to activate", agent.slash_command_path, agent.name)
            }
            Cli if !agent.slash_command_path.is_empty() => format!(
                "Add {} to your {} configuration to activate",
                agent.slash_command_path, agent.name
            ),
            _ => "Refer to your AI agent documentation to activate fspec".to_string(),
        },
    }
}

/// Args accepted by `init`. The dispatcher passes `{ "agent": ["claude", ...] }`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InitArgs {
    /// Agent ids to install. An EMPTY list is the headless/no-TTY case which
    /// the TS action handler rejects (interactive selection is unsupported in
    /// a non-TTY dispatcher).
    #[serde(default)]
    agent: Vec<String>,
}

/// Dispatcher + CLI entry point. 2-arg signature: raw JSON args + the
/// canonical project root. Returns a JSON string describing the install
/// (`{ filesInstalled, cancelled, success, agents }`).
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: InitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "init",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Headless guard: no TTY here, so an empty agent list cannot be resolved
    // via interactive selection (TS action handler at init.ts:308-316).
    if args.agent.is_empty() {
        return Err(FspecCoreError::Message(
            "Interactive mode requires a TTY. Use --agent flag instead:\n  \
             fspec init --agent=claude\n  fspec init --agent=cursor --agent=claude"
                .to_string(),
        ));
    }

    // Validate every requested agent id up-front (parity with installAgents
    // at init.ts:99-111 — the first unknown id aborts before any file write).
    for agent_id in &args.agent {
        if get_agent_by_id(agent_id).is_none() {
            let valid = AGENT_REGISTRY
                .iter()
                .map(|a| format!("  - {}: {}", a.id, a.description))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(FspecCoreError::Message(format!(
                "Unknown agent: {agent_id}.\n\nValid agent IDs:\n{valid}"
            )));
        }
    }

    // Install each agent in order, accumulating the installed display paths.
    let mut files_installed: Vec<String> = Vec::new();
    for agent_id in &args.agent {
        let agent = get_agent_by_id(agent_id)
            .ok_or_else(|| FspecCoreError::Message(format!("Unknown agent: {agent_id}")))?;
        let files = install_agent_files(project_root, agent)?;
        files_installed.extend(files);
    }

    // Write agent config recording the FIRST agent (read-modify-write so
    // pre-existing keys survive — parity with writeAgentConfig).
    write_agent_config(project_root, &args.agent[0])?;

    let first = get_agent_by_id(&args.agent[0]).ok_or_else(|| {
        FspecCoreError::Message(format!("Unknown agent: {}", args.agent[0]))
    })?;
    let result = json!({
        "filesInstalled": files_installed,
        "cancelled": false,
        "success": true,
        "agents": args.agent,
        "activationMessage": activation_message(first),
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "init",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Install the doc file + slash command for one agent. Returns the display
/// paths recorded in `filesInstalled` (parity with installAgentFiles).
fn install_agent_files(project_root: &Path, agent: &Agent) -> Result<Vec<String>, FspecCoreError> {
    let mut installed = Vec::new();

    // 1. Full documentation: spec/<DOC_TEMPLATE>.
    let spec_dir = project_root.join("spec");
    fs_create_dir_all(&spec_dir)?;
    let doc_path = spec_dir.join(agent.doc_template);
    let doc = generate_agent_doc(agent, project_root);
    fs_write(&doc_path, &doc)?;
    installed.push(format!("spec/{}", agent.doc_template));

    // 2. Slash command file.
    let display = install_slash_command(project_root, agent)?;
    installed.push(display);

    Ok(installed)
}

/// Install the slash command for an agent and return its display path.
///
/// Codex / codex-cli write to `$HOME/.codex/prompts/fspec.md` (HOME read from
/// the injectable process env, never hard-coded). All others write under the
/// project root at `agent.slash_command_path`.
fn install_slash_command(project_root: &Path, agent: &Agent) -> Result<String, FspecCoreError> {
    let is_codex = agent.id == "codex" || agent.id == "codex-cli";
    let commands_dir = if is_codex {
        home_dir().join(".codex").join("prompts")
    } else {
        project_root.join(agent.slash_command_path)
    };
    fs_create_dir_all(&commands_dir)?;

    let filename = if agent.slash_command_format == Toml {
        "fspec.toml"
    } else {
        "fspec.md"
    };
    let command_path = commands_dir.join(filename);
    let content = generate_slash_command_content(agent);
    fs_write(&command_path, &content)?;

    if is_codex {
        Ok("~/.codex/prompts/fspec.md".to_string())
    } else {
        Ok(format!("{}{}", agent.slash_command_path, filename))
    }
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
1. fspec --help\n2. fspec help specs\n3. fspec help work\n4. fspec help discovery\n\n\
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
fn generate_agent_doc(agent: &Agent, project_root: &Path) -> String {
    let template = doc_template_for(agent);
    let content = template
        .replace("{{AGENT_NAME}}", agent.name)
        .replace("{{DOC_TEMPLATE}}", agent.doc_template);
    apply_tool_command_replacements(&content, project_root)
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
fn apply_tool_command_replacements(content: &str, project_root: &Path) -> String {
    let user_config = read_json(&fspec_user_dir().join("fspec-config.json"));
    let project_config = read_json(&project_root.join("spec").join("fspec-config.json"));

    let mut result = content.to_string();

    // Project value wins over user value (parity with `deepMerge(user, project)`
    // narrowed to the two scalar keys this command reads).
    let test_command = tool_test_command(&project_config).or_else(|| tool_test_command(&user_config));
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

/// Resolve `~/.fspec` (parity with the TS `getFspecUserDir`
/// = `join(homedir(), '.fspec')`), reusing the injectable HOME resolution. The
/// (absent) file simply doesn't contribute when HOME is unset, leaving
/// placeholders intact.
fn fspec_user_dir() -> PathBuf {
    home_dir().join(".fspec")
}

/// Write spec/fspec-config.json recording `agent_id`, preserving any existing
/// keys (read-modify-write) — port of writeAgentConfig
/// (`src/utils/agentRuntimeConfig.ts:65-90`). 2-space-indented JSON.
fn write_agent_config(project_root: &Path, agent_id: &str) -> Result<(), FspecCoreError> {
    let spec_dir = project_root.join("spec");
    fs_create_dir_all(&spec_dir)?;
    let config_path = spec_dir.join("fspec-config.json");

    // Start from any existing object, falling back to a fresh map on read or
    // parse failure (parity with the TS try/catch).
    let mut obj = match std::fs::read_to_string(&config_path) {
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        },
        Err(_) => serde_json::Map::new(),
    };
    obj.insert("agent".to_string(), json!(agent_id));

    let serialized = serde_json::to_string_pretty(&Value::Object(obj)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "init",
            reason: format!("failed to serialize config: {e}"),
        }
    })?;
    fs_write(&config_path, &serialized)
}

/// Resolve the home directory from the injectable `HOME` env var (never a
/// hard-coded path) so tests can redirect codex writes into a sandbox.
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Blocking `create_dir_all` mapping errors into the command-tagged variant.
fn fs_create_dir_all(path: &Path) -> Result<(), FspecCoreError> {
    std::fs::create_dir_all(path).map_err(|source| FspecCoreError::Io {
        command: "init",
        source,
    })
}

/// Blocking `write` mapping errors into the command-tagged variant.
fn fs_write(path: &Path, contents: &str) -> Result<(), FspecCoreError> {
    std::fs::write(path, contents).map_err(|source| FspecCoreError::Io {
        command: "init",
        source,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn registry_contains_claude_and_gemini() {
        assert!(get_agent_by_id("claude").is_some());
        assert_eq!(get_agent_by_id("gemini").unwrap().slash_command_format, Toml);
        assert!(get_agent_by_id("bogus").is_none());
    }

    #[test]
    fn available_agents_lists_all_registry_agents_in_order() {
        let agents = available_agents();
        assert_eq!(agents.len(), AGENT_REGISTRY.len());
        assert_eq!(agents[0].id, "claude");
        assert_eq!(agents[0].name, "Claude Code");
    }

    #[test]
    fn detect_agents_finds_an_agent_by_its_detection_path() {
        let tmp = TempDir::new().unwrap();
        // A `.cursor/` directory marks the cursor agent as already in use.
        std::fs::create_dir_all(tmp.path().join(".cursor")).unwrap();
        let detected = detect_agents(tmp.path());
        assert_eq!(detected, vec!["cursor".to_string()]);
    }

    #[test]
    fn detect_agents_returns_empty_for_a_clean_project() {
        let tmp = TempDir::new().unwrap();
        assert!(detect_agents(tmp.path()).is_empty());
    }

    #[test]
    fn placeholders_are_replaced_for_claude() {
        let tmp = TempDir::new().unwrap();
        let claude = get_agent_by_id("claude").unwrap();
        let doc = generate_agent_doc(claude, tmp.path());
        assert!(!doc.contains("{{AGENT_NAME}}"));
        assert!(!doc.contains("{{DOC_TEMPLATE}}"));
        assert!(doc.contains("Claude Code"));
    }

    #[test]
    fn system_reminders_preserved_for_claude_stripped_for_gemini() {
        let tmp = TempDir::new().unwrap();
        let claude = get_agent_by_id("claude").unwrap();
        let gemini = get_agent_by_id("gemini").unwrap();
        assert!(generate_agent_doc(claude, tmp.path()).contains("<system-reminder>"));
        let g = generate_agent_doc(gemini, tmp.path());
        assert!(!g.contains("<system-reminder>"));
        assert!(g.contains("**IMPORTANT:**"));
    }

    #[test]
    fn meta_cognitive_phrases_removed_for_gemini() {
        let tmp = TempDir::new().unwrap();
        let gemini = get_agent_by_id("gemini").unwrap();
        let g = generate_agent_doc(gemini, tmp.path());
        assert!(!g.to_lowercase().contains("ultrathink"));
        assert!(!g.to_lowercase().contains("deeply consider"));
    }

    #[test]
    fn ide_agent_uses_emoji_important_prefix() {
        let tmp = TempDir::new().unwrap();
        let cursor = get_agent_by_id("cursor").unwrap();
        let doc = generate_agent_doc(cursor, tmp.path());
        assert!(!doc.contains("<system-reminder>"));
        assert!(doc.contains("**⚠️ IMPORTANT:**"));
    }

    #[test]
    fn tool_commands_substituted_from_project_config() {
        let tmp = TempDir::new().unwrap();
        let spec = tmp.path().join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(
            spec.join("fspec-config.json"),
            r#"{ "tools": { "test": { "command": "npm test" },
                 "qualityCheck": { "commands": ["npm run lint", "npm run fmt"] } } }"#,
        )
        .unwrap();
        let cursor = get_agent_by_id("cursor").unwrap();
        let doc = generate_agent_doc(cursor, tmp.path());
        assert!(!doc.contains("<test-command>"));
        assert!(!doc.contains("<quality-check-commands>"));
        assert!(doc.contains("npm test"));
        assert!(doc.contains("npm run lint && npm run fmt"));
    }

    #[test]
    fn tool_command_placeholders_intact_without_config() {
        // Redirect HOME at an empty sandbox so a developer ~/.fspec config does
        // not substitute the placeholders under test.
        let home = TempDir::new().unwrap();
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());

        let tmp = TempDir::new().unwrap();
        let claude = get_agent_by_id("claude").unwrap();
        let doc = generate_agent_doc(claude, tmp.path());

        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }

        assert!(doc.contains("<test-command>"));
        assert!(doc.contains("<quality-check-commands>"));
    }
}
