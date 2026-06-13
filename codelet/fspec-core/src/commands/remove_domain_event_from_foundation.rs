//! `remove-domain-event-from-foundation` — Rust port of
//! `src/commands/remove-domain-event-from-foundation.ts` (RPC-272).
//!
//! Soft-deletes an `event` Event Storm item (sets `deleted: true`) within a
//! named `bounded_context` in the foundation's Big Picture `eventStorm.items`
//! array. Mirrors the TS `removeDomainEventFromFoundation` helper. Twin:
//! `remove_command_from_foundation.rs` (RPC-270); the only domain diffs are
//! that the matched item `type` is `event`, the not-found noun is
//! "Domain event", and the success-message noun is "domain event".
//!
//! ## Semantics (parity with TS)
//!
//! 1. Load `spec/foundation.json` via `read_or_init_json` with the TS
//!    inline minimal default (NOT `ensure_foundation_file`).
//! 2. If the document has NO `eventStorm` field →
//!    `"Bounded context '{name}' not found (no Event Storm data)"` and NO
//!    write.
//! 3. Find the non-deleted `bounded_context` whose `text` equals
//!    `contextName`. Missing → `"Bounded context '{name}' not found"`.
//! 4. Find the non-deleted `event` whose `text` equals `eventName`
//!    AND whose `boundedContextId` equals the matched context's `id`.
//!    Missing → `"Domain event '{event}' not found in bounded context
//!    '{ctx}'"`.
//! 5. Set that event's `deleted` to `true`, write atomically.
//!
//! On any failure path the file is left byte-identical (no write).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/remove_domain_event_from_foundation.rs` is JSON
//! marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};

/// CLI arguments accepted by `remove-domain-event-from-foundation`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveDomainEventFromFoundationArgs {
    context_name: String,
    event_name: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveDomainEventFromFoundationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-domain-event-from-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load-or-init foundation.json. The TS helper reads via
    // `fileManager.readJSON(path, default)` with an INLINE minimal default
    // (version/project/problemSpace/solutionSpace only — NOT the richer
    // `ensureFoundationFile` literal), so a missing file is created with
    // exactly that shape. Mirror it here.
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    let root = data
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "top-level value must be a JSON object".to_string(),
        })?;

    // [2] No eventStorm field → canonical "no Event Storm data" error.
    let es = match root.get_mut("eventStorm").and_then(Value::as_object_mut) {
        Some(es) => es,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-domain-event-from-foundation",
                reason: format!(
                    "Bounded context '{}' not found (no Event Storm data)",
                    args.context_name
                ),
            });
        }
    };

    let items = match es.get("items").and_then(Value::as_array) {
        Some(arr) => arr,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-domain-event-from-foundation",
                reason: format!("Bounded context '{}' not found", args.context_name),
            });
        }
    };

    // [3] Find the non-deleted bounded context by name; capture its id.
    let bounded_context_id = items
        .iter()
        .find(|i| {
            i.get("type").and_then(Value::as_str) == Some("bounded_context")
                && i.get("text").and_then(Value::as_str) == Some(args.context_name.as_str())
                && !i.get("deleted").and_then(Value::as_bool).unwrap_or(false)
        })
        .and_then(|i| i.get("id").and_then(Value::as_u64));

    let bounded_context_id = match bounded_context_id {
        Some(id) => id,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-domain-event-from-foundation",
                reason: format!("Bounded context '{}' not found", args.context_name),
            });
        }
    };

    // [4] Find the matching, non-deleted domain event within that context.
    let event_index = items.iter().position(|i| {
        i.get("type").and_then(Value::as_str) == Some("event")
            && i.get("text").and_then(Value::as_str) == Some(args.event_name.as_str())
            && !i.get("deleted").and_then(Value::as_bool).unwrap_or(false)
            && i.get("boundedContextId").and_then(Value::as_u64) == Some(bounded_context_id)
    });

    let event_index = match event_index {
        Some(idx) => idx,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-domain-event-from-foundation",
                reason: format!(
                    "Domain event '{}' not found in bounded context '{}'",
                    args.event_name, args.context_name
                ),
            });
        }
    };

    // [5] Soft-delete the matched domain event.
    if let Some(arr) = es.get_mut("items").and_then(Value::as_array_mut) {
        if let Some(obj) = arr.get_mut(event_index).and_then(Value::as_object_mut) {
            obj.insert("deleted".to_string(), Value::Bool(true));
        }
    }

    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let message = format!(
        "Removed domain event \"{}\" from \"{}\" bounded context",
        args.event_name, args.context_name
    );
    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "remove-domain-event-from-foundation",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `removeDomainEventFromFoundation` helper
/// (`src/commands/remove-domain-event-from-foundation.ts` lines 34-45).
/// DELIBERATELY narrower than `ensureFoundationFile`'s literal — only seeds
/// version/project/problemSpace/solutionSpace in this exact key order, with NO
/// `personas` or `architectureDiagrams`. Only written when
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
        let a: RemoveDomainEventFromFoundationArgs = serde_json::from_str(
            r#"{"contextName":"Work Management","eventName":"WorkUnitCreated"}"#,
        )
        .unwrap();
        assert_eq!(a.context_name, "Work Management");
        assert_eq!(a.event_name, "WorkUnitCreated");
    }
}
