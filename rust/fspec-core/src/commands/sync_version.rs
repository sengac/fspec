//! `--sync-version` — BUG-285 port of `src/commands/sync-version.ts` (INIT-011).
//!
//! Feature: spec/features/automatic-version-check-and-update-for-slash-command-files.feature
//!
//! The /fspec slash command written by `init` runs
//! `fspec --sync-version <embedded>` as its first step. This compares the
//! embedded version with the running binary version ([`init::FSPEC_VERSION`],
//! the same constant `init` and `bootstrap` embed):
//!
//!   - match → BUG-607: when the agent's doc or slash command is not what
//!     this fspec writes (missing or different content), reinstall both and
//!     return the restart message + exit 1; otherwise
//!     reuse the configure-tools checks; silent + exit 0 when a test
//!     command is configured, otherwise emit both checks + exit 1 so the agent
//!     runs `fspec configure-tools` first;
//!   - mismatch → detect the agent (spec/fspec-config.json, falling back to
//!     filesystem detection), reinstall its slash command + spec doc through
//!     init's installer, return the restart message + exit 1.
//!
//! Two front doors share this `run`: the shell flag
//! (rust/fspec/src/sync_version.rs) and any JSON caller. The result is
//! `{ "exitCode": 0|1, "output": "<text to print, may be empty>" }`.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::commands::init;
use crate::commands::update_work_unit_status::reminders::{
    check_quality_commands, check_test_command,
};
use crate::error::FspecCoreError;

/// Marker the configure-tools test check emits when no test command is set.
const NO_TEST_COMMAND: &str = "NO TEST COMMAND CONFIGURED";

/// Args: `{ "embeddedVersion": "<version from the slash command>" }`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SyncVersionArgs {
    embedded_version: String,
}

/// Dispatcher + CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: SyncVersionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "sync-version",
            reason: format!("failed to parse args: {e}"),
        })?;

    let current = init::FSPEC_VERSION;
    let (exit_code, mut output) = if args.embedded_version == current {
        on_match(project_root, current)?
    } else {
        on_mismatch(project_root, &args.embedded_version, current)?
    };
    // COV-077: keep the commit gate current. A project outside git gets no
    // notice here (fspec init names it); a hook fspec did not write does.
    let gate = init::commit_gate::install(project_root)?;
    if !matches!(gate, init::commit_gate::Gate::NoRepository) {
        if let Some(message) = gate.message() {
            output = if output.is_empty() {
                message
            } else {
                format!("{output}\n{message}")
            };
        }
    }

    serde_json::to_string(&json!({ "exitCode": exit_code, "output": output })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "sync-version",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Versions match. BUG-607: the agent's files are reinstalled, with a
/// restart request, when their content is not what this fspec writes;
/// otherwise silent when a test command is configured, else the
/// configure-tools test + quality checks stop the workflow.
fn on_match(project_root: &Path, current: &str) -> Result<(u8, String), FspecCoreError> {
    if let Some((agent_id, from_config)) = detect_agent(project_root)? {
        let Some(stale) = init::stale_agent_files(project_root, &agent_id)? else {
            return Ok((1, "⚠️  Unknown agent detected.".to_string()));
        };
        if !stale.is_empty() {
            let Some(supports_system_reminders) =
                init::reinstall_agent_files(project_root, &agent_id)?
            else {
                return Ok((1, "⚠️  Unknown agent detected.".to_string()));
            };
            let instructions = restart_instructions(from_config.then_some(agent_id.as_str()));
            let message = format!(
                "⚠️  fspec files updated: {} did not match fspec v{current}\n\n\
STOP: Please restart your AI agent to load the latest context.\n\n\
{instructions}\n\n\
The outdated documentation may already be in your context.\n\
Restarting ensures you get the complete v{current} workflow.",
                stale.join(", ")
            );
            return Ok((1, wrap(message, supports_system_reminders)));
        }
    }
    let test_check = check_test_command(project_root);
    if !test_check.contains(NO_TEST_COMMAND) {
        return Ok((0, String::new()));
    }
    let quality_check = check_quality_commands(project_root);
    Ok((1, format!("{test_check}\n{quality_check}")))
}

/// The message as the agent sees it: in a system-reminder when it supports them.
fn wrap(message: String, supports_system_reminders: bool) -> String {
    if supports_system_reminders {
        format!("<system-reminder>\n{message}\n</system-reminder>")
    } else {
        message
    }
}

/// Versions differ: reinstall the agent's files and ask for a restart.
fn on_mismatch(
    project_root: &Path,
    embedded: &str,
    current: &str,
) -> Result<(u8, String), FspecCoreError> {
    let Some((agent_id, from_config)) = detect_agent(project_root)? else {
        return Ok((
            1,
            "⚠️  Cannot detect agent. Version mismatch detected.\nPlease run: fspec init"
                .to_string(),
        ));
    };
    let Some(supports_system_reminders) = init::reinstall_agent_files(project_root, &agent_id)?
    else {
        return Ok((1, "⚠️  Unknown agent detected.".to_string()));
    };

    // Agent-specific instructions only when the agent came from the config;
    // a filesystem guess gets the generic message (parity with the TS port).
    let instructions = restart_instructions(from_config.then_some(agent_id.as_str()));
    let message = format!(
        "⚠️  fspec files updated from v{embedded} to v{current}\n\n\
STOP: Please restart your AI agent to load the latest context.\n\n\
{instructions}\n\n\
The old v{embedded} documentation may already be in your context.\n\
Restarting ensures you get the complete v{current} workflow."
    );
    Ok((1, wrap(message, supports_system_reminders)))
}

/// Agent id from spec/fspec-config.json (`fromConfig = true`), else the first
/// agent detected on the filesystem (`fromConfig = false`).
fn detect_agent(project_root: &Path) -> Result<Option<(String, bool)>, FspecCoreError> {
    let config_path = crate::io::spec_paths::config(project_root)?;
    Ok(detect_agent_from(project_root, &config_path))
}

fn detect_agent_from(project_root: &Path, config_path: &Path) -> Option<(String, bool)> {
    let configured = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|cfg| cfg.get("agent").and_then(Value::as_str).map(str::to_string))
        .filter(|id| !id.is_empty());
    if let Some(id) = configured {
        return Some((id, true));
    }
    init::detect_agents(project_root)
        .into_iter()
        .next()
        .map(|id| (id, false))
}

/// Restart instructions per agent (port of `getRestartMessage`'s switch).
fn restart_instructions(agent_id: Option<&str>) -> &'static str {
    match agent_id {
        Some("claude") => "Exit this conversation and start a new one. Run /fspec again.",
        Some("cursor") => "Restart Cursor and run /fspec again",
        Some("aider") => "Restart Aider and run /fspec again",
        Some("windsurf") => "Restart Windsurf and run /fspec again",
        _ => "Restart your AI agent and run /fspec again",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::restart_instructions;

    #[test]
    fn filesystem_detection_gets_the_generic_restart_instructions() {
        assert_eq!(
            restart_instructions(None),
            "Restart your AI agent and run /fspec again"
        );
        assert_eq!(
            restart_instructions(Some("cursor")),
            "Restart Cursor and run /fspec again"
        );
    }
}
