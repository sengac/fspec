//! `add-aggregate-to-foundation` — Rust port of
//! `src/commands/add-aggregate-to-foundation.ts` (RPC-166).
//!
//! Appends an `aggregate` Event Storm item to `spec/foundation.json`'s
//! top-level `eventStorm.items` array during Big Picture Event Storming.
//! The aggregate is linked to an existing `bounded_context` item via
//! `boundedContextId`.
//!
//! ## Semantics (parity with TS)
//!
//! 1. `spec/foundation.json` is loaded-or-initialized through
//!    [`read_or_init_json`] using the TS command's INLINE slim default
//!    (`version/project/problemSpace/solutionSpace` only — empty-string
//!    fields, `projectType: "other"`, `impact: "medium"`). This is NOT
//!    `ensure_foundation_file`'s richer literal: the TS helper calls
//!    `fileManager.readJSON(path, defaults)` with its OWN minimal object
//!    (`src/commands/add-aggregate-to-foundation.ts:32-50`), so when the
//!    file is missing it is created with exactly that slim shape.
//! 2. The named bounded context is located by
//!    `type == "bounded_context" && text == contextName`. If absent →
//!    `"Bounded context '{contextName}' not found"` and NO write occurs.
//!    (The TS `add` path does NOT filter soft-deleted contexts.)
//! 3. On success the aggregate item is appended with
//!    `id = eventStorm.nextItemId`, `nextItemId` is post-incremented, and
//!    the whole document is written atomically.
//!
//! ## On-disk item shape & key order
//!
//! The TS object literal builds the aggregate as
//! `{ id, type, text, boundedContextId, color, deleted, createdAt,
//! [description] }`. We reproduce that exact insertion order using
//! `serde_json::Map` (the workspace builds `serde_json` with
//! `preserve_order`). `color` is the JSON string literal `"yellow"`
//! (Event Storming convention).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/add_aggregate_to_foundation.rs` is JSON marshalling
//! only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::{read_or_init_json, write_json_atomic};
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-aggregate-to-foundation`. Mirrors the TS
/// `AddAggregateToFoundationOptions` plus the two positional args
/// (`contextName`, `aggregateName`).
///
/// `contextName` and `aggregateName` are REQUIRED — supplying args JSON
/// missing either field surfaces a [`FspecCoreError::InvalidArgs`] (whose
/// `Display` contains `"Invalid args for fspec command
/// add-aggregate-to-foundation"`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddAggregateArgs {
    context_name: String,
    aggregate_name: String,
    #[serde(default)]
    description: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddAggregateArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-aggregate-to-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load-or-init foundation.json. The TS helper reads via
    // `fileManager.readJSON(path, default)` with an INLINE minimal default
    // (version/project/problemSpace/solutionSpace only — empty strings,
    // projectType "other", impact "medium"), so when the file is missing it
    // is created with exactly that slim shape. Mirror it here.
    let path = project_root.join("spec").join("foundation.json");
    let mut data: Value = read_or_init_json(&path, &foundation_read_default(), "foundation.json")?;

    // Locate the named bounded context. The TS code is:
    //   if (!data.eventStorm) { data.eventStorm = {items: [], ...}; }
    //   const bc = data.eventStorm.items.find(...);
    // So a FALSY `eventStorm` (false / 0 / null / "" / absent) is REINITIALIZED
    // to `{ items: [] }` — `items.find` then matches nothing → "not found".
    // A TRUTHY-but-non-object `eventStorm` (e.g. a string) is NOT reinit'd, so
    // `data.eventStorm.items` is `undefined` and `.find` throws the raw
    // `TypeError: Cannot read properties of undefined (reading 'find')`.
    let es_truthy = data.get("eventStorm").map(is_js_truthy).unwrap_or(false);
    let es_items_missing = data
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array)
        .is_none();
    if es_truthy && es_items_missing {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-aggregate-to-foundation",
            reason: "Cannot read properties of undefined (reading 'find')".to_string(),
        });
    }

    // The TS finds the bounded context by type+text only — a match with NO
    // `id` field is still a successful match (`boundedContext.id` is then
    // JS `undefined`). We must distinguish three states the TS object spread
    // collapses differently:
    //   - no bounded_context matched         → error
    //   - matched, `id` key ABSENT (undefined)→ success, omit boundedContextId
    //   - matched, `id` present (incl. null) → success, store id verbatim
    // The outer `Option` answers "matched?"; the inner `Option<Value>`
    // answers "did the matched context have an `id` key?".
    let found = data
        .get("eventStorm")
        .and_then(|es| es.get("items"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find(|i| {
                i.get("type").and_then(Value::as_str) == Some("bounded_context")
                    && i.get("text").and_then(Value::as_str) == Some(args.context_name.as_str())
            })
        })
        .map(|bc| bc.get("id").cloned());

    // `bounded_context_id` is `Some(value)` when the `id` key was present
    // (where `value` may itself be `Value::Null`), or `None` when the key was
    // absent (JS `undefined` → key omitted from the written item).
    let bounded_context_id: Option<Value> = match found {
        Some(id_opt) => id_opt,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-aggregate-to-foundation",
                reason: format!("Bounded context '{}' not found", args.context_name),
            });
        }
    };

    // Mutate: seed eventStorm if absent (success path always has it, but we
    // mirror the TS init for completeness), append item, bump nextItemId.
    let root_obj = match data.as_object_mut() {
        Some(o) => o,
        None => {
            return Err(FspecCoreError::ParseJson {
                file: "foundation.json".to_string(),
                reason: "top-level value must be a JSON object".to_string(),
            });
        }
    };

    let es_entry = root_obj
        .entry("eventStorm".to_string())
        .or_insert_with(default_event_storm);
    if !es_entry.is_object() {
        *es_entry = default_event_storm();
    }
    let es = es_entry
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: "eventStorm must be an object".to_string(),
        })?;

    // The TS object literal sets `id: data.eventStorm.nextItemId` then does
    // `data.eventStorm.nextItemId++`. When `nextItemId` is absent the value is
    // `undefined`, and `JSON.stringify` OMITS keys whose value is `undefined`
    // — so the `id` key disappears from the written item, and `undefined++`
    // becomes `NaN` → serialized as `null`. When present it may be any JS
    // number (including a float like `2.5`), which we model with f64 so the
    // round-trip and `+ 1` match exactly.
    let next_item_id = es.get("nextItemId").and_then(Value::as_f64);

    // Build the aggregate body in TS key order:
    // id, type, text, boundedContextId, color, deleted, createdAt, [description].
    let mut item = Map::new();
    if let Some(id) = next_item_id {
        item.insert("id".to_string(), number_value(id));
    }
    item.insert("type".to_string(), Value::String("aggregate".to_string()));
    item.insert(
        "text".to_string(),
        Value::String(args.aggregate_name.clone()),
    );
    // `boundedContextId: boundedContext.id`. When the matched context had no
    // `id` key (JS `undefined`), `JSON.stringify` OMITS the key entirely. When
    // the id was present (including an explicit `null`), it is written
    // verbatim. We mirror that by only inserting when the key existed.
    if let Some(id) = bounded_context_id {
        item.insert("boundedContextId".to_string(), id);
    }
    item.insert("color".to_string(), Value::String("yellow".to_string()));
    item.insert("deleted".to_string(), Value::Bool(false));
    item.insert("createdAt".to_string(), Value::String(iso8601_now()));
    // TS uses `...(options.description && { description })`. Because `&&`
    // treats an empty string as falsy, an empty `--description ""` is
    // OMITTED entirely (only a non-empty string adds the key).
    if let Some(desc) = args.description.as_deref() {
        if !desc.is_empty() {
            item.insert("description".to_string(), Value::String(desc.to_string()));
        }
    }

    let items = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items.is_array() {
        *items = Value::Array(Vec::new());
    }
    if let Value::Array(arr) = items {
        arr.push(Value::Object(item));
    }

    // TS does `data.eventStorm.nextItemId++`. When `nextItemId` was a number
    // it becomes `n + 1`; when it was `undefined`, `undefined++` evaluates to
    // `NaN`, which `JSON.stringify` writes as `null`.
    match next_item_id {
        Some(id) => {
            es.insert("nextItemId".to_string(), number_value(id + 1.0));
        }
        None => {
            es.insert("nextItemId".to_string(), Value::Null);
        }
    }

    // Single atomic write of the full document — unknown top-level keys are
    // preserved verbatim.
    write_json_atomic(&path, &data)?;

    // Auto-regenerate FOUNDATION.md after updating foundation.json,
    // mirroring the TS `await generateFoundationMdCommand({ cwd })` call.
    crate::commands::generate_foundation_md::regenerate(project_root);

    let message = format!(
        "Added aggregate \"{}\" to \"{}\" bounded context",
        args.aggregate_name, args.context_name
    );
    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "add-aggregate-to-foundation",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Canonical empty `eventStorm` section as initialized by the TS command
/// (`level = "big_picture"`, `items = []`, `nextItemId = 1`).
fn default_event_storm() -> Value {
    json!({
        "level": "big_picture",
        "items": [],
        "nextItemId": 1
    })
}

/// Mirror JavaScript truthiness for a `serde_json::Value` in the narrow set
/// of shapes a `foundation.json` field can hold. The TS guard
/// `if (!data.eventStorm)` treats `false`, `0`, `null`, and `""` as falsy;
/// everything else (objects, arrays, non-empty strings, non-zero numbers) is
/// truthy. `undefined` (absent key) is handled by the caller's `Option`.
fn is_js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Build a JSON number `Value` from an `f64` the way `JSON.stringify` would:
/// an integral value (e.g. `3.0`) serializes WITHOUT a fractional part
/// (`3`), while a true float (`2.5`) keeps it. Falls back to `Null` only for
/// the impossible non-finite case (NaN/Inf can't reach here — `as_f64`
/// already filtered them out at read time).
fn number_value(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 {
        // Integral: emit as i64 so the on-disk form is `3`, not `3.0`.
        Value::from(n as i64)
    } else {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// Inline minimal default passed to `fileManager.readJSON` by the TS
/// `addAggregateToFoundation` helper
/// (`src/commands/add-aggregate-to-foundation.ts:32-50`). This is
/// DELIBERATELY narrower than `ensureFoundationFile`'s literal — it only
/// seeds version/project/problemSpace/solutionSpace, in this exact key
/// order, with empty-string fields (`projectType: "other"`,
/// `impact: "medium"`) and NO `personas`/`architectureDiagrams`. Only
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
    fn args_parse_camel_case_minimal() {
        let a: AddAggregateArgs =
            serde_json::from_str(r#"{"contextName":"Sales","aggregateName":"Order"}"#).unwrap();
        assert_eq!(a.context_name, "Sales");
        assert_eq!(a.aggregate_name, "Order");
        assert!(a.description.is_none());
    }

    #[test]
    fn args_parse_with_description() {
        let a: AddAggregateArgs = serde_json::from_str(
            r#"{"contextName":"Billing","aggregateName":"Invoice","description":"root"}"#,
        )
        .unwrap();
        assert_eq!(a.description.as_deref(), Some("root"));
    }

    #[test]
    fn args_missing_context_name_fails() {
        let r: Result<AddAggregateArgs, _> = serde_json::from_str(r#"{"aggregateName":"Order"}"#);
        assert!(r.is_err());
    }

    #[test]
    fn is_js_truthy_matches_javascript_falsy_set() {
        assert!(!is_js_truthy(&Value::Null));
        assert!(!is_js_truthy(&json!(false)));
        assert!(!is_js_truthy(&json!(0)));
        assert!(!is_js_truthy(&json!("")));
        assert!(is_js_truthy(&json!(true)));
        assert!(is_js_truthy(&json!(1)));
        assert!(is_js_truthy(&json!("x")));
        assert!(is_js_truthy(&json!([])));
        assert!(is_js_truthy(&json!({})));
    }

    #[test]
    fn number_value_integral_serializes_without_fraction() {
        assert_eq!(serde_json::to_string(&number_value(3.0)).unwrap(), "3");
        assert_eq!(serde_json::to_string(&number_value(2.5)).unwrap(), "2.5");
    }
}
