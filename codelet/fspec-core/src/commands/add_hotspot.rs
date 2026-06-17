//! `add-hotspot` — Rust port of `src/commands/add-hotspot.ts` (RPC-185).
//!
//! Appends a hotspot item (color `red`, type `hotspot`) to a work unit's
//! Event Storm during the discovery/specifying phase. Hotspots capture
//! uncertainties, risks, or problems to investigate.
//!
//! ## Shared `addEventStormItem` semantics — NO dedup
//!
//! The TypeScript `addHotspot` delegates to the shared `addEventStormItem`
//! helper (`src/commands/event-storm-utils.ts`), which — unlike
//! `add-domain-event` — performs **no duplicate check**. The same hotspot
//! text may be added repeatedly, each getting a fresh sequential id. We
//! mirror that helper's behaviour inline here (a dedicated shared Rust util
//! module would require touching `commands/mod.rs`, which is supervisor-owned;
//! the observable contract is identical either way).
//!
//! ## Missing-file behaviour — Option B (INLINE existsSync + read, NO auto-create)
//!
//! `addEventStormItem` checks `existsSync(workUnitsFile)` and returns
//! `spec/work-units.json not found. Run fspec init first.` WITHOUT creating
//! the file (`src/commands/event-storm-utils.ts:36-41`). We do NOT use
//! [`crate::io::ensure::ensure_work_units_file`] (which auto-creates).
//!
//! ## On-disk item shape
//!
//! Field order matches the TS object spread `{...itemData, id, deleted,
//! createdAt}`: `type, color, text`, then optional `concern`, `timestamp`,
//! `boundedContext`, then `id, deleted, createdAt`.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/add_hotspot.rs` is JSON marshalling only — no domain
//! logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `add-hotspot`. Mirrors the TS
/// `AddHotspotOptions` interface at `src/commands/add-hotspot.ts:12-19`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddHotspotArgs {
    work_unit_id: String,
    text: String,
    #[serde(default)]
    concern: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::js_compat::deserialize_present_value"
    )]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddHotspotResult {
    success: bool,
    #[serde(rename = "hotspotId")]
    hotspot_id: u64,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddHotspotArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-hotspot",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("work-units.json");

    // Option B — INLINE existsSync check, NO auto-create. Mirrors TS
    // `src/commands/event-storm-utils.ts:36-41`.
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-hotspot",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }

    // Read + parse (escalate malformed JSON, parity with TS readJSON throw).
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "add-hotspot",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors src/commands/event-storm-utils.ts:63-69).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-hotspot",
                reason: format!("Work unit {} not found", args.work_unit_id),
            });
        }
    };

    // Validate work unit is not in done/blocked state (mirrors
    // src/commands/event-storm-utils.ts:72-77).
    let status_str = wu.status.as_str();
    if status_str == "done" || status_str == "blocked" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-hotspot",
            reason: format!("Cannot add Event Storm items to work unit in {status_str} state"),
        });
    }

    // Initialize eventStorm if missing (mirrors src/commands/event-storm-utils.ts:83-89).
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

    // Ensure items array exists.
    let items_val = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items_val.is_array() {
        *items_val = Value::Array(Vec::new());
    }

    let now = iso8601_now();

    // nextItemId → hotspotId (post-increment). NO dedup — hotspots may repeat.
    let hotspot_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

    // Build the hotspot item. Field order matches the TS spread
    // `{...itemData, id, deleted, createdAt}` where itemData is built as
    // {type, color, text, [concern], [timestamp], [boundedContext]}.
    let mut item_obj = Map::new();
    item_obj.insert("type".to_string(), Value::String("hotspot".to_string()));
    item_obj.insert("color".to_string(), Value::String("red".to_string()));
    item_obj.insert("text".to_string(), Value::String(args.text.clone()));
    if let Some(c) = &args.concern {
        if !c.is_empty() {
            item_obj.insert("concern".to_string(), Value::String(c.clone()));
        }
    }
    if let Some(ts) = &args.timestamp {
        item_obj.insert("timestamp".to_string(), ts.clone());
    }
    if let Some(bc) = &args.bounded_context {
        if !bc.is_empty() {
            item_obj.insert("boundedContext".to_string(), Value::String(bc.clone()));
        }
    }
    item_obj.insert("id".to_string(), Value::from(hotspot_id));
    item_obj.insert("deleted".to_string(), Value::Bool(false));
    item_obj.insert("createdAt".to_string(), Value::String(now.clone()));

    // Append the new hotspot and post-increment nextItemId.
    if let Some(Value::Array(items)) = es.get_mut("items") {
        items.push(Value::Object(item_obj));
    }
    es.insert("nextItemId".to_string(), Value::from(hotspot_id + 1));

    // Bump updatedAt.
    wu.updated_at = now;

    // Single atomic write.
    write_json_atomic(&path, &data)?;

    let result = AddHotspotResult {
        success: true,
        hotspot_id,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-hotspot",
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
    fn args_parse_camel_case() {
        let a: AddHotspotArgs =
            serde_json::from_str(r#"{"workUnitId":"RPC-185","text":"Unclear retry policy"}"#)
                .unwrap();
        assert_eq!(a.work_unit_id, "RPC-185");
        assert_eq!(a.text, "Unclear retry policy");
        assert!(a.concern.is_none());
    }

    #[test]
    fn args_parse_optional_fields() {
        let a: AddHotspotArgs = serde_json::from_str(
            r#"{"workUnitId":"RPC-185","text":"Timeout unknown","concern":"How long?","timestamp":500,"boundedContext":"Payments"}"#,
        )
        .unwrap();
        assert_eq!(a.concern.as_deref(), Some("How long?"));
        assert_eq!(a.timestamp, Some(Value::from(500)));
        assert_eq!(a.bounded_context.as_deref(), Some("Payments"));
    }
}
