//! `validate-hooks` — Rust port of `src/commands/validate-hooks.ts` (RPC-322).
//!
//! Validates the hook configuration at `spec/fspec-hooks.json` and verifies
//! that every configured hook's `command` script exists on disk.
//!
//! ## Framing A — single-envelope result (RPC-247 list-hooks precedent)
//!
//! The dispatcher entry point returns a JSON envelope
//! `{valid, exitCode, message, errors?}`. The standalone Rust CLI bridge
//! (`rust/fspec/src/validate_hooks.rs`) prints `message` and exits with
//! `exitCode`. ALL rendering decisions live in the core message so both
//! front doors (dispatcher + clap subcommand) agree byte-for-byte.
//!
//! ## Outcomes (parity with the TS programmatic API + help-fixture rendering)
//!   - config missing →
//!     `{valid:false, exitCode:1, message:"Failed to load hook configuration"}`
//!   - config broken (BUG-611) → `{valid:false, exitCode:1, message:<the
//!     not-changed error naming the problem>}`; the file is never changed
//!   - `hooks` object configures zero hooks →
//!     `{valid:true, exitCode:0, message:"No hooks configured (nothing to validate)"}`
//!   - one or more `command` scripts missing on disk →
//!     `{valid:false, exitCode:1, message:"✗ Hook validation failed\n\n<lines>\n\nFix…", errors:[…]}`
//!   - every script exists →
//!     `{valid:true, exitCode:0, message:"✓ All hooks are valid"}`
//!
//! ## Two-front-doors
//! Both the LLM dispatcher AND the clap subcommand call this single
//! function (RPC-003 §7/§11). The CLI bridge is JSON marshalling + a thin
//! `print message; exit exitCode` shim with NO inline validation logic.

use std::path::Path;

use serde_json::{json, Value};

use crate::command_hooks::config_file::load_for_read;
use crate::error::FspecCoreError;

/// Dispatcher entry point. `args_json` carries no flags for this command.
pub async fn run(_args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    // ---- Load config: missing → load failure, broken → its message ----
    let config = match load_for_read(project_root, "validate-hooks") {
        Ok(Some(config)) => config,
        Ok(None) => return load_failure(),
        Err(error) => {
            return ok(json!({
                "valid": false,
                "exitCode": 1,
                "message": error.to_string(),
            }))
        }
    };
    let empty = serde_json::Map::new();
    let hooks = config
        .get("hooks")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    // ---- Walk every event → hook, checking each command script exists ----
    let mut errors: Vec<String> = Vec::new();
    let mut total_hooks = 0usize;
    for (_event, list) in hooks {
        let Some(arr) = list.as_array() else { continue };
        for hook in arr {
            total_hooks += 1;
            let command = hook.get("command").and_then(Value::as_str).unwrap_or("");
            let hook_path = project_root.join(command);
            if !hook_path.exists() {
                errors.push(format!("Hook command not found: {command}"));
            }
        }
    }

    // ---- No hooks configured ----
    if total_hooks == 0 {
        return ok(json!({
            "valid": true,
            "exitCode": 0,
            "message": "No hooks configured (nothing to validate)",
        }));
    }

    // ---- Missing scripts ----
    if !errors.is_empty() {
        let message = format!(
            "✗ Hook validation failed\n\n{}\n\nFix these issues before using hooks.",
            errors.join("\n")
        );
        return ok(json!({
            "valid": false,
            "exitCode": 1,
            "message": message,
            "errors": errors,
        }));
    }

    // ---- All valid ----
    ok(json!({
        "valid": true,
        "exitCode": 0,
        "message": "✓ All hooks are valid",
    }))
}

/// The load-failure envelope for a missing config.
fn load_failure() -> Result<String, FspecCoreError> {
    ok(json!({
        "valid": false,
        "exitCode": 1,
        "message": "Failed to load hook configuration",
    }))
}

/// Serialise an envelope value to the `Ok(String)` returned to the dispatcher.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate-hooks",
        reason: format!("failed to serialise response: {e}"),
    })
}
