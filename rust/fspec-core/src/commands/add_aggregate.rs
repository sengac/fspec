//! `add-aggregate` — Rust port of `src/commands/add-aggregate.ts` (RPC-165).
//!
//! Appends an Event-Storm *aggregate* item (yellow sticky) to a work unit's
//! `eventStorm.items` array during the discovery / Process-Modeling phase.
//! The work unit must exist and must NOT be in `done` / `blocked` status;
//! otherwise a canonical validation error is returned and disk state is left
//! untouched.
//!
//! ## Option B — inline missing-source check (supervisor ruling)
//!
//! Unlike [`crate::commands::add_rule`], this command MUST NOT auto-create
//! `spec/work-units.json`. The TS source (`src/commands/add-aggregate.ts:41`)
//! guards with `existsSync` and returns
//! `"spec/work-units.json not found. Run fspec init first."` when the file is
//! absent — without ever touching the disk. We mirror that exactly: an inline
//! `Path::exists` check, a plain `std::fs::read_to_string` + parse, and a
//! final [`write_json_atomic`] on the success path only.
//!
//! ## On-disk item shape (eventStorm.items[])
//!
//! ```json
//! { "id": 0, "type": "aggregate", "color": "yellow", "text": "Order",
//!   "deleted": false, "createdAt": "<iso>",
//!   "responsibilities": ["..."], "timestamp": 123, "boundedContext": "..." }
//! ```
//!
//! Field declaration order matches the TS object-literal insertion order so
//! the persisted JSON is byte-identical (serde_json's `preserve_order`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/add_aggregate.rs` parses the `{success, aggregateId}`
//! result JSON to render the success line — it contains no domain logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnitStatus, WorkUnitsData};

/// CLI arguments accepted by `add-aggregate`. Mirrors the TS
/// `AddAggregateOptions` interface at `src/commands/add-aggregate.ts:13-20`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddAggregateArgs {
    work_unit_id: String,
    text: String,
    /// Comma-separated list, split/trimmed/empty-filtered on the success path.
    #[serde(default)]
    responsibilities: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::js_compat::deserialize_present_value"
    )]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddAggregateResult {
    success: bool,
    #[serde(rename = "aggregateId")]
    aggregate_id: u64,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddAggregateArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-aggregate",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Option B: inline missing-source check — NO auto-create.
    let path = project_root.join("spec").join("work-units.json");
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-aggregate",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "add-aggregate",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors src/commands/add-aggregate.ts:68-74).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-aggregate",
            reason: format!("Work unit {} not found", args.work_unit_id),
        });
    }

    // Validate work unit not in done/blocked status
    // (mirrors src/commands/add-aggregate.ts:77-82). The status string is
    // embedded verbatim in its TS-lowercase form.
    let status = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "add-aggregate",
            reason: format!("Work unit {} not found", args.work_unit_id),
        })?;
    if matches!(status, WorkUnitStatus::Done | WorkUnitStatus::Blocked) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-aggregate",
            reason: format!(
                "Cannot add Event Storm items to work unit in {} state",
                status.as_str()
            ),
        });
    }

    let now = iso8601_now();

    // ---- mutate the work unit's eventStorm section ----
    let aggregate_id: u64 = {
        let wu = data.work_units.get_mut(&args.work_unit_id).ok_or_else(|| {
            FspecCoreError::InvalidArgs {
                command: "add-aggregate",
                reason: format!("Work unit {} not found", args.work_unit_id),
            }
        })?;

        // Initialize eventStorm only when absent (mirrors `if (!workUnit.eventStorm)`).
        let es_missing = !matches!(wu.extra.get("eventStorm"), Some(Value::Object(_)));
        if es_missing {
            wu.extra.insert(
                "eventStorm".to_string(),
                json!({ "level": "process_modeling", "items": [], "nextItemId": 0 }),
            );
        }

        let es = wu
            .extra
            .get_mut("eventStorm")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "add-aggregate",
                reason: "failed to initialize eventStorm section".to_string(),
            })?;

        let next_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

        // Build the aggregate item in TS object-literal insertion order.
        let mut item = Map::new();
        item.insert("id".to_string(), Value::from(next_id));
        item.insert("type".to_string(), Value::String("aggregate".to_string()));
        item.insert("color".to_string(), Value::String("yellow".to_string()));
        item.insert("text".to_string(), Value::String(args.text.clone()));
        item.insert("deleted".to_string(), Value::Bool(false));
        item.insert("createdAt".to_string(), Value::String(now.clone()));

        // Optional fields, appended in TS order.
        if let Some(resp) = args.responsibilities.as_ref() {
            if !resp.is_empty() {
                let list: Vec<Value> = resp
                    .split(',')
                    .map(str::trim)
                    .filter(|r| !r.is_empty())
                    .map(|r| Value::String(r.to_string()))
                    .collect();
                item.insert("responsibilities".to_string(), Value::Array(list));
            }
        }
        if let Some(ts) = args.timestamp.as_ref() {
            item.insert("timestamp".to_string(), ts.clone());
        }
        if let Some(bc) = args.bounded_context.as_ref() {
            if !bc.is_empty() {
                item.insert("boundedContext".to_string(), Value::String(bc.clone()));
            }
        }

        // Append, then post-increment nextItemId.
        let items = es
            .entry("items".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !items.is_array() {
            *items = Value::Array(Vec::new());
        }
        if let Value::Array(arr) = items {
            arr.push(Value::Object(item));
        }
        es.insert("nextItemId".to_string(), Value::from(next_id + 1));

        wu.updated_at = now.clone();
        next_id
    };

    // Bump meta.lastUpdated when present (mirrors src/commands/add-aggregate.ts:127-129).
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    write_json_atomic(&path, &data)?;

    let result = AddAggregateResult {
        success: true,
        aggregate_id,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-aggregate",
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
    fn args_parse_camel_case_with_optionals() {
        let a: AddAggregateArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","text":"Order","responsibilities":"a, b","boundedContext":"X"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.text, "Order");
        assert_eq!(a.responsibilities.as_deref(), Some("a, b"));
        assert_eq!(a.bounded_context.as_deref(), Some("X"));
        assert!(a.timestamp.is_none());
    }

    #[test]
    fn args_parse_fails_without_text() {
        let err =
            serde_json::from_str::<AddAggregateArgs>(r#"{"workUnitId":"AUTH-001"}"#).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("text"));
    }
}
