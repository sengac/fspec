//! `init` — Rust port of `src/commands/init.ts` (RPC-239).
//!
//! Scaffolds fspec into a project for one or more AI agents using BLOCKING
//! `std::fs` (no `.await`, no network; COV-077: one `git rev-parse` for the
//! commit gate's hooks directory, see `commit_gate`) so it runs under
//! `poll_sync_future`. Both front doors call this single `run`:
//!   - LLM tool call JSON → dispatch_command → init::run
//!   - Shell argv → clap → rust/fspec/src/init.rs → init::run
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

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;

/// fspec version embedded into the slash command template — the running
/// binary's workspace version (BUG-285: the old pinned `0.9.3` never matched
/// the binary, so `fspec --sync-version` always reported a mismatch). Shared
/// with `bootstrap` and `sync_version` so all three agree.
pub(crate) const FSPEC_VERSION: &str = env!("CARGO_PKG_VERSION");

mod agents;
pub(crate) mod commit_gate;
mod files;
mod registry;
mod templates;

pub use agents::{available_agents, detect_agents, AgentInfo};
pub use files::{HomeEnv, HomePlatform};

use agents::activation_message;
use files::{fs_create_dir_all, fs_write, write_agent_config};
use registry::{get_agent_by_id, Agent, AGENT_REGISTRY};
use templates::{generate_agent_doc, is_codex, slash_command_file};

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
    run_with_home(args_json, project_root, &HomeEnv::process())
}

/// [`run`] with the platform and environment the home directory is resolved
/// from injected (BUG-533): the Codex prompt goes to `<home>/.codex/prompts`
/// and the user config is read from `<home>/.fspec`.
pub fn run_with_home(
    args_json: &str,
    project_root: &Path,
    home: &HomeEnv,
) -> Result<String, FspecCoreError> {
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

    // BUG-533: a Codex agent needs the home directory; refuse before any
    // file is written rather than fall back to the project.
    for agent_id in &args.agent {
        if get_agent_by_id(agent_id).is_some_and(is_codex) {
            home.require_home_dir()?;
        }
    }

    // Install each agent in order, accumulating the installed display paths.
    let mut files_installed: Vec<String> = Vec::new();
    for agent_id in &args.agent {
        let agent = get_agent_by_id(agent_id)
            .ok_or_else(|| FspecCoreError::Message(format!("Unknown agent: {agent_id}")))?;
        let files = install_agent_files(project_root, agent, home)?;
        files_installed.extend(files);
    }

    // Write agent config recording the FIRST agent (read-modify-write so
    // pre-existing keys survive — parity with writeAgentConfig).
    write_agent_config(project_root, &args.agent[0])?;

    // COV-077: the git pre-commit hook that runs fspec audit --changed.
    let commit_gate = commit_gate::install(project_root)?.message();

    let first = get_agent_by_id(&args.agent[0])
        .ok_or_else(|| FspecCoreError::Message(format!("Unknown agent: {}", args.agent[0])))?;
    let result = json!({
        "filesInstalled": files_installed,
        "cancelled": false,
        "success": true,
        "agents": args.agent,
        "activationMessage": activation_message(first),
        "commitGate": commit_gate,
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "init",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// BUG-285: reinstall the doc file + slash command for `agent_id` (the
/// `--sync-version` mismatch path), reusing [`install_agent_files`]. Returns
/// `Ok(None)` for an unknown agent id, otherwise whether the agent supports
/// `<system-reminder>` tags.
pub(crate) fn reinstall_agent_files(
    project_root: &Path,
    agent_id: &str,
) -> Result<Option<bool>, FspecCoreError> {
    let Some(agent) = get_agent_by_id(agent_id) else {
        return Ok(None);
    };
    install_agent_files(project_root, agent, &HomeEnv::process())?;
    Ok(Some(agent.supports_system_reminders))
}

/// BUG-607: the display paths of `agent_id`'s doc and slash command files
/// whose content on disk is not what [`install_agent_files`] would write now
/// (a missing file counts). `Ok(None)` for an unknown agent id.
pub(crate) fn stale_agent_files(
    project_root: &Path,
    agent_id: &str,
) -> Result<Option<Vec<String>>, FspecCoreError> {
    let Some(agent) = get_agent_by_id(agent_id) else {
        return Ok(None);
    };
    let mut stale = Vec::new();
    for file in plan_agent_files(project_root, agent, &HomeEnv::process())? {
        let current = match std::fs::read(&file.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(FspecCoreError::Io {
                    command: "sync-version",
                    source,
                })
            }
        };
        if current.as_deref() != Some(file.content.as_bytes()) {
            stale.push(file.display);
        }
    }
    Ok(Some(stale))
}

/// One file the installer writes for an agent.
struct AgentFile {
    path: std::path::PathBuf,
    display: String,
    content: String,
}

/// The doc file + slash command for one agent, as the installer writes them.
fn plan_agent_files(
    project_root: &Path,
    agent: &Agent,
    home: &HomeEnv,
) -> Result<Vec<AgentFile>, FspecCoreError> {
    // 1. Full documentation: spec/<DOC_TEMPLATE>.
    let spec_dir = crate::io::spec_paths::spec_dir_of(project_root);
    let doc = AgentFile {
        path: spec_dir.join(agent.doc_template),
        display: format!("spec/{}", agent.doc_template),
        content: generate_agent_doc(agent, project_root, home)?,
    };
    // 2. Slash command file.
    let (path, display, content) = slash_command_file(project_root, agent, home)?;
    Ok(vec![
        doc,
        AgentFile {
            path,
            display,
            content,
        },
    ])
}

/// Install the doc file + slash command for one agent. Returns the display
/// paths recorded in `filesInstalled` (parity with installAgentFiles).
fn install_agent_files(
    project_root: &Path,
    agent: &Agent,
    home: &HomeEnv,
) -> Result<Vec<String>, FspecCoreError> {
    let mut installed = Vec::new();
    for file in plan_agent_files(project_root, agent, home)? {
        if let Some(parent) = file.path.parent() {
            fs_create_dir_all(parent)?;
        }
        fs_write(&file.path, &file.content)?;
        installed.push(file.display);
    }
    Ok(installed)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod placeholder_tests;
