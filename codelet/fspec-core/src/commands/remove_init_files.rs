//! `remove-init-files` — Rust port of `removeInitFiles` /
//! `executeRemoveInitFiles` in `src/commands/remove-init-files.ts` (RPC-276).
//!
//! File-deletion command: removes the agent-specific fspec initialization
//! files (and optionally `spec/fspec-config.json`) for the detected agent.
//!
//! ## Agent detection (parity with the TS exported function)
//!
//! 1. Read `spec/fspec-config.json` and use its `.agent` field if present,
//!    parseable, AND a non-empty string (TS `if (config.agent)`).
//! 2. Otherwise scan each agent's `detectionPaths` (in registry order) and
//!    pick the FIRST agent any of whose detection paths exists in `cwd`.
//!    (NOTE: the TS file-detection branch returns the whole `DetectedAgent`
//!    object — a bug producing `Unknown agent: [object Object]`. The
//!    supervisor-approved `remove-init-files-rust-port` spec encodes the
//!    INTENDED behaviour — resolve the agent id — so we follow the spec.)
//! 3. No agent detected → error
//!    `No fspec agent installation detected. Nothing to remove.`
//! 4. Detected agent id unknown → error `Unknown agent: <id>`.
//!
//! ## Removal (parity)
//!
//! * `spec/<docTemplate>` (e.g. `spec/CLAUDE.md`).
//! * `<slashCommandPath><fspec.md|fspec.toml>` (filename depends on the
//!   agent's slash-command format).
//! * Both use FORCE removal so missing files are silently skipped
//!   (idempotent); `filesRemoved` still lists the attempted paths.
//! * `spec/fspec-config.json` is removed UNLESS `keepConfig == true`.
//!
//! ## keepConfig (supervisor decision)
//!
//! The interactive Ink prompt (TS uses it when `keepConfig` is undefined) is
//! not reproducible headless. An UNSPECIFIED `keepConfig` defaults to `false`
//! (remove config), matching the destructive `--no-keep-config` default;
//! `--keep-config` overrides to preserve.
//!
//! The agent registry is INLINED as a local `const` table here (supervisor
//! decision — `init.rs` is still a stub and no shared `agents.rs` module
//! exists). Field set mirrors the subset of `AGENT_REGISTRY` this command
//! needs (`id`, `docTemplate`, `slashCommandPath`, `slashCommandFormat`,
//! `detectionPaths`).
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge
//! both call this single function — no inline detection/deletion elsewhere.

use std::path::Path;

use serde::Deserialize;

use crate::error::FspecCoreError;

/// Inlined subset of an `AGENT_REGISTRY` entry needed by remove-init-files.
struct Agent {
    id: &'static str,
    doc_template: &'static str,
    slash_command_path: &'static str,
    /// `true` → slash command file is `fspec.toml`; `false` → `fspec.md`.
    slash_command_toml: bool,
    detection_paths: &'static [&'static str],
}

/// Local agent registry, in the same order as the TS `AGENT_REGISTRY` so that
/// detection picks the same first-match.
const AGENTS: &[Agent] = &[
    Agent { id: "claude", doc_template: "CLAUDE.md", slash_command_path: ".claude/commands/", slash_command_toml: false, detection_paths: &[".claude/", ".claude/commands/"] },
    Agent { id: "cursor", doc_template: "CURSOR.md", slash_command_path: ".cursor/commands/", slash_command_toml: false, detection_paths: &[".cursor/", ".cursor/commands/"] },
    Agent { id: "cline", doc_template: "CLINE.md", slash_command_path: ".cline/commands/", slash_command_toml: false, detection_paths: &[".cline/", ".continue/"] },
    Agent { id: "aider", doc_template: "AIDER.md", slash_command_path: ".aider/", slash_command_toml: false, detection_paths: &[".aider/"] },
    Agent { id: "windsurf", doc_template: "WINDSURF.md", slash_command_path: ".windsurf/workflows/", slash_command_toml: false, detection_paths: &[".windsurf/"] },
    Agent { id: "copilot", doc_template: "COPILOT.md", slash_command_path: ".github/prompts/", slash_command_toml: false, detection_paths: &[".github/prompts/"] },
    Agent { id: "gemini", doc_template: "GEMINI.md", slash_command_path: ".gemini/commands/", slash_command_toml: true, detection_paths: &[".gemini/"] },
    Agent { id: "qwen", doc_template: "QWEN.md", slash_command_path: ".qwen/commands/", slash_command_toml: true, detection_paths: &[".qwen/"] },
    Agent { id: "kilocode", doc_template: "KILOCODE.md", slash_command_path: ".kilocode/rules/", slash_command_toml: false, detection_paths: &[".kilocode/"] },
    Agent { id: "roo", doc_template: "ROO.md", slash_command_path: ".roo/rules/", slash_command_toml: false, detection_paths: &[".roo/"] },
    Agent { id: "codebuddy", doc_template: "CODEBUDDY.md", slash_command_path: ".codebuddy/commands/", slash_command_toml: false, detection_paths: &[".codebuddy/"] },
    Agent { id: "amazonq", doc_template: "AMAZONQ.md", slash_command_path: ".amazonq/prompts/", slash_command_toml: false, detection_paths: &[".amazonq/"] },
    Agent { id: "auggie", doc_template: "AUGGIE.md", slash_command_path: ".auggie/", slash_command_toml: false, detection_paths: &[".auggie/"] },
    Agent { id: "opencode", doc_template: "OPENCODE.md", slash_command_path: ".opencode/command/", slash_command_toml: false, detection_paths: &[".opencode/"] },
    Agent { id: "codex", doc_template: "AGENTS.md", slash_command_path: ".codex/prompts/", slash_command_toml: false, detection_paths: &[".codex/"] },
    Agent { id: "factory", doc_template: "FACTORY.md", slash_command_path: ".factory/commands/", slash_command_toml: false, detection_paths: &[".factory/"] },
    Agent { id: "crush", doc_template: "CRUSH.md", slash_command_path: ".crush/commands/", slash_command_toml: false, detection_paths: &[".crush/"] },
    Agent { id: "codex-cli", doc_template: "AGENTS.md", slash_command_path: ".codex/prompts/", slash_command_toml: false, detection_paths: &[".codex-cli/"] },
    Agent { id: "antigravity", doc_template: "ANTIGRAVITY.md", slash_command_path: ".antigravity/commands/", slash_command_toml: false, detection_paths: &[".antigravity/"] },
];

fn agent_by_id(id: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.id == id)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveInitFilesArgs {
    /// `Some(true)` → preserve `spec/fspec-config.json`; `Some(false)` or
    /// `None` → remove it (headless default).
    #[serde(default)]
    keep_config: Option<bool>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveInitFilesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-init-files",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Unspecified keepConfig → false (remove config), per supervisor decision.
    let keep_config = args.keep_config.unwrap_or(false);

    // TS `detectInstalledAgent` returns EITHER the config `.agent` string OR
    // (on the file-detection fall-back) the whole `DetectedAgent` OBJECT. The
    // object stringifies to `[object Object]` when interpolated into the
    // `Unknown agent: <id>` error, so file detection NEVER resolves to a valid
    // agent in the TS source. We mirror that exact behaviour.
    let detected_id = match detect_installed_agent(project_root) {
        Some(id) => id,
        None => {
            return Err(FspecCoreError::Message(
                "No fspec agent installation detected. Nothing to remove.".to_string(),
            ));
        }
    };

    let agent = agent_by_id(&detected_id)
        .ok_or_else(|| FspecCoreError::Message(format!("Unknown agent: {detected_id}")))?;

    let mut files_removed: Vec<String> = Vec::new();

    // Remove spec/<docTemplate>.
    let doc_rel = format!("spec/{}", agent.doc_template);
    force_remove(project_root, &doc_rel)?;
    files_removed.push(doc_rel);

    // Remove the slash command file.
    let filename = if agent.slash_command_toml {
        "fspec.toml"
    } else {
        "fspec.md"
    };
    let slash_rel = format!("{}{filename}", agent.slash_command_path);
    force_remove(project_root, &slash_rel)?;
    files_removed.push(slash_rel);

    // Optionally remove the config file.
    if !keep_config {
        force_remove(project_root, "spec/fspec-config.json")?;
        files_removed.push("spec/fspec-config.json".to_string());
    }

    let payload = serde_json::json!({ "filesRemoved": files_removed });
    serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-init-files",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Detect the installed agent id, mirroring the TS `detectInstalledAgent`
/// (with the supervisor-approved intended behaviour for the directory branch).
///
/// Returns the config `.agent` string when present AND truthy (TS uses
/// `if (config.agent)`, so empty string / null fall through). Otherwise scan
/// each agent's `detectionPaths` (registry order) and return the FIRST agent
/// id any of whose detection paths exists in `cwd`.
///
/// NOTE: the TS file-detection branch returns the whole `DetectedAgent` OBJECT
/// (a bug — it stringifies to `[object Object]` and fails the downstream
/// `getAgentById`). The supervisor-approved spec (`remove-init-files-rust-port`
/// BUSINESS RULE #1 / EXAMPLE #2) encodes the INTENDED behaviour — return the
/// detected agent's id — so directory detection resolves correctly. We follow
/// the spec, not the TS bug.
fn detect_installed_agent(project_root: &Path) -> Option<String> {
    let config_path = project_root.join("spec").join("fspec-config.json");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                // TS truthiness: a non-empty string `.agent` short-circuits;
                // empty string / missing / non-string falls through.
                if let Some(agent) = value.get("agent").and_then(serde_json::Value::as_str) {
                    if !agent.is_empty() {
                        return Some(agent.to_string());
                    }
                }
            }
        }
        // Fall through to file detection on parse failure.
    }

    for agent in AGENTS {
        for path in agent.detection_paths {
            if project_root.join(path).exists() {
                return Some(agent.id.to_string());
            }
        }
    }
    None
}

/// Force-remove `rel` under `project_root`; a missing file is NOT an error
/// (idempotent, parity with the TS `rm(path, { force: true })`).
fn force_remove(project_root: &Path, rel: &str) -> Result<(), FspecCoreError> {
    let path = project_root.join(rel);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(FspecCoreError::Io {
            command: "remove-init-files",
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::{json, Value};
    use tempfile::TempDir;

    fn write_config(root: &Path, agent: &str) {
        let spec = root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(spec.join("fspec-config.json"), json!({ "agent": agent }).to_string())
            .unwrap();
    }

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, "").unwrap();
    }

    fn removed(out: &str) -> Vec<String> {
        let parsed: Value = serde_json::from_str(out).unwrap();
        parsed["filesRemoved"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn claude_removes_all_three() {
        let tmp = TempDir::new().unwrap();
        write_config(tmp.path(), "claude");
        touch(tmp.path(), "spec/CLAUDE.md");
        touch(tmp.path(), ".claude/commands/fspec.md");
        let out = run("{}", tmp.path()).await.unwrap();
        let r = removed(&out);
        assert!(r.contains(&"spec/CLAUDE.md".to_string()));
        assert!(r.contains(&".claude/commands/fspec.md".to_string()));
        assert!(r.contains(&"spec/fspec-config.json".to_string()));
        assert!(!tmp.path().join("spec/CLAUDE.md").exists());
    }

    #[tokio::test]
    async fn gemini_detected_by_directory() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".gemini")).unwrap();
        touch(tmp.path(), "spec/GEMINI.md");
        touch(tmp.path(), ".gemini/commands/fspec.toml");
        let out = run("{}", tmp.path()).await.unwrap();
        let r = removed(&out);
        assert!(r.contains(&"spec/GEMINI.md".to_string()));
        assert!(r.contains(&".gemini/commands/fspec.toml".to_string()));
    }

    #[tokio::test]
    async fn empty_config_agent_falls_through_to_detection() {
        // TS `if (config.agent)` is falsy for "" → file detection. The
        // .claude/ directory then resolves the claude agent (intended spec
        // behaviour), so removal succeeds.
        let tmp = TempDir::new().unwrap();
        write_config(tmp.path(), "");
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        touch(tmp.path(), "spec/CLAUDE.md");
        let out = run("{}", tmp.path()).await.unwrap();
        let r = removed(&out);
        assert!(r.contains(&"spec/CLAUDE.md".to_string()));
    }

    #[tokio::test]
    async fn keep_config_preserves() {
        let tmp = TempDir::new().unwrap();
        write_config(tmp.path(), "claude");
        touch(tmp.path(), "spec/CLAUDE.md");
        let out = run(r#"{"keepConfig":true}"#, tmp.path()).await.unwrap();
        let r = removed(&out);
        assert!(!r.contains(&"spec/fspec-config.json".to_string()));
        assert!(tmp.path().join("spec/fspec-config.json").exists());
    }

    #[tokio::test]
    async fn no_agent_errors() {
        let tmp = TempDir::new().unwrap();
        let err = run("{}", tmp.path()).await.unwrap_err();
        assert!(err
            .to_string()
            .contains("No fspec agent installation detected. Nothing to remove."));
    }

    #[tokio::test]
    async fn unknown_agent_errors() {
        let tmp = TempDir::new().unwrap();
        write_config(tmp.path(), "not-a-real-agent");
        let err = run("{}", tmp.path()).await.unwrap_err();
        assert!(err.to_string().contains("Unknown agent: not-a-real-agent"));
    }

    #[tokio::test]
    async fn idempotent_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        write_config(tmp.path(), "claude");
        touch(tmp.path(), ".claude/commands/fspec.md");
        let out = run("{}", tmp.path()).await.unwrap();
        let r = removed(&out);
        assert!(r.contains(&"spec/CLAUDE.md".to_string()));
    }
}
