//! `add-hook` — Rust port of `src/commands/add-hook.ts` (RPC-184).
//!
//! Appends a single hook entry to `spec/fspec-hooks.json`, creating the
//! file on first call.
//!
//! ## Semantics
//!
//! 1. Read `spec/fspec-hooks.json` through
//!    [`crate::command_hooks::config_file::load_for_write`]: a missing file
//!    starts a fresh `{"hooks": {}}` config; a broken file (BUG-611) is
//!    backed up to `spec/fspec-hooks.json.broken-<timestamp>` and a fresh
//!    config is started, and the backup notice is the command's output.
//! 2. Ensure the `hooks` object exists at the top level (insertion order
//!    preserved relative to any sibling top-level fields such as `global`).
//! 3. Push a new hook entry `{ name, command, blocking[, timeout] }` onto
//!    `hooks[event]`, inserting the event's array when it is missing.
//!    `timeout` is OMITTED when not supplied.
//! 4. `std::fs::create_dir_all(spec/)` then `write_json_atomic(spec/fspec-hooks.json, &config)`.
//!
//! ## Two-front-doors contract
//!
//! Both the LLM-facing dispatcher AND the standalone fspec Rust binary
//! call `run(args_json, project_root)`. The CLI bridge at
//! `rust/fspec/src/add_hook.rs` performs only argv→JSON marshalling.
//!
//! ## Unknown-field preservation (and top-level ORDER preservation)
//!
//! Everything outside the `hooks` object — including a top-level
//! `"global": { ... }` block that may appear **before** `hooks` in the
//! original on-disk JSON — round-trips with exact positional fidelity. We
//! achieve this by representing the on-disk shape as a `Map<String, Value>`
//! (backed by `IndexMap` thanks to the workspace-wide `serde_json`
//! `preserve_order` feature) and walking it directly, rather than via a
//! named-field struct + `#[serde(flatten)]` (which always serialises named
//! fields BEFORE the flattened map and therefore lost positional parity
//! with TS when `global` was declared first on disk).

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::command_hooks::config_file::{empty_config, load_for_write, ForWrite};
use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments for `add-hook`. Mirrors the TS `AddHookOptions`
/// interface at `src/commands/add-hook.ts:11-18`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AddHookArgs {
    event: String,
    name: String,
    command: String,
    #[serde(default)]
    blocking: bool,
    timeout: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

/// Dispatcher entry point.
///
/// Returns `✓ Hook added: <event>/<name>` (BUG-638), after the BUG-611
/// backup notice when a broken config was backed up.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    run_at(args_json, project_root, chrono::Utc::now()).await
}

/// [`run`] at `now`, the time a broken config's backup is named after.
pub async fn run_at(
    args_json: &str,
    project_root: &Path,
    now: DateTime<Utc>,
) -> Result<String, FspecCoreError> {
    let args: AddHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    // BUG-590: reject an event no hook ever runs on, before reading or
    // writing spec/fspec-hooks.json.
    // The reason is the whole message: the args themselves are well formed.
    crate::command_hooks::check_hook_event(&args.event).map_err(FspecCoreError::Message)?;

    let path = crate::io::spec_paths::hooks_config_of(project_root);

    // BUG-477: hold the spec lock from the first read to the last write.
    let _spec_lock = crate::io::spec_lock::lock_spec_dir(project_root, "add-hook")?;
    let (mut config, notice) = match load_for_write(project_root, "add-hook", now)? {
        ForWrite::Missing(_) => (empty_config(), String::new()),
        ForWrite::Config(config) => (config, String::new()),
        ForWrite::BackedUp { config, notice } => (config, notice),
    };

    // load_for_write guarantees `hooks`, when present, is an object of
    // arrays; insert it when absent, keeping any sibling order.
    let hooks = config
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let event_hooks = hooks
        .as_object_mut()
        .map(|hooks| {
            hooks
                .entry(args.event.clone())
                .or_insert_with(|| Value::Array(Vec::new()))
        })
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            FspecCoreError::Message(format!("internal: hooks.{} is not an array", args.event))
        })?;

    let added = format!("✓ Hook added: {}/{}", args.event, args.name);
    let mut entry: Map<String, Value> = Map::new();
    entry.insert("name".to_string(), Value::String(args.name));
    entry.insert("command".to_string(), Value::String(args.command));
    entry.insert("blocking".to_string(), Value::Bool(args.blocking));
    if let Some(timeout) = args.timeout {
        entry.insert("timeout".to_string(), json!(timeout));
    }
    event_hooks.push(Value::Object(entry));

    // Ensure spec/ exists, then atomic write.
    let spec_dir = crate::io::spec_paths::spec_dir_of(project_root);
    std::fs::create_dir_all(&spec_dir).map_err(|source| FspecCoreError::Io {
        command: "add-hook",
        source,
    })?;
    write_json_atomic(&path, &Value::Object(config))?;

    Ok(if notice.is_empty() {
        added
    } else {
        format!("{notice}\n{added}")
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_with_minimal_fields() {
        let json = r#"{"event":"pre","name":"n","command":"c.sh","blocking":false}"#;
        let a: AddHookArgs = serde_json::from_str(json).unwrap();
        assert_eq!(a.event, "pre");
        assert_eq!(a.name, "n");
        assert_eq!(a.command, "c.sh");
        assert!(!a.blocking);
        assert!(a.timeout.is_none());
    }

    #[test]
    fn args_parse_with_timeout() {
        let json = r#"{"event":"pre","name":"n","command":"c.sh","blocking":true,"timeout":300}"#;
        let a: AddHookArgs = serde_json::from_str(json).unwrap();
        assert!(a.blocking);
        assert_eq!(a.timeout, Some(300));
    }
}
