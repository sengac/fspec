//! `add-command-to-foundation` — Rust port of
//! `src/commands/add-command-to-foundation.ts` (RPC-175).
//!
//! Appends a `command` Event Storm item to the foundation's Big Picture
//! `eventStorm.items` array, linking it to an existing `bounded_context`
//! by name. Mirrors the TS `addCommandToFoundation` helper.
//!
//! ## Semantics (parity with TS)
//!
//! 1. Load `spec/foundation.json` via `read_or_init_json` with the TS
//!    inline minimal default (version/project/problemSpace/solutionSpace
//!    only — NOT `ensure_foundation_file`'s richer literal), matching the
//!    TS `readJSON(path, default)` call which creates that exact shape when
//!    the file is missing.
//! 2. Seed `eventStorm` (`level=big_picture`, `items=[]`, `nextItemId=1`)
//!    if absent — matching the TS initializer.
//! 3. Find the `bounded_context` item whose `text` equals `contextName`.
//!    Missing → `"Bounded context '{name}' not found"` and NO write.
//! 4. On success: append a `command` item with `id = nextItemId`,
//!    post-increment `nextItemId`, write atomically.
//!
//! ## On-disk item shape & key order
//!
//! The TS object literal fixes key insertion order as
//! `id, type, text, boundedContextId, color, deleted, createdAt,
//! [description]`. We reproduce that order with `serde_json::Map` (the
//! workspace builds `serde_json` with `preserve_order`). `color` is the
//! JSON string literal `"blue"` (Event Storming convention).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/add_command_to_foundation.rs` is JSON marshalling
//! only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-command-to-foundation`. Mirrors the TS
/// `AddCommandToFoundationOptions` plus the two positional arguments.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddCommandToFoundationArgs {
    context_name: String,
    command_name: String,
    #[serde(default)]
    description: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddCommandToFoundationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-command-to-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load-or-init foundation.json. The TS helper reads via
    // `fileManager.readJSON(path, default)` with an INLINE minimal default
    // (version/project/problemSpace/solutionSpace only — NOT the richer
    // `ensureFoundationFile` literal), so when the file is missing it is
    // created with exactly that shape. Mirror it here.
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    let root = data
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "top-level value must be a JSON object".to_string(),
        })?;

    // [2] Seed eventStorm if absent (TS initializes with nextItemId: 1).
    let es_entry = root
        .entry("eventStorm".to_string())
        .or_insert_with(seed_event_storm);
    if !es_entry.is_object() {
        *es_entry = seed_event_storm();
    }
    let es = es_entry
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "eventStorm must be an object".to_string(),
        })?;

    // Ensure items is an array before scanning/appending.
    let items_entry = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items_entry.is_array() {
        *items_entry = Value::Array(Vec::new());
    }

    // [3] Find the bounded context by name (TS does NOT filter deleted on
    // the add path). Capture its stable id, then drop the borrow.
    let bounded_context_id = es.get("items").and_then(Value::as_array).and_then(|arr| {
        arr.iter()
            .find(|i| {
                i.get("type").and_then(Value::as_str) == Some("bounded_context")
                    && i.get("text").and_then(Value::as_str) == Some(args.context_name.as_str())
            })
            .and_then(|i| i.get("id").and_then(Value::as_u64))
    });

    let bounded_context_id = match bounded_context_id {
        Some(id) => id,
        None => {
            // No write on failure — the file remains byte-identical.
            return Err(FspecCoreError::InvalidArgs {
                command: "add-command-to-foundation",
                reason: format!("Bounded context '{}' not found", args.context_name),
            });
        }
    };

    // [4] Build the command item in TS key order, append, bump counter.
    let item_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

    let mut item: Map<String, Value> = Map::new();
    item.insert("id".to_string(), Value::from(item_id));
    item.insert("type".to_string(), Value::String("command".to_string()));
    item.insert("text".to_string(), Value::String(args.command_name.clone()));
    item.insert(
        "boundedContextId".to_string(),
        Value::from(bounded_context_id),
    );
    item.insert("color".to_string(), Value::String("blue".to_string()));
    item.insert("deleted".to_string(), Value::Bool(false));
    item.insert("createdAt".to_string(), Value::String(iso8601_now()));
    if let Some(desc) = args.description.as_deref() {
        item.insert("description".to_string(), Value::String(desc.to_string()));
    }

    if let Some(arr) = es.get_mut("items").and_then(Value::as_array_mut) {
        arr.push(Value::Object(item));
    }
    es.insert("nextItemId".to_string(), Value::from(item_id + 1));

    // Single atomic write of the full document — preserves unknown
    // top-level keys and existing ordering byte-for-byte.
    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let message = format!(
        "Added command \"{}\" to \"{}\" bounded context",
        args.command_name, args.context_name
    );
    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "add-command-to-foundation",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Canonical empty Big Picture Event Storm section (TS seeds nextItemId=1).
fn seed_event_storm() -> Value {
    json!({
        "level": "big_picture",
        "items": [],
        "nextItemId": 1
    })
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `addCommandToFoundation` helper (`src/commands/add-command-to-foundation.ts`
/// lines 32-50). This is DELIBERATELY narrower than `ensureFoundationFile`'s
/// literal — it only seeds version/project/problemSpace/solutionSpace, in this
/// exact key order, and contains NO `personas` or `architectureDiagrams`. Only
/// written when `spec/foundation.json` is missing.
fn foundation_read_default() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "",
            "vision": "",
            "projectType": "other"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "",
                "description": "",
                "impact": "medium"
            }
        },
        "solutionSpace": {
            "overview": "",
            "capabilities": []
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_minimal() {
        let a: AddCommandToFoundationArgs = serde_json::from_str(
            r#"{"contextName":"Work Management","commandName":"CreateWorkUnit"}"#,
        )
        .unwrap();
        assert_eq!(a.context_name, "Work Management");
        assert_eq!(a.command_name, "CreateWorkUnit");
        assert!(a.description.is_none());
    }

    #[test]
    fn args_parse_with_description() {
        let a: AddCommandToFoundationArgs =
            serde_json::from_str(r#"{"contextName":"X","commandName":"Y","description":"d"}"#)
                .unwrap();
        assert_eq!(a.description.as_deref(), Some("d"));
    }
}
