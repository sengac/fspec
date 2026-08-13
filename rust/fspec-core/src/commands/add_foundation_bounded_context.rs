//! `add-foundation-bounded-context` — Rust port of
//! `src/commands/add-foundation-bounded-context.ts` (RPC-183).
//!
//! Appends a `bounded_context` item to the foundation-level Big Picture
//! Event Storm — `spec/foundation.json`'s top-level `eventStorm.items`
//! array (level `big_picture`). This is the STRATEGIC counterpart to the
//! work-unit-level `add-bounded-context` (Process Modeling), which targets
//! `spec/work-units.json`.
//!
//! ## Semantics (parity with TS)
//!
//! 1. Load-or-init `spec/foundation.json`. The TS command reads via
//!    `fileManager.readJSON(path, defaults)` with its OWN INLINE slim default
//!    (version/project/problemSpace/solutionSpace only — empty strings,
//!    `projectType: "other"`, `impact: "medium"`, NO
//!    `personas`/`architectureDiagrams`). When the file is missing it is
//!    created with exactly that slim shape — NOT the richer
//!    `ensure_foundation_file` literal. Mirror the slim default here.
//! 2. Seed the `eventStorm` sub-object when absent:
//!    `{ level: "big_picture", items: [], nextItemId: 1 }`. NOTE the
//!    foundation-level counter starts at **1** (the work-unit-level
//!    Event Storm starts at 0).
//! 3. Append a `bounded_context` item with `id = nextItemId`,
//!    post-increment `nextItemId`, write atomically.
//!
//! ## On-disk item shape & key order
//!
//! The TS command builds the object literal
//! `{ id, type, text, color, deleted, createdAt }` — `id` FIRST (unlike
//! the work-unit helper which appends `id` last). We reproduce that exact
//! insertion order using `serde_json::Map` (the workspace builds
//! `serde_json` with `preserve_order`). `color` is the JSON literal
//! `null`.
//!
//! ## Framing A divergence from TypeScript
//!
//! * **FOUNDATION.md regeneration**: the TS command calls
//!   `generateFoundationMdCommand` after the write. `generate-foundation-md`
//!   (RPC-233) is itself unported, so the Rust core does NOT touch
//!   `spec/FOUNDATION.md`. NOTE: the TS command does NOT print any
//!   "Regenerated" line (it invokes `generateFoundationMdCommand`, whose
//!   result is discarded — only `generateFoundationMdCommandCLI` prints).
//!   So the CLI bridge emits ONLY the `✓ <message>` line, matching TS.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/add_foundation_bounded_context.rs` is JSON
//! marshalling only.
//!
//! ## Args (camelCase JSON)
//!
//! `{ "text": String }`

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};
use crate::io::time::iso8601_now;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddFoundationBoundedContextArgs {
    text: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddFoundationBoundedContextArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-foundation-bounded-context",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load-or-init foundation.json. The TS command passes its OWN inline
    // slim default to `fileManager.readJSON`, so a missing file is created
    // with that minimal shape (NOT the richer ensure_foundation_file
    // literal).
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    let root_obj = data
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "top-level value must be a JSON object".to_string(),
        })?;

    // Seed eventStorm when absent (or coerce a non-object shape).
    let es_entry = root_obj
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

    // Next stable id (foundation-level counter starts at 1).
    let item_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(1);

    // Build the item in TS object-literal order: id, type, text, color,
    // deleted, createdAt.
    let mut item: Map<String, Value> = Map::new();
    item.insert("id".to_string(), Value::from(item_id));
    item.insert(
        "type".to_string(),
        Value::String("bounded_context".to_string()),
    );
    item.insert("text".to_string(), Value::String(args.text.clone()));
    item.insert("color".to_string(), Value::Null);
    item.insert("deleted".to_string(), Value::Bool(false));
    item.insert("createdAt".to_string(), Value::String(iso8601_now()));

    let items = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items.is_array() {
        *items = Value::Array(Vec::new());
    }
    if let Value::Array(arr) = items {
        arr.push(Value::Object(item));
    }

    es.insert("nextItemId".to_string(), Value::from(item_id + 1));

    // Atomic write — preserves all unknown top-level fields verbatim.
    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let message = format!(
        "Added bounded context \"{}\" to foundation Event Storm",
        args.text
    );
    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "add-foundation-bounded-context",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Canonical fresh `eventStorm` sub-object for the foundation Big Picture
/// level. Counter starts at 1 (parity with TS `nextItemId: 1`).
fn seed_event_storm() -> Value {
    let mut es = Map::new();
    es.insert(
        "level".to_string(),
        Value::String("big_picture".to_string()),
    );
    es.insert("items".to_string(), Value::Array(Vec::new()));
    es.insert("nextItemId".to_string(), Value::from(1u64));
    Value::Object(es)
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `addFoundationBoundedContext` helper
/// (`src/commands/add-foundation-bounded-context.ts:33-54`). DELIBERATELY
/// narrower than `ensureFoundationFile`'s literal: only seeds
/// version/project/problemSpace/solutionSpace, in this exact key order, with
/// empty-string fields (`projectType: "other"`, `impact: "medium"`) and NO
/// `personas`/`architectureDiagrams`. Only written when
/// `spec/foundation.json` is missing.
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
        let a: AddFoundationBoundedContextArgs =
            serde_json::from_str(r#"{"text":"Work Management"}"#).unwrap();
        assert_eq!(a.text, "Work Management");
    }

    #[test]
    fn seed_has_big_picture_level_and_counter_one() {
        let es = seed_event_storm();
        assert_eq!(es["level"].as_str(), Some("big_picture"));
        assert_eq!(es["nextItemId"].as_u64(), Some(1));
        assert!(es["items"].as_array().unwrap().is_empty());
    }
}
