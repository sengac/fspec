//! `add-hook` — Rust port of `src/commands/add-hook.ts` (RPC-184).
//!
//! Appends a single hook entry to `spec/fspec-hooks.json`, creating the
//! file on first call. Mirrors the TS `addHook` implementation byte-for-byte
//! including the "swallow read/parse errors and start from scratch"
//! semantics produced by the bare `try { ... } catch { config = {hooks: {}} }`
//! at `src/commands/add-hook.ts:26-32`.
//!
//! ## Semantics (mirrors src/commands/add-hook.ts:20-54)
//!
//! 1. Try to read+parse `spec/fspec-hooks.json`. **Either** an IO failure
//!    (ENOENT or otherwise) **OR** a `serde_json::from_str` parse failure
//!    silently resets the in-memory config to a fresh default whose only
//!    key is `"hooks": {}` — the "TS bare catch" branch.
//! 2. Ensure the `hooks` object exists at the top level (insertion order
//!    preserved relative to any sibling top-level fields such as `global`).
//! 3. If the event key is missing from `hooks`, insert an empty array
//!    (preserving insertion order — `serde_json` is configured with the
//!    `preserve_order` feature workspace-wide so `Map<String, Value>` is
//!    backed by `IndexMap`).
//! 4. Push a new hook entry `{ name, command, blocking[, timeout] }` onto
//!    that array. `timeout` is OMITTED when not supplied — matching the TS
//!    `JSON.stringify(undefined) === undefined` semantic.
//! 5. `std::fs::create_dir_all(spec/)` then `write_json_atomic(spec/fspec-hooks.json, &config)`.
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

use serde::Deserialize;
use serde_json::{json, Map, Value};

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
/// Returns `Ok(String::new())` on success — `add-hook` has no rendered
/// output, mirroring the TS `Commander.js` action handler which prints
/// nothing on success.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("fspec-hooks.json");

    // Load: TS bare-catch parity. Either an IO failure OR a parse failure
    // resets the in-memory config to a fresh default whose only key is
    // `"hooks": {}`.
    let mut config = load_or_default(&path);

    // Ensure the `hooks` key exists at the top level. If absent we INSERT
    // it (preserving any pre-existing sibling order), and if present we
    // leave its position untouched.
    if !config.contains_key("hooks") {
        config.insert("hooks".to_string(), Value::Object(Map::new()));
    }
    // Coerce a non-object `hooks` shape into an empty object to avoid
    // panicking on garbage data — mirrors the TS spread-clone behaviour.
    let hooks_entry = match config.get_mut("hooks") {
        Some(v) if v.is_object() => v,
        Some(v) => {
            *v = Value::Object(Map::new());
            v
        }
        // Unreachable: we just inserted "hooks" above. Bail with a
        // structured error rather than panic to satisfy clippy::expect_used.
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "fspec-hooks.json".to_string(),
                reason: "internal: failed to materialise hooks object".to_string(),
            });
        }
    };
    let hooks_obj = match hooks_entry.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "fspec-hooks.json".to_string(),
                reason: "internal: hooks is not an object after coercion".to_string(),
            });
        }
    };

    // Initialise the event array if absent — Map<String, Value>'s
    // backing IndexMap preserves insertion order so a new event key is
    // appended at the tail.
    let event_entry = hooks_obj
        .entry(args.event.clone())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !event_entry.is_array() {
        *event_entry = Value::Array(Vec::new());
    }
    let event_arr = match event_entry.as_array_mut() {
        Some(a) => a,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "fspec-hooks.json".to_string(),
                reason: format!(
                    "internal: hooks['{}'] is not an array after coercion",
                    args.event
                ),
            });
        }
    };

    // Build the hook entry. `timeout` is omitted when absent — matches
    // `JSON.stringify(undefined) === undefined`.
    let mut entry: Map<String, Value> = Map::new();
    entry.insert("name".to_string(), Value::String(args.name));
    entry.insert("command".to_string(), Value::String(args.command));
    entry.insert("blocking".to_string(), Value::Bool(args.blocking));
    if let Some(timeout) = args.timeout {
        entry.insert("timeout".to_string(), json!(timeout));
    }
    event_arr.push(Value::Object(entry));

    // Ensure spec/ exists, then atomic write.
    let spec_dir = project_root.join("spec");
    std::fs::create_dir_all(&spec_dir).map_err(|source| FspecCoreError::Io {
        command: "add-hook",
        source,
    })?;

    let value = Value::Object(config);
    write_json_atomic(&path, &value)?;

    Ok(String::new())
}

/// "Swallow everything" loader — mirrors the TS bare `try { ... } catch
/// { config = {hooks: {}} }` at `src/commands/add-hook.ts:26-32`. Either
/// branch (IO or parse error) returns the default empty config WITHOUT
/// writing to disk; the caller's subsequent atomic write OVERWRITES any
/// pre-existing malformed bytes.
///
/// On success, returns the parsed top-level `Map<String, Value>` exactly
/// as deserialised — preserving key insertion order so any sibling keys
/// (e.g. `global`) keep their original position relative to `hooks`.
fn load_or_default(path: &Path) -> Map<String, Value> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return default_config(),
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(map)) => map,
        _ => default_config(),
    }
}

/// Fresh in-memory config used on read/parse failure. Mirrors the TS
/// `config = { hooks: {} }` reset.
fn default_config() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("hooks".to_string(), Value::Object(Map::new()));
    m
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

    #[test]
    fn load_or_default_returns_empty_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("missing.json");
        let cfg = load_or_default(&p);
        assert_eq!(cfg.keys().collect::<Vec<_>>(), vec!["hooks"]);
        assert!(cfg["hooks"].as_object().unwrap().is_empty());
    }

    #[test]
    fn load_or_default_returns_empty_on_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("bad.json");
        std::fs::write(&p, "{ not json").unwrap();
        let cfg = load_or_default(&p);
        assert_eq!(cfg.keys().collect::<Vec<_>>(), vec!["hooks"]);
    }

    #[test]
    fn load_or_default_preserves_sibling_field_order() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("hooks.json");
        std::fs::write(
            &p,
            r#"{"global":{"timeout":30},"hooks":{"pre":[]},"zzz":1}"#,
        )
        .unwrap();
        let cfg = load_or_default(&p);
        let keys: Vec<&String> = cfg.keys().collect();
        assert_eq!(keys, vec!["global", "hooks", "zzz"]);
    }
}
