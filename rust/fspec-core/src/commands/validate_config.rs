//! `validate-config` — Rust-only extension command (CONFIG-009, DISC-003
//! pattern like `foundation-status`).
//!
//! Feature: spec/features/config-scope-validation.feature
//!
//! Codifies the config-scope key-ownership rule: user-scope
//! `~/.fspec/fspec-config.json` holds user-owned state (`providers`, `tui.*`,
//! `rlcd`, optionally `research` credentials); project-scope
//! `spec/fspec-config.json` holds repo-owned state (`tools`, `agent`,
//! optionally `research`). A key whose only writer targets one scope must
//! never appear in the other scope's file — so `tools`/`agent` in the
//! USER-scope file is contamination (the BUG-193 leak) and is flagged.
//!
//! ## Envelope (two front doors agree byte-for-byte)
//!
//! `{valid, exitCode, message, warnings[]}`:
//!   - user file parses cleanly, no project-only keys →
//!     `{valid:true, exitCode:0, message:"✓ User-scope config is clean", warnings:[]}`
//!   - user file parses but contains `tools`/`agent` → same valid=true /
//!     exitCode=0 with a per-key warning + remediation text (hygiene issue,
//!     not a correctness break — nothing reads these keys from user scope).
//!   - user file missing or empty → clean (nothing to validate).
//!   - user file malformed JSON → `{valid:true, exitCode:0,
//!     message:"⚠ <path> contains malformed JSON ..." , warnings:[...]}` —
//!     a reported parse warning, never a crash.
//!
//! Read-only: never writes config. The user dir resolves from the
//! `FSPEC_USER_DIR` env override first, then `HOME`/`.fspec` — so tests
//! redirect `FSPEC_USER_DIR` to a temp dir and a developer's real
//! `~/.fspec` is never read or touched.

use std::path::Path;

use serde_json::{json, Value};

use crate::error::FspecCoreError;

/// Top-level keys that must live in the PROJECT scope only. A key whose only
/// writer targets `<cwd>/spec/fspec-config.json` (configure-tools for
/// `tools`, `fspec init` for `agent`) is contamination when found in the
/// user-scope file. `research` is deliberately NOT in this set — it has a
/// legitimate user-scope fallback (reminders.rs reads project-first).
const PROJECT_ONLY_KEYS: &[&str] = &["tools", "agent"];

/// Remediation text per project-only key (surfaced in the warning + message).
fn remediation_for(key: &str) -> &'static str {
    match key {
        "tools" => {
            "Remove `tools` from your user-scope config; configure it per-project with `fspec configure-tools` (writes spec/fspec-config.json)."
        }
        "agent" => {
            "Remove `agent` from your user-scope config; it is set per-project by `fspec init`."
        }
        // Unreachable for the current PROJECT_ONLY_KEYS set; keep a stable
        // generic line if the set ever grows.
        _ => "Remove this project-scope key from your user-scope config.",
    }
}

/// Dispatcher entry point. `args_json` carries no flags for this command.
pub async fn run(_args_json: &str, _project_root: &Path) -> Result<String, FspecCoreError> {
    let user_dir = user_dir();
    let config_path = user_dir.join("fspec-config.json");

    // ---- Resolve the user-scope file contents (read-only) ----
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(_) => {
            // Missing file (or unreadable dir): nothing to validate.
            return ok(json!({
                "valid": true,
                "exitCode": 0,
                "message": "✓ User-scope config is clean (no user-scope config file found)",
                "warnings": []
            }));
        }
    };

    if raw.trim().is_empty() {
        return ok(json!({
            "valid": true,
            "exitCode": 0,
            "message": "✓ User-scope config is clean (no user-scope config file found)",
            "warnings": []
        }));
    }

    // ---- Parse; malformed JSON is a reported warning, never a crash ----
    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            let message = format!(
                "⚠ {} contains malformed JSON ({}). Fix the file or remove it.",
                config_path.display(),
                err
            );
            return ok(json!({
                "valid": true,
                "exitCode": 0,
                "message": message,
                "warnings": [message]
            }));
        }
    };

    let Some(top) = value.as_object() else {
        // A valid JSON scalar/array at the top level is not a config object;
        // report it as a parse-shape warning instead of flagging keys.
        let message = format!(
            "⚠ {} must be a JSON object (found {}); no project-scope keys were checked.",
            config_path.display(),
            value_type_name(&value)
        );
        return ok(json!({
            "valid": true,
            "exitCode": 0,
            "message": message,
            "warnings": [message]
        }));
    };

    // ---- Flag project-only keys present in the user scope ----
    let mut warnings: Vec<String> = Vec::new();
    for key in PROJECT_ONLY_KEYS {
        if top.contains_key(*key) {
            warnings.push(format!(
                "`{key}` is a project-scope key: {}",
                remediation_for(key)
            ));
        }
    }

    if warnings.is_empty() {
        return ok(json!({
            "valid": true,
            "exitCode": 0,
            "message": "✓ User-scope config is clean (no project-scope keys found)",
            "warnings": []
        }));
    }

    let message = format!(
        "⚠ User-scope config has {} project-scope key(s):\n\n{}",
        warnings.len(),
        warnings
            .iter()
            .map(|w| format!("  - {w}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    ok(json!({
        "valid": true,
        "exitCode": 0,
        "message": message,
        "warnings": warnings
    }))
}

/// Resolve the user-scope dir: `FSPEC_USER_DIR` override first (tests),
/// then `$HOME/.fspec` (Windows: `USERPROFILE`). Returns a relative
/// `.fspec` when no home can be resolved — the (almost certainly absent)
/// file then simply validates as clean, matching the sibling commands'
/// graceful-degrade convention.
fn user_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("FSPEC_USER_DIR") {
        if !dir.is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    match home {
        Some(h) => std::path::PathBuf::from(h).join(".fspec"),
        None => std::path::PathBuf::from(".fspec"),
    }
}

/// Short human name for a JSON top-level value (warning text only).
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Serialise an envelope value to the `Ok(String)` returned to the
/// dispatcher.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate-config",
        reason: format!("failed to serialise response: {e}"),
    })
}
