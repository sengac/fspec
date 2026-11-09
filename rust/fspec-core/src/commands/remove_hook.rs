//! `remove-hook` — Rust port of `src/commands/remove-hook.ts` (RPC-275).
//!
//! Removes all entries matching `name` from the array at `hooks[event]`
//! in `spec/fspec-hooks.json`.
//!
//! ## Semantics
//!
//! 1. Read `spec/fspec-hooks.json` through
//!    [`crate::command_hooks::config_file::load_for_write`]. A missing file
//!    is an `FspecCoreError::Io` error (nothing to remove from, no file is
//!    created). A broken file (BUG-611) is backed up to
//!    `spec/fspec-hooks.json.broken-<timestamp>`, a fresh `{"hooks": {}}`
//!    config is written, and the backup notice is the command's output.
//! 2. If the event key exists, retain only entries whose `name != arg.name`.
//!    All duplicate matches are dropped in a single pass.
//! 3. Empty array after removal is **retained** — the event key is NOT
//!    deleted.
//! 4. Missing event key OR no-match name → no mutation, but the file is
//!    still rewritten. The rewrite is idempotent.
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

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::command_hooks::config_file::{load_for_write, ForWrite};
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
/// Returns the BUG-611 backup notice when a broken config was backed up,
/// and an empty string otherwise.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    run_at(args_json, project_root, chrono::Utc::now()).await
}

/// [`run`] at `now`, the time a broken config's backup is named after.
pub async fn run_at(
    args_json: &str,
    project_root: &Path,
    now: DateTime<Utc>,
) -> Result<String, FspecCoreError> {
    let args: RemoveHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = crate::io::spec_paths::hooks_config_of(project_root);

    // BUG-477: hold the spec lock from the first read to the last write.
    let _spec_lock = crate::io::spec_lock::lock_spec_dir(project_root, "remove-hook")?;
    let (mut config, notice) = match load_for_write(project_root, "remove-hook", now)? {
        ForWrite::Missing(source) => {
            return Err(FspecCoreError::Io {
                command: "remove-hook",
                source,
            })
        }
        ForWrite::Config(config) => (config, String::new()),
        ForWrite::BackedUp { config, notice } => (config, notice),
    };

    // Retain-filter on hooks[event]; load_for_write guarantees `hooks`, when
    // present, is an object of arrays. Missing `hooks`, missing event key,
    // or no-match name → no-op. Empty-after-filter is RETAINED.
    if let Some(arr) = config
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .and_then(|hooks| hooks.get_mut(&args.event))
        .and_then(Value::as_array_mut)
    {
        arr.retain(|entry| {
            entry
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|n| n != args.name)
        });
    }

    write_json_atomic(&path, &Value::Object(config))?;

    Ok(notice)
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
