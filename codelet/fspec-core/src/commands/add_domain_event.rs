//! `add-domain-event` — Rust port of `src/commands/add-domain-event.ts` (RPC-179).
//!
//! Appends a domain-event item (color `orange`, type `event`) to a work
//! unit's Event Storm during the discovery/specifying phase of Big Picture
//! Event Storming.
//!
//! ## Why this command INLINES its logic (no shared util)
//!
//! Unlike `add-hotspot` (which routes through the shared `addEventStormItem`
//! helper), the TypeScript `addDomainEvent` carries the **BUG-087** fix: a
//! case-insensitive duplicate check over non-deleted `type === 'event'` items.
//! That guard lives inline in the TS function (`src/commands/add-domain-event.ts:93-106`),
//! so the Rust port mirrors it inline here.
//!
//! ## Missing-file behaviour — Option B (INLINE existsSync + read, NO auto-create)
//!
//! The TS implementation explicitly checks `existsSync(workUnitsFile)` and
//! returns the error `spec/work-units.json not found. Run fspec init first.`
//! WITHOUT creating the file (`src/commands/add-domain-event.ts:41-46`). We
//! therefore do NOT use [`crate::io::ensure::ensure_work_units_file`] (which
//! auto-creates); instead we inline the existence check, read, and parse.
//!
//! ## On-disk item shape
//!
//! Field declaration order matches the TS object literal so on-disk JSON is
//! byte-identical: `id, type, color, text, deleted, createdAt`, then the
//! optional `timestamp` and `boundedContext` appended only when supplied.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/add_domain_event.rs` is JSON marshalling only — no
//! domain logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `add-domain-event`. Mirrors the TS
/// `AddDomainEventOptions` interface at `src/commands/add-domain-event.ts:14-20`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddDomainEventArgs {
    work_unit_id: String,
    text: String,
    #[serde(
        default,
        deserialize_with = "crate::js_compat::deserialize_present_value"
    )]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddDomainEventResult {
    success: bool,
    #[serde(rename = "eventId")]
    event_id: u64,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddDomainEventArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("work-units.json");

    // Option B — INLINE existsSync check, NO auto-create. Mirrors TS
    // `src/commands/add-domain-event.ts:41-46`.
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }

    // Read + parse (escalate malformed JSON, parity with TS readJSON throw).
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "add-domain-event",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: e.to_string(),
        })?;

    // Validate work unit exists (mirrors src/commands/add-domain-event.ts:68-74).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: format!("Work unit {} not found", args.work_unit_id),
        });
    }

    // Validate work unit is not in done/blocked state (mirrors
    // src/commands/add-domain-event.ts:77-82). Capture the status string
    // BEFORE mutating; the error message embeds it lowercase.
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: format!("Work unit {} not found", args.work_unit_id),
        })?;
    if status_str == "done" || status_str == "blocked" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: format!("Cannot add Event Storm items to work unit in {status_str} state"),
        });
    }

    let wu =
        data.work_units
            .get_mut(&args.work_unit_id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "add-domain-event",
                reason: format!("Work unit {} not found", args.work_unit_id),
            })?;

    // Initialize eventStorm if missing (mirrors src/commands/add-domain-event.ts:85-91).
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
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "add-domain-event",
            reason: "failed to initialize eventStorm section".to_string(),
        })?;

    // Ensure items array exists.
    let items_val = es
        .entry("items".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !items_val.is_array() {
        *items_val = Value::Array(Vec::new());
    }

    // BUG-087: case-insensitive duplicate check over non-deleted type:event
    // items (mirrors src/commands/add-domain-event.ts:93-106). Done BEFORE
    // any mutation so a duplicate leaves the file untouched.
    let text_lower = args.text.to_lowercase();
    if let Value::Array(items) = items_val {
        for item in items.iter() {
            let is_event = item.get("type").and_then(Value::as_str) == Some("event");
            let deleted = matches!(item.get("deleted"), Some(Value::Bool(true)));
            let matches_text = item
                .get("text")
                .and_then(Value::as_str)
                .map(|t| t.to_lowercase() == text_lower)
                .unwrap_or(false);
            if is_event && !deleted && matches_text {
                let existing_id = item.get("id").and_then(Value::as_u64).unwrap_or(0);
                return Err(FspecCoreError::InvalidArgs {
                    command: "add-domain-event",
                    reason: format!("Event '{}' already exists (ID: {existing_id})", args.text),
                });
            }
        }
    }

    let now = iso8601_now();

    // nextItemId → eventId (post-increment).
    let event_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

    // Build the event item with explicit field order matching the TS object
    // literal: id, type, color, text, deleted, createdAt, then optionals.
    let mut event_obj = Map::new();
    event_obj.insert("id".to_string(), Value::from(event_id));
    event_obj.insert("type".to_string(), Value::String("event".to_string()));
    event_obj.insert("color".to_string(), Value::String("orange".to_string()));
    event_obj.insert("text".to_string(), Value::String(args.text.clone()));
    event_obj.insert("deleted".to_string(), Value::Bool(false));
    event_obj.insert("createdAt".to_string(), Value::String(now.clone()));
    if let Some(ts) = &args.timestamp {
        event_obj.insert("timestamp".to_string(), ts.clone());
    }
    if let Some(bc) = &args.bounded_context {
        if !bc.is_empty() {
            event_obj.insert("boundedContext".to_string(), Value::String(bc.clone()));
        }
    }

    // Append the new event and post-increment nextItemId.
    if let Some(Value::Array(items)) = es.get_mut("items") {
        items.push(Value::Object(event_obj));
    }
    es.insert("nextItemId".to_string(), Value::from(event_id + 1));

    // Bump updatedAt.
    wu.updated_at = now;

    // Single atomic write.
    write_json_atomic(&path, &data)?;

    let result = AddDomainEventResult {
        success: true,
        event_id,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-domain-event",
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
        let a: AddDomainEventArgs =
            serde_json::from_str(r#"{"workUnitId":"RPC-179","text":"UserRegistered"}"#).unwrap();
        assert_eq!(a.work_unit_id, "RPC-179");
        assert_eq!(a.text, "UserRegistered");
        assert!(a.timestamp.is_none());
        assert!(a.bounded_context.is_none());
    }

    #[test]
    fn args_parse_optional_fields() {
        let a: AddDomainEventArgs = serde_json::from_str(
            r#"{"workUnitId":"RPC-179","text":"OrderPlaced","timestamp":1000,"boundedContext":"Sales"}"#,
        )
        .unwrap();
        assert_eq!(a.timestamp, Some(Value::from(1000)));
        assert_eq!(a.bounded_context.as_deref(), Some("Sales"));
    }
}
