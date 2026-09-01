//! `remove-aggregate-from-foundation` — Rust port of
//! `src/commands/remove-aggregate-from-foundation.ts` (RPC-266).
//!
//! Soft-deletes an `aggregate` Event Storm item in
//! `spec/foundation.json`'s top-level `eventStorm.items` array. The
//! aggregate is matched by name AND its `boundedContextId` link to the
//! named (non-deleted) bounded context.
//!
//! ## Semantics (parity with TS)
//!
//! 1. `spec/foundation.json` is loaded-or-initialized through
//!    [`read_or_init_json`] using the TS command's INLINE slim default
//!    (`version/project/problemSpace/solutionSpace` only — empty-string
//!    fields, `projectType: "other"`, `impact: "medium"`). This mirrors the
//!    TS `fileManager.readJSON(path, defaults)` call
//!    (`src/commands/remove-aggregate-from-foundation.ts:34-45`), NOT
//!    `ensure_foundation_file`'s richer literal.
//! 2. If there is no `eventStorm` section at all →
//!    `"Bounded context '{contextName}' not found (no Event Storm data)"`.
//! 3. The bounded context is located by
//!    `type == "bounded_context" && text == contextName && !deleted`.
//!    If absent → `"Bounded context '{contextName}' not found"`.
//! 4. The aggregate is located by
//!    `type == "aggregate" && text == aggregateName && !deleted &&
//!    boundedContextId == <context id>`. If absent →
//!    `"Aggregate '{aggregateName}' not found in bounded context
//!    '{contextName}'"`.
//! 5. On success the matched item's `deleted` is set to `true` (the item
//!    is NOT removed from the array) and the document is written
//!    atomically.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/remove_aggregate_from_foundation.rs` is JSON
//! marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::foundation::guidance;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};

/// CLI arguments accepted by `remove-aggregate-from-foundation`. Both
/// `contextName` and `aggregateName` are REQUIRED.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveAggregateArgs {
    context_name: String,
    aggregate_name: String,
}

/// `true` when the item is NOT soft-deleted (`deleted` missing or false).
fn is_live(item: &Value) -> bool {
    item.get("deleted").and_then(Value::as_bool) != Some(true)
}

/// Mirror JavaScript truthiness for the `eventStorm` field. The TS guard
/// `if (!data.eventStorm)` treats `false`, `0`, `null`, and `""` as falsy;
/// objects, arrays, non-empty strings, and non-zero numbers are truthy.
/// `undefined` (absent key) is handled by the caller's `Option`.
fn is_js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveAggregateArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-aggregate-from-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load-or-init foundation.json with the TS command's INLINE slim default
    // (`src/commands/remove-aggregate-from-foundation.ts:34-45`). When the
    // file is missing it is created with exactly that slim shape (which has
    // no `eventStorm`), so the no-Event-Storm error path below fires.
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    // [2] No (truthy) eventStorm section. The TS guard is
    // `if (!data.eventStorm)`, which treats a FALSY value (`false` / `0` /
    // `null` / `""`) the same as an absent key → "no Event Storm data".
    let es_truthy = data.get("eventStorm").map(is_js_truthy).unwrap_or(false);
    if !es_truthy {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-aggregate-from-foundation",
            reason: format!(
                "Bounded context '{}' not found (no Event Storm data)",
                args.context_name
            ),
        });
    }

    // [2b] eventStorm is truthy but has no `items` array (e.g. a string or an
    // object without `items`). The `!data.eventStorm` guard passed, so
    // `data.eventStorm.items.find` dereferences `undefined` and throws the raw
    // `TypeError: Cannot read properties of undefined (reading 'find')`.
    if data
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array)
        .is_none()
    {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-aggregate-from-foundation",
            reason: "Cannot read properties of undefined (reading 'find')".to_string(),
        });
    }

    let items = data
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    // [3] Locate the live bounded context by name. A match with NO `id` key
    // is still a successful match (JS `boundedContext.id` is then
    // `undefined`). The outer `Option` answers "matched?"; the inner
    // `Option<Value>` is "did the matched context carry an `id` key?".
    let found_ctx = items
        .iter()
        .find(|i| {
            i.get("type").and_then(Value::as_str) == Some("bounded_context")
                && i.get("text").and_then(Value::as_str) == Some(args.context_name.as_str())
                && is_live(i)
        })
        .map(|bc| bc.get("id").cloned());

    let bounded_context_id: Option<Value> = match found_ctx {
        Some(id_opt) => id_opt,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-aggregate-from-foundation",
                reason: format!("Bounded context '{}' not found", args.context_name),
            });
        }
    };

    // [4] Locate the live aggregate scoped to that context. The TS predicate
    // is `'boundedContextId' in item && item.boundedContextId === boundedContext.id`:
    //   - the `in` check requires the aggregate to HAVE a `boundedContextId` key;
    //   - the `===` is strict JS equality (no coercion). When the context id
    //     was `undefined` (no `id` key), `=== undefined` only holds if the
    //     aggregate's `boundedContextId` is itself `undefined` — but the `in`
    //     check already required the key be present, so an `undefined` context
    //     id can NEVER match. We model that: if the context id key was absent,
    //     no aggregate matches.
    let target_idx = items.iter().position(|i| {
        i.get("type").and_then(Value::as_str) == Some("aggregate")
            && i.get("text").and_then(Value::as_str) == Some(args.aggregate_name.as_str())
            && is_live(i)
            && match (i.get("boundedContextId"), bounded_context_id.as_ref()) {
                // aggregate has the key AND context had a concrete id → strict eq
                (Some(agg_id), Some(ctx_id)) => agg_id == ctx_id,
                // aggregate lacks the key (`in` is false) → never matches
                // context id was undefined → `=== undefined` after `in` → never
                _ => false,
            }
    });

    let target_idx = match target_idx {
        Some(idx) => idx,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-aggregate-from-foundation",
                reason: format!(
                    "Aggregate '{}' not found in bounded context '{}'",
                    args.aggregate_name, args.context_name
                ),
            });
        }
    };

    // [5] Soft-delete: set deleted = true on the matched item in place.
    let item = data
        .get_mut("eventStorm")
        .and_then(|es| es.get_mut("items"))
        .and_then(Value::as_array_mut)
        .and_then(|arr| arr.get_mut(target_idx))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "eventStorm.items must be an array of objects".to_string(),
        })?;
    item.insert("deleted".to_string(), Value::Bool(true));

    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let message = format!(
        "Removed aggregate \"{}\" from \"{}\" bounded context",
        args.aggregate_name, args.context_name
    );
    // DISC-003 rule 4: event-storm trailer on the success envelope.
    let next_steps = guidance::event_storm_trailer(&data, args.context_name.as_str());
    serde_json::to_string(&json!({
        "success": true,
        "message": message,
        "nextSteps": next_steps,
    }))
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-aggregate-from-foundation",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `removeAggregateFromFoundation` helper
/// (`src/commands/remove-aggregate-from-foundation.ts:34-45`). DELIBERATELY
/// narrower than `ensureFoundationFile`'s literal — only seeds
/// version/project/problemSpace/solutionSpace (empty-string fields,
/// `projectType: "other"`, `impact: "medium"`), NO `personas`/
/// `architectureDiagrams`. Only written when `spec/foundation.json` is
/// missing.
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
    fn args_parse_camel_case() {
        let a: RemoveAggregateArgs =
            serde_json::from_str(r#"{"contextName":"Sales","aggregateName":"Order"}"#).unwrap();
        assert_eq!(a.context_name, "Sales");
        assert_eq!(a.aggregate_name, "Order");
    }

    #[test]
    fn args_missing_context_name_fails() {
        let r: Result<RemoveAggregateArgs, _> =
            serde_json::from_str(r#"{"aggregateName":"Order"}"#);
        assert!(r.is_err());
    }

    #[test]
    fn is_live_treats_missing_deleted_as_live() {
        assert!(is_live(&json!({"type": "aggregate"})));
        assert!(is_live(&json!({"deleted": false})));
        assert!(!is_live(&json!({"deleted": true})));
    }

    #[test]
    fn is_js_truthy_matches_javascript_falsy_set() {
        assert!(!is_js_truthy(&Value::Null));
        assert!(!is_js_truthy(&json!(false)));
        assert!(!is_js_truthy(&json!(0)));
        assert!(!is_js_truthy(&json!("")));
        assert!(is_js_truthy(&json!(true)));
        assert!(is_js_truthy(&json!("x")));
        assert!(is_js_truthy(&json!({})));
    }
}
