//! `remove-hook` — Rust port of `src/commands/remove-hook.ts` (RPC-275).
//!
//! Removes all entries matching `name` from the array at `hooks[event]`
//! in `spec/fspec-hooks.json`. Mirrors the TS `removeHook` implementation
//! at `src/commands/remove-hook.ts:17-35` including the **divergent**
//! error-handling contract:
//!
//! ## Critical divergence from `add-hook`
//!
//! Unlike `add-hook`, `remove-hook` does **NOT** swallow read/parse errors.
//!
//! * `std::fs::read_to_string` failure → `FspecCoreError::Io { command:
//!   "remove-hook", source }`. ENOENT propagates; no auto-create.
//! * `serde_json::from_str` failure → `FspecCoreError::ParseJson { file:
//!   "fspec-hooks.json", reason }`. The on-disk bytes are LEFT UNCHANGED.
//!
//! This matches the TS file at lines 21-22: the `await readFile(...)` +
//! `JSON.parse(...)` calls run WITHOUT a try/catch, so an ENOENT or a parse
//! error propagates out of `removeHook()` and is reported by the CLI/agent
//! layer.
//!
//! ## Semantics (mirrors src/commands/remove-hook.ts:24-29)
//!
//! 1. Read+parse `spec/fspec-hooks.json` (errors propagate per above).
//! 2. If the event key exists, retain only entries whose `name != arg.name`.
//!    All duplicate matches are dropped in a single pass.
//! 3. Empty array after removal is **retained** — the event key is NOT
//!    deleted. (TS `config.hooks[event] = […].filter(...)` does NOT delete
//!    the key when the array becomes empty.)
//! 4. Missing event key OR no-match name → no mutation, but the file is
//!    still rewritten (mirrors TS unconditional `fileManager.transaction`
//!    at line 32-34). The rewrite is idempotent.
//! 5. `write_json_atomic(spec/fspec-hooks.json, &config)` to persist.
//!
//! ## Unknown-field preservation (and top-level ORDER preservation)
//!
//! Top-level extras (e.g. `global:{timeout:30}` declared BEFORE `hooks` on
//! disk) round-trip with exact positional fidelity. We represent the
//! on-disk shape as `Map<String, Value>` (backed by `IndexMap` thanks to
//! the workspace-wide `serde_json` `preserve_order` feature) and walk it
//! directly. A named-field struct with `#[serde(flatten)]` would always
//! serialise `hooks` BEFORE the flattened map and break parity with TS for
//! files that have `global` declared first.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments for `remove-hook`. Mirrors the TS `RemoveHookOptions`
/// interface at `src/commands/remove-hook.ts:11-15`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveHookArgs {
    event: String,
    name: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

/// Dispatcher entry point.
///
/// Returns `Ok(String::new())` on success — `remove-hook` has no rendered
/// output, mirroring the TS `Commander.js` action handler which prints
/// nothing on success.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("fspec-hooks.json");

    // Read+parse with hard-error propagation (DIVERGES from add-hook).
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "remove-hook",
        source,
    })?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "fspec-hooks.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    // Top-level must be a JSON object — preserve its key insertion order.
    let mut config = match parsed {
        Value::Object(map) => map,
        _ => {
            return Err(FspecCoreError::ParseJson {
                file: "fspec-hooks.json".to_string(),
                reason: "top-level value must be a JSON object".to_string(),
            });
        }
    };

    // Retain-filter on hooks[event]. Missing `hooks`, missing event key,
    // or no-match name → silent no-op. Empty-after-filter is RETAINED.
    if let Some(hooks_val) = config.get_mut("hooks") {
        if let Some(hooks_obj) = hooks_val.as_object_mut() {
            if let Some(arr_val) = hooks_obj.get_mut(&args.event) {
                if let Some(arr) = arr_val.as_array_mut() {
                    arr.retain(|entry| {
                        entry
                            .get("name")
                            .and_then(Value::as_str)
                            .is_none_or(|n| n != args.name)
                    });
                }
            }
        }
    }

    // Persist. The file existed by precondition (read_to_string above
    // already succeeded), so spec/ is already present.
    let value = Value::Object(config);
    write_json_atomic(&path, &value)?;

    Ok(String::new())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_minimal() {
        let json = r#"{"event":"pre","name":"lint"}"#;
        let a: RemoveHookArgs = serde_json::from_str(json).unwrap();
        assert_eq!(a.event, "pre");
        assert_eq!(a.name, "lint");
    }

    #[test]
    fn retain_filter_drops_all_matching_names() {
        let raw = r#"{"hooks":{"pre":[{"name":"a","command":"a.sh","blocking":false},{"name":"a","command":"a2.sh","blocking":true},{"name":"b","command":"b.sh","blocking":false}]}}"#;
        let mut config: serde_json::Map<String, Value> = match serde_json::from_str(raw).unwrap() {
            Value::Object(m) => m,
            _ => panic!("expected object"),
        };
        let arr = config
            .get_mut("hooks")
            .and_then(Value::as_object_mut)
            .and_then(|h| h.get_mut("pre"))
            .and_then(Value::as_array_mut)
            .unwrap();
        arr.retain(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|n| n != "a")
        });
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "b");
    }

    #[test]
    fn empty_array_after_filter_is_retained() {
        let raw = r#"{"hooks":{"pre":[{"name":"a","command":"a.sh","blocking":false}]}}"#;
        let mut config: serde_json::Map<String, Value> = match serde_json::from_str(raw).unwrap() {
            Value::Object(m) => m,
            _ => panic!("expected object"),
        };
        let arr = config
            .get_mut("hooks")
            .and_then(Value::as_object_mut)
            .and_then(|h| h.get_mut("pre"))
            .and_then(Value::as_array_mut)
            .unwrap();
        arr.retain(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|n| n != "a")
        });
        assert!(arr.is_empty());
        // Event key must still exist on the hooks object.
        assert!(config["hooks"].as_object().unwrap().contains_key("pre"));
    }

    #[test]
    fn preserves_top_level_sibling_field_order() {
        let raw = r#"{"global":{"timeout":30},"hooks":{"pre":[{"name":"a","command":"a.sh","blocking":false}]},"zzz":1}"#;
        let parsed: Value = serde_json::from_str(raw).unwrap();
        let map = match parsed {
            Value::Object(m) => m,
            _ => panic!("expected object"),
        };
        let keys: Vec<&String> = map.keys().collect();
        assert_eq!(keys, vec!["global", "hooks", "zzz"]);
    }

    #[test]
    fn preserves_per_entry_extras_on_retained_entries() {
        let raw = r#"{"hooks":{"pre":[{"name":"a","command":"a.sh","blocking":true,"timeout":120,"condition":{"tags":["@x"]}},{"name":"b","command":"b.sh","blocking":false}]}}"#;
        let mut config: serde_json::Map<String, Value> = match serde_json::from_str(raw).unwrap() {
            Value::Object(m) => m,
            _ => panic!("expected object"),
        };
        let arr = config
            .get_mut("hooks")
            .and_then(Value::as_object_mut)
            .and_then(|h| h.get_mut("pre"))
            .and_then(Value::as_array_mut)
            .unwrap();
        arr.retain(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|n| n != "b")
        });
        let s = serde_json::to_string(&arr[0]).unwrap();
        assert!(
            s.contains("\"condition\""),
            "extras must round-trip; got {s}"
        );
        assert!(s.contains("\"timeout\":120"));
    }
}
