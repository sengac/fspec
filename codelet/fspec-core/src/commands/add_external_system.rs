//! `add-external-system` — Rust port of `src/commands/add-external-system.ts`
//! (RPC-182).
//!
//! Appends an `external_system` Event Storm item to a work unit's
//! `eventStorm.items` array during the Process-Modeling phase of Event
//! Storming. Mirrors the shared TS helper `addEventStormItem`
//! (`src/commands/event-storm-utils.ts`) collapsed inline for module
//! isolation.
//!
//! ## Semantics (parity with TS)
//!
//! 1. If `spec/work-units.json` does NOT exist → return the canonical
//!    `"spec/work-units.json not found. Run fspec init first."` error
//!    WITHOUT auto-creating the file (TS `existsSync` guard at
//!    `event-storm-utils.ts:36-41`).
//! 2. Missing work unit → `"Work unit {id} not found"`.
//! 3. `done` / `blocked` status → `"Cannot add Event Storm items to work
//!    unit in {status} state"`.
//! 4. On success: seed `eventStorm` (`level=process_modeling`, `items=[]`,
//!    `nextItemId=0`) if absent, append the item with `id = nextItemId`,
//!    post-increment `nextItemId`, write atomically.
//!
//! ## On-disk item shape & key order
//!
//! The TS helper spreads `...itemData` (which the command builds as
//! `type, color, text, [integrationType], [timestamp], [boundedContext]`)
//! and THEN appends `id, deleted, createdAt`. We reproduce that exact
//! insertion order using `serde_json::Map`. `color` is the string
//! literal `"pink"` for external systems (matching physical Event Storming
//! convention). The CLI `--type` flag maps to the `integrationType`
//! field (design decision).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge
//! at `codelet/fspec/src/add_external_system.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-external-system`. Mirrors the TS
/// `AddExternalSystemOptions` interface at
/// `src/commands/add-external-system.ts:12-24`. The `type` field maps to
/// the persisted `integrationType` item field.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddExternalSystemArgs {
    work_unit_id: String,
    text: String,
    #[serde(default, rename = "type")]
    integration_type: Option<String>,
    #[serde(default, deserialize_with = "crate::js_compat::deserialize_present_value")]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddExternalSystemArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-external-system",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Build the item body in TS insertion order (type, color, text,
    // [integrationType], [timestamp], [boundedContext]). `id`, `deleted`,
    // `createdAt` are appended by `append_event_storm_item`.
    let mut item_body = Map::new();
    item_body.insert("type".to_string(), Value::String("external_system".to_string()));
    item_body.insert("color".to_string(), Value::String("pink".to_string()));
    item_body.insert("text".to_string(), Value::String(args.text.clone()));
    if let Some(it) = args.integration_type.as_deref() {
        item_body.insert("integrationType".to_string(), Value::String(it.to_string()));
    }
    if let Some(ts) = args.timestamp.as_ref() {
        item_body.insert("timestamp".to_string(), ts.clone());
    }
    if let Some(bc) = args.bounded_context.as_deref() {
        item_body.insert("boundedContext".to_string(), Value::String(bc.to_string()));
    }

    let item_id =
        append_event_storm_item(project_root, "add-external-system", &args.work_unit_id, item_body)?;

    let mut result = Map::new();
    result.insert("success".to_string(), Value::Bool(true));
    result.insert("externalSystemId".to_string(), Value::from(item_id));
    serde_json::to_string(&Value::Object(result)).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-external-system",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Inline Rust analog of the shared TS `addEventStormItem` helper
/// (`src/commands/event-storm-utils.ts:29-114`). Duplicated within each
/// Event-Storm command module to keep the module self-contained (no new
/// shared submodule in the supervisor-owned `commands/mod.rs`).
///
/// Returns the assigned stable item `id` (= `nextItemId` before
/// increment). The whole `spec/work-units.json` document is round-tripped
/// as a raw `serde_json::Value` so unknown top-level keys and existing
/// field order are preserved byte-for-byte.
fn append_event_storm_item(
    project_root: &Path,
    command: &'static str,
    work_unit_id: &str,
    mut item_body: Map<String, Value>,
) -> Result<u64, FspecCoreError> {
    let path = project_root.join("spec").join("work-units.json");

    // [1] existsSync guard — do NOT auto-create the file.
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command,
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command,
        source,
    })?;
    let mut root: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "work-units.json".to_string(),
        reason: e.to_string(),
    })?;

    // [2] Validate work unit exists.
    let status = root
        .get("workUnits")
        .and_then(|w| w.get(work_unit_id))
        .and_then(|wu| wu.get("status"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let exists = root
        .get("workUnits")
        .and_then(|w| w.get(work_unit_id))
        .is_some();
    if !exists {
        return Err(FspecCoreError::InvalidArgs {
            command,
            reason: format!("Work unit {work_unit_id} not found"),
        });
    }

    // [3] done/blocked guard. The status string is echoed verbatim.
    if let Some(st) = status.as_deref() {
        if st == "done" || st == "blocked" {
            return Err(FspecCoreError::InvalidArgs {
                command,
                reason: format!(
                    "Cannot add Event Storm items to work unit in {st} state"
                ),
            });
        }
    }

    // [4] Mutate: seed eventStorm if absent, append item, bump nextItemId.
    let wu = root
        .get_mut("workUnits")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.get_mut(work_unit_id))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command,
            reason: format!("Work unit {work_unit_id} not found"),
        })?;

    let es_entry = wu
        .entry("eventStorm".to_string())
        .or_insert_with(|| {
            let mut es = Map::new();
            es.insert("level".to_string(), Value::String("process_modeling".to_string()));
            es.insert("items".to_string(), Value::Array(Vec::new()));
            es.insert("nextItemId".to_string(), Value::from(0u64));
            Value::Object(es)
        });
    if !es_entry.is_object() {
        let mut es = Map::new();
        es.insert("level".to_string(), Value::String("process_modeling".to_string()));
        es.insert("items".to_string(), Value::Array(Vec::new()));
        es.insert("nextItemId".to_string(), Value::from(0u64));
        *es_entry = Value::Object(es);
    }
    let es = es_entry.as_object_mut().expect("eventStorm is an object");

    let item_id = es
        .get("nextItemId")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    // Append id, deleted, createdAt in TS spread order.
    item_body.insert("id".to_string(), Value::from(item_id));
    item_body.insert("deleted".to_string(), Value::Bool(false));
    item_body.insert("createdAt".to_string(), Value::String(iso8601_now()));

    let items = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items.is_array() {
        *items = Value::Array(Vec::new());
    }
    if let Value::Array(arr) = items {
        arr.push(Value::Object(item_body));
    }

    es.insert("nextItemId".to_string(), Value::from(item_id + 1));

    // Single atomic write of the full document.
    write_json_atomic(&path, &root)?;

    Ok(item_id)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case_minimal() {
        let a: AddExternalSystemArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","text":"Payment Gateway"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.text, "Payment Gateway");
        assert!(a.integration_type.is_none());
    }

    #[test]
    fn type_field_maps_to_integration_type() {
        let a: AddExternalSystemArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","text":"Stripe API","type":"REST_API"}"#,
        )
        .unwrap();
        assert_eq!(a.integration_type.as_deref(), Some("REST_API"));
    }

    #[test]
    fn iso8601_helper_shape() {
        let s = iso8601_now();
        assert_eq!(s.len(), 24);
        assert!(s.ends_with('Z'));
    }
}
