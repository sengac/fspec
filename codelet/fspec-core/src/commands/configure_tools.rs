//! `configure-tools` — Rust port of `src/commands/configure-tools.ts`
//! (RPC-208).
//!
//! Persists the platform-agnostic test + quality-check command configuration
//! to `spec/fspec-config.json`. Read-modify-write semantics:
//!
//! 1. Ensure `spec/` exists.
//! 2. `--reconfigure` short-circuits: emit the `RECONFIGURE TOOLS` guidance
//!    and write NOTHING.
//! 3. Otherwise load the existing config (default `{ "agent": "claude" }`),
//!    seed `tools` if absent, set `tools.test.command` from `testCommand`
//!    and/or `tools.qualityCheck.commands` from `qualityCommands`, then write
//!    the merged document atomically. Previously-stored keys are preserved.
//!
//! ## Deferred divergence (supervisor ruling, orchestration-state.md)
//!
//! * **D3** — the TS source regenerates the agent templates silently after a
//!   write (`installAgentFiles` via the `agent` registry). That template
//!   regeneration is DEFERRED; the Rust port persists the config only.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/configure_tools.rs` is JSON marshalling + stdout
//! rendering only — it contains NO config-merge or file-write logic and emits
//! whatever `message` the core returns verbatim.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// CLI arguments accepted by `configure-tools`. Mirrors the TS
/// `ConfigureToolsOptions` flag set.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigureToolsArgs {
    #[serde(default)]
    test_command: Option<String>,
    #[serde(default)]
    quality_commands: Option<Vec<String>>,
    #[serde(default)]
    reconfigure: bool,
}

/// The `RECONFIGURE TOOLS` guidance emitted by the `--reconfigure` branch.
///
/// TODO(parity-bug RPC-208-D4): the TS reconfigure branch calls
/// `formatAgentOutput(cwd, ...)` passing the `cwd` STRING where an
/// `AgentConfig` is expected, so the message is NOT wrapped in
/// `<system-reminder>` tags — it falls through to the plain prefixed-text
/// branch. We reproduce that bug-for-bug for byte-parity: this constant is
/// the raw, UNWRAPPED message. Revisit when the agent-config plumbing lands.
const RECONFIGURE_MESSAGE: &str = "RECONFIGURE TOOLS\n\nUse Read/Glob tools to detect test frameworks and quality check tools, then run:\n\n  fspec configure-tools --test-command <cmd>\n  fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>'\n";

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ConfigureToolsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "configure-tools",
            reason: format!("failed to parse args: {e}"),
        })?;

    let spec_dir = project_root.join("spec");
    let config_path = spec_dir.join("fspec-config.json");

    // Ensure the spec directory exists (mirrors the TS mkdirSync guard, which
    // runs BEFORE the reconfigure short-circuit).
    std::fs::create_dir_all(&spec_dir).map_err(|source| FspecCoreError::Io {
        command: "configure-tools",
        source,
    })?;

    // [2] --reconfigure short-circuits without writing the config file.
    if args.reconfigure {
        return serde_json::to_string(&json!({
            "success": true,
            "reconfigure": true,
            "message": RECONFIGURE_MESSAGE,
        }))
        .map_err(|e| FspecCoreError::InvalidArgs {
            command: "configure-tools",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    // [3] Load existing config or seed the default `{ "agent": "claude" }`.
    let mut config: Value = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path).map_err(|source| FspecCoreError::Io {
            command: "configure-tools",
            source,
        })?;
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "fspec-config.json".to_string(),
            reason: e.to_string(),
        })?
    } else {
        json!({ "agent": "claude" })
    };

    let root = config
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "fspec-config.json".to_string(),
            reason: "top-level value must be a JSON object".to_string(),
        })?;

    // Seed `tools` if absent.
    let tools_entry = root
        .entry("tools".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !tools_entry.is_object() {
        *tools_entry = Value::Object(serde_json::Map::new());
    }
    let tools = tools_entry
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "fspec-config.json".to_string(),
            reason: "tools must be an object".to_string(),
        })?;

    if let Some(test_command) = args.test_command.as_deref() {
        tools.insert("test".to_string(), json!({ "command": test_command }));
    }

    if let Some(quality_commands) = args.quality_commands.as_ref() {
        tools.insert(
            "qualityCheck".to_string(),
            json!({ "commands": quality_commands }),
        );
    }

    write_json_atomic(&config_path, &config)?;

    // D3 deferred: no installAgentFiles template regeneration here.

    serde_json::to_string(&json!({
        "success": true,
        "reconfigure": false,
        "message": "✓ Tool configuration saved to spec/fspec-config.json",
    }))
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "configure-tools",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_test_command_only() {
        let a: ConfigureToolsArgs =
            serde_json::from_str(r#"{"testCommand":"cargo test"}"#).unwrap();
        assert_eq!(a.test_command.as_deref(), Some("cargo test"));
        assert!(a.quality_commands.is_none());
        assert!(!a.reconfigure);
    }

    #[test]
    fn args_parse_quality_commands_vec() {
        let a: ConfigureToolsArgs =
            serde_json::from_str(r#"{"qualityCommands":["eslint .","prettier --check ."]}"#)
                .unwrap();
        assert_eq!(
            a.quality_commands.as_ref().map(Vec::as_slice),
            Some(["eslint .".to_string(), "prettier --check .".to_string()].as_slice())
        );
    }

    #[test]
    fn args_parse_reconfigure_flag() {
        let a: ConfigureToolsArgs = serde_json::from_str(r#"{"reconfigure":true}"#).unwrap();
        assert!(a.reconfigure);
    }

    #[test]
    fn reconfigure_message_is_not_wrapped_in_system_reminder() {
        // RPC-208-D4 bug parity: the guidance must NOT be wrapped.
        assert!(RECONFIGURE_MESSAGE.contains("RECONFIGURE TOOLS"));
        assert!(!RECONFIGURE_MESSAGE.contains("<system-reminder>"));
    }
}
