//! `show-foundation-event-storm` — Rust port of
//! `src/commands/show-foundation-event-storm.ts` (RPC-306).
//!
//! Reads `spec/foundation.json` and emits the Event Storm items array
//! (structural filtering only — no semantic interpretation).
//!
//! ## Behaviour parity with TypeScript (`src/commands/show-foundation-event-storm.ts`)
//!
//! * `spec/foundation.json` missing → `Err(InvalidArgs)` whose Display
//!   contains the substring `"foundation.json"` (so CLI stderr / dispatcher
//!   error envelope can be substring-asserted).
//! * `foundation.json` present but `eventStorm` field absent →
//!   `Ok(envelope)` with `data: []`, `message: "No Event Storm data in
//!   foundation.json"`.
//! * Soft-deleted items (`deleted: true`) are filtered out before any
//!   other filter runs.
//! * `context` filter (by `text` of a `bounded_context` item):
//!     - matched → keep the BC itself plus every item whose
//!       `boundedContextId == bc.id`.
//!     - unmatched → `Err(InvalidArgs)` naming the available bounded
//!       context names (DISC-003: a typo'd context must not silently
//!       return an empty array).
//! * `type` filter (post-context) → keep items whose `type` field equals.
//!
//! ## Envelope shape
//!
//! Always 2-space pretty JSON with `success`, `data`, `items` (aliased to
//! `data`), and optionally `message`. The dispatcher serializes this
//! payload verbatim; the CLI bridge extracts `data` and re-prints it as
//! a top-level 2-space JSON array (parity with `JSON.stringify(result.data, null, 2)`).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;

/// CLI / dispatcher arguments accepted by `show-foundation-event-storm`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowArgs {
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    context: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-foundation-event-storm",
            reason: format!("failed to parse args: {e}"),
        })?;

    let foundation_path = project_root.join("spec").join("foundation.json");
    if !foundation_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "show-foundation-event-storm",
            reason: format!("foundation.json not found at {}", foundation_path.display()),
        });
    }

    let raw = std::fs::read_to_string(&foundation_path).map_err(|source| FspecCoreError::Io {
        command: "show-foundation-event-storm",
        source,
    })?;

    let foundation: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "foundation.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    // `eventStorm` missing → empty data + canonical message.
    let event_storm = match foundation.get("eventStorm") {
        Some(v) if v.is_object() => v,
        _ => {
            let envelope = json!({
                "success": true,
                "data": [],
                "message": "No Event Storm data in foundation.json",
            });
            return serde_json_to_string(&envelope);
        }
    };

    let items_raw = match event_storm.get("items").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            let envelope = json!({
                "success": true,
                "data": [],
                "items": [],
            });
            return serde_json_to_string(&envelope);
        }
    };

    // Filter out soft-deleted items (structural filter only).
    let mut items: Vec<Value> = items_raw
        .iter()
        .filter(|item| !matches!(item.get("deleted"), Some(d) if d.as_bool() == Some(true)))
        .cloned()
        .collect();

    // Context filter.
    if let Some(ctx) = args.context.as_deref() {
        // Find the bounded_context with text == ctx.
        let bc = items.iter().find(|item| {
            item.get("type").and_then(|v| v.as_str()) == Some("bounded_context")
                && item.get("text").and_then(|v| v.as_str()) == Some(ctx)
        });

        match bc {
            Some(bc_owned) => {
                let bc_id = bc_owned.get("id").cloned().unwrap_or(Value::Null);
                items.retain(|item| {
                    let is_self = item.get("type").and_then(|v| v.as_str())
                        == Some("bounded_context")
                        && item.get("text").and_then(|v| v.as_str()) == Some(ctx);
                    if is_self {
                        return true;
                    }
                    match item.get("boundedContextId") {
                        Some(v) => v == &bc_id,
                        None => false,
                    }
                });
            }
            // DISC-003 rule 10: an unmatched context name is almost always a
            // typo — error with the available bounded context names instead
            // of silently returning an empty array.
            None => {
                let available: Vec<String> = items
                    .iter()
                    .filter(|item| {
                        item.get("type").and_then(|v| v.as_str()) == Some("bounded_context")
                    })
                    .filter_map(|item| item.get("text").and_then(|v| v.as_str()))
                    .map(str::to_string)
                    .collect();
                return Err(FspecCoreError::InvalidArgs {
                    command: "show-foundation-event-storm",
                    reason: format!(
                        "Unknown context '{ctx}'. Available bounded contexts: {}",
                        if available.is_empty() {
                            "(none)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ),
                });
            }
        }
    }

    // Type filter.
    if let Some(ty) = args.r#type.as_deref() {
        items.retain(|item| item.get("type").and_then(|v| v.as_str()) == Some(ty));
    }

    let envelope = json!({
        "success": true,
        "data": items,
        "items": items,
    });
    serde_json_to_string(&envelope)
}

fn serde_json_to_string(v: &Value) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(v).map_err(|e| FspecCoreError::InvalidArgs {
        command: "show-foundation-event-storm",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: ShowArgs = serde_json::from_str("{}").unwrap();
        assert!(a.r#type.is_none());
        assert!(a.context.is_none());
    }

    #[test]
    fn args_parse_type_and_context() {
        let a: ShowArgs =
            serde_json::from_str(r#"{"type":"aggregate","context":"Work Management"}"#).unwrap();
        assert_eq!(a.r#type.as_deref(), Some("aggregate"));
        assert_eq!(a.context.as_deref(), Some("Work Management"));
    }
}
