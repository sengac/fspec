//! `remove-foundation-bounded-context` — Rust port of
//! `src/commands/remove-foundation-bounded-context.ts` (RPC-274).
//!
//! SOFT-deletes a `bounded_context` item from the foundation-level Big
//! Picture Event Storm (`spec/foundation.json`'s `eventStorm.items`),
//! optionally cascading the soft-delete to child items that carry the
//! context's `boundedContextId`. Mirrors the `ItemWithId` soft-delete
//! convention (`deleted: true`, never spliced).
//!
//! ## Semantics (parity with TS)
//!
//! 1. Load (or init with TS's slim inline default) `spec/foundation.json`.
//! 2. No `eventStorm` field → error
//!    `Bounded context '{name}' not found (no Event Storm data)`.
//! 3. Find the FIRST non-deleted `bounded_context` whose `text == name`.
//!    None → error `Bounded context '{name}' not found`. An already
//!    soft-deleted context is therefore treated as not found.
//! 4. Count non-deleted child items carrying `boundedContextId == ctx.id`.
//!    If `> 0` and `--cascade` was NOT supplied → refuse with
//!    `Bounded context '{name}' has {n} child items. Use --cascade to
//!    remove the context and all its children.` (no disk write).
//! 5. Set the context's `deleted = true`; when `--cascade`, also set every
//!    matched child's `deleted = true`. Single atomic write.
//!
//! ## Atomicity / no-write-on-error
//!
//! All validation runs against an in-memory `serde_json::Value` BEFORE any
//! disk write, so every error path leaves `spec/foundation.json`
//! byte-for-byte unchanged (parity with the TS `fileManager.transaction`
//! throw-before-commit contract).
//!
//! ## Framing A divergence from TypeScript
//!
//! * **FOUNDATION.md regeneration**: `generate-foundation-md` (RPC-233) is
//!   unported, so the Rust core does NOT touch `spec/FOUNDATION.md`. NOTE:
//!   the TS command does NOT print any "Regenerated" line (it invokes
//!   `generateFoundationMdCommand`, whose result is discarded). So the CLI
//!   bridge emits ONLY the `✓ <message>` line, matching TS.
//!
//! ## Two-front-doors
//!
//! Both the LLM dispatcher and the standalone Rust binary's clap subcommand
//! call this single function. The CLI bridge is JSON marshalling only.
//!
//! ## Args (camelCase JSON)
//!
//! `{ "contextName": String, "cascade"?: bool }`

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveFoundationBoundedContextArgs {
    context_name: String,
    #[serde(default)]
    cascade: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveFoundationBoundedContextArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-foundation-bounded-context",
            reason: format!("failed to parse args: {e}"),
        })?;

    let name = args.context_name.as_str();

    // Load-or-init foundation.json. The TS command reads via
    // `fileManager.readJSON(path, defaults)` with its OWN INLINE slim default
    // (version/project/problemSpace/solutionSpace only; NO
    // personas/architectureDiagrams). A missing file is created with that
    // minimal shape (then the "no Event Storm data" error fires).
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    // [2] No eventStorm field → not found (no Event Storm data).
    let items = match data
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array)
    {
        Some(arr) => arr,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-foundation-bounded-context",
                reason: format!("Bounded context '{name}' not found (no Event Storm data)"),
            });
        }
    };

    // [3] Locate the first non-deleted bounded_context with matching text.
    let bc_index = items.iter().position(|i| {
        i.get("type").and_then(Value::as_str) == Some("bounded_context")
            && i.get("text").and_then(Value::as_str) == Some(name)
            && !i.get("deleted").and_then(Value::as_bool).unwrap_or(false)
    });
    let bc_index = match bc_index {
        Some(idx) => idx,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-foundation-bounded-context",
                reason: format!("Bounded context '{name}' not found"),
            });
        }
    };

    let bc_id = items[bc_index].get("id").cloned().unwrap_or(Value::Null);

    // [4] Collect non-deleted children carrying boundedContextId == bc_id.
    let child_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, i)| {
            !i.get("deleted").and_then(Value::as_bool).unwrap_or(false)
                && i.get("boundedContextId")
                    .map(|v| *v == bc_id)
                    .unwrap_or(false)
        })
        .map(|(idx, _)| idx)
        .collect();

    if !child_indices.is_empty() && !args.cascade {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-foundation-bounded-context",
            reason: format!(
                "Bounded context '{name}' has {} child items. \
                 Use --cascade to remove the context and all its children.",
                child_indices.len()
            ),
        });
    }

    // [5] Mutate: soft-delete the context (+ children when cascading).
    let arr = data
        .get_mut("eventStorm")
        .and_then(|es| es.get_mut("items"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "eventStorm.items must be an array".to_string(),
        })?;

    if let Some(obj) = arr[bc_index].as_object_mut() {
        obj.insert("deleted".to_string(), Value::Bool(true));
    }
    if args.cascade {
        for idx in &child_indices {
            if let Some(obj) = arr[*idx].as_object_mut() {
                obj.insert("deleted".to_string(), Value::Bool(true));
            }
        }
    }

    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let cascade_msg = if args.cascade {
        " and all its children"
    } else {
        ""
    };
    let message =
        format!("Removed bounded context \"{name}\"{cascade_msg} from foundation Event Storm");
    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "remove-foundation-bounded-context",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `removeFoundationBoundedContext` helper
/// (`src/commands/remove-foundation-bounded-context.ts:37-48`). DELIBERATELY
/// narrower than `ensureFoundationFile`'s literal: only seeds
/// version/project/problemSpace/solutionSpace, in this exact key order, with
/// empty-string fields (`projectType: "other"`, `impact: "medium"`) and NO
/// `personas`/`architectureDiagrams`. Only written when
/// `spec/foundation.json` is missing — after which the "no Event Storm data"
/// error fires (since this default has no `eventStorm`).
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
    fn args_parse_minimal_defaults_no_cascade() {
        let a: RemoveFoundationBoundedContextArgs =
            serde_json::from_str(r#"{"contextName":"Sales"}"#).unwrap();
        assert_eq!(a.context_name, "Sales");
        assert!(!a.cascade);
    }

    #[test]
    fn args_parse_with_cascade() {
        let a: RemoveFoundationBoundedContextArgs =
            serde_json::from_str(r#"{"contextName":"Sales","cascade":true}"#).unwrap();
        assert!(a.cascade);
    }
}
