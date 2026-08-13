//! `add-policy` — Rust port of `src/commands/add-policy.ts` (RPC-187).
//!
//! Appends an Event Storm `policy` item (color `purple`) to a work unit's
//! `eventStorm.items` array. Mirrors the shared-utility behaviour of the
//! TypeScript `addEventStormItem` helper (`src/commands/event-storm-utils.ts`)
//! that `add-policy.ts` delegates to:
//!
//! * Work unit must exist — otherwise `Work unit <id> not found`.
//! * Work unit must NOT be in `done`/`blocked` state — otherwise
//!   `Cannot add Event Storm items to work unit in <status> state`.
//! * On first add, the `eventStorm` sub-object is seeded with
//!   `{ level: "process_modeling", items: [], nextItemId: 0 }`.
//! * The new item's `id` is the current `nextItemId`; `nextItemId` is then
//!   post-incremented. `deleted` defaults to `false`; `createdAt` is a fresh
//!   ISO-8601 timestamp.
//!
//! ## On-disk item field order (TS object-literal insertion order)
//!
//! The TS source builds `itemData = { type, color, text }` then conditionally
//! appends `when`, `then`, `timestamp`, `boundedContext` (in that order), and
//! the shared util spreads `{ ...itemData, id, deleted, createdAt }`. So the
//! key order is:
//!
//! ```text
//! type, color, text, [when], [then], [timestamp], [boundedContext], id, deleted, createdAt
//! ```
//!
//! We mutate the raw `eventStorm` value living in the work unit's `extra`
//! map (round-tripped via `#[serde(flatten)]` on
//! [`crate::types::work_unit::WorkUnit`]) using `serde_json::Map`
//! (insertion-order-preserving in this workspace) so the on-disk JSON matches
//! TS byte-for-byte.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/add_policy.rs` is JSON marshalling only — no domain
//! logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `add-policy`. Mirrors the TS
/// `AddPolicyOptions` interface at `src/commands/add-policy.ts:12-20`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddPolicyArgs {
    work_unit_id: String,
    text: String,
    #[serde(default)]
    when: Option<String>,
    #[serde(default)]
    then: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::js_compat::deserialize_present_value"
    )]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddPolicyResult {
    success: bool,
    #[serde(rename = "policyId")]
    policy_id: u64,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let mut args: AddPolicyArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-policy",
            reason: format!("failed to parse args: {e}"),
        })?;

    // JS-truthiness normalisation: add-policy.ts guards the optional string
    // fields with `if (options.when)` / `if (options.then)` /
    // `if (options.boundedContext)` — an empty string is falsy in JS, so those
    // flags are treated as ABSENT and omitted from the on-disk item. Only
    // `timestamp` uses `!== undefined` (so `0` IS persisted, which our
    // `Option<Value>` verbatim passthrough mirrors). Without this,
    // Rust's `Some("")` would
    // wrongly persist `"when": ""` etc. Whitespace-only values are truthy in
    // JS, so we strip only the exactly-empty string.
    if args.when.as_deref() == Some("") {
        args.when = None;
    }
    if args.then.as_deref() == Some("") {
        args.then = None;
    }
    if args.bounded_context.as_deref() == Some("") {
        args.bounded_context = None;
    }

    // Mirror the TS shared util's existence guard (event-storm-utils.ts:36-41)
    // AND the canonical sibling event-storm commands (add-command,
    // add-domain-event, …): `addEventStormItem` checks
    // `existsSync(join(cwd, 'spec', 'work-units.json'))` FIRST and returns the
    // verbatim error below when the file is absent — it does NOT auto-create
    // the file. We therefore read the file directly (NO `ensure_work_units_file`
    // auto-create) so both the error text and the resulting on-disk state match
    // TS byte-for-byte.
    let path = project_root.join("spec").join("work-units.json");
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-policy",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "add-policy",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors event-storm-utils.ts:63-69).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-policy",
                reason: format!("Work unit {} not found", args.work_unit_id),
            });
        }
    };

    // Validate work unit is not done/blocked (mirrors
    // event-storm-utils.ts:72-77). Capture the status string BEFORE
    // mutating so the canonical error embeds the TS-lowercase form.
    let status_str = wu.status.as_str();
    if status_str == "done" || status_str == "blocked" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-policy",
            reason: format!("Cannot add Event Storm items to work unit in {status_str} state"),
        });
    }

    let now = iso8601_now();

    // Seed eventStorm if missing or not an object (TS literal order:
    // level, items, nextItemId).
    let es_entry = wu.extra.entry("eventStorm".to_string()).or_insert_with(|| {
        let mut m = Map::new();
        m.insert(
            "level".to_string(),
            Value::String("process_modeling".to_string()),
        );
        m.insert("items".to_string(), Value::Array(Vec::new()));
        m.insert("nextItemId".to_string(), Value::from(0u64));
        Value::Object(m)
    });
    if !es_entry.is_object() {
        let mut m = Map::new();
        m.insert(
            "level".to_string(),
            Value::String("process_modeling".to_string()),
        );
        m.insert("items".to_string(), Value::Array(Vec::new()));
        m.insert("nextItemId".to_string(), Value::from(0u64));
        *es_entry = Value::Object(m);
    }

    let es = es_entry
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: "eventStorm must be an object".to_string(),
        })?;

    // Resolve nextItemId (default 0 if missing/non-numeric).
    let item_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

    // Build the policy item in TS object-literal insertion order:
    // type, color, text, [when], [then], [timestamp], [boundedContext],
    // id, deleted, createdAt.
    let mut item = Map::new();
    item.insert("type".to_string(), Value::String("policy".to_string()));
    item.insert("color".to_string(), Value::String("purple".to_string()));
    item.insert("text".to_string(), Value::String(args.text.clone()));
    if let Some(w) = args.when.as_deref() {
        item.insert("when".to_string(), Value::String(w.to_string()));
    }
    if let Some(t) = args.then.as_deref() {
        item.insert("then".to_string(), Value::String(t.to_string()));
    }
    if let Some(ts) = args.timestamp.as_ref() {
        item.insert("timestamp".to_string(), ts.clone());
    }
    if let Some(bc) = args.bounded_context.as_deref() {
        item.insert("boundedContext".to_string(), Value::String(bc.to_string()));
    }
    item.insert("id".to_string(), Value::from(item_id));
    item.insert("deleted".to_string(), Value::Bool(false));
    item.insert("createdAt".to_string(), Value::String(now));

    // Append to items (init if missing or non-array).
    let items_entry = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items_entry.is_array() {
        *items_entry = Value::Array(Vec::new());
    }
    if let Value::Array(arr) = items_entry {
        arr.push(Value::Object(item));
    }

    // Post-increment nextItemId.
    es.insert("nextItemId".to_string(), Value::from(item_id + 1));

    // Single atomic write. The TS shared util does NOT touch updatedAt, so
    // neither do we. `path` was resolved by the existence guard above.
    write_json_atomic(&path, &data)?;

    let result = AddPolicyResult {
        success: true,
        policy_id: item_id,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-policy",
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
    fn args_parse_camel_case_minimal() {
        let a: AddPolicyArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","text":"P1"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.text, "P1");
        assert!(a.when.is_none());
        assert!(a.then.is_none());
        assert!(a.timestamp.is_none());
        assert!(a.bounded_context.is_none());
    }

    #[test]
    fn args_parse_all_optional_fields() {
        let a: AddPolicyArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","text":"P1","when":"E","then":"C","timestamp":1000,"boundedContext":"Identity"}"#,
        )
        .unwrap();
        assert_eq!(a.when.as_deref(), Some("E"));
        assert_eq!(a.then.as_deref(), Some("C"));
        assert_eq!(a.timestamp, Some(Value::from(1000)));
        assert_eq!(a.bounded_context.as_deref(), Some("Identity"));
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<AddPolicyArgs>(r#"{"text":"P1"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }
}
