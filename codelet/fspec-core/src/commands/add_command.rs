//! `add-command` — Rust port of `src/commands/add-command.ts` (RPC-174).
//!
//! Appends an Event-Storm *command* item (blue sticky) to a work unit's
//! `eventStorm.items` array during Big-Picture Event-Storming discovery. The
//! work unit must exist and must NOT be in `done` / `blocked` status;
//! otherwise a canonical validation error is returned and disk state is left
//! untouched.
//!
//! ## Option B — inline missing-source check (supervisor ruling)
//!
//! Like [`crate::commands::add_aggregate`], this command MUST NOT auto-create
//! `spec/work-units.json`. The TS source (`src/commands/add-command.ts:42`)
//! guards with `existsSync` and returns
//! `"spec/work-units.json not found. Run fspec init first."` when the file is
//! absent — without touching disk. We mirror that with an inline
//! `Path::exists` check + plain read, then a single [`write_json_atomic`] on
//! the success path only.
//!
//! ## On-disk item shape (eventStorm.items[])
//!
//! ```json
//! { "id": 0, "type": "command", "color": "blue", "text": "PlaceOrder",
//!   "deleted": false, "createdAt": "<iso>",
//!   "actor": "Customer", "timestamp": 123, "boundedContext": "..." }
//! ```
//!
//! ## Two-front-doors
//!
//! Both the dispatcher AND the clap subcommand call this single function; the
//! CLI bridge at `codelet/fspec/src/add_command.rs` parses the
//! `{success, commandId}` result JSON to render the success line.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnitStatus, WorkUnitsData};

/// CLI arguments accepted by `add-command`. Mirrors the TS
/// `AddCommandOptions` interface at `src/commands/add-command.ts:14-21`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddCommandArgs {
    work_unit_id: String,
    text: String,
    #[serde(default)]
    actor: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::js_compat::deserialize_present_value"
    )]
    timestamp: Option<Value>,
    #[serde(default)]
    bounded_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddCommandResult {
    success: bool,
    #[serde(rename = "commandId")]
    command_id: u64,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddCommandArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-command",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Option B: inline missing-source check — NO auto-create.
    let path = project_root.join("spec").join("work-units.json");
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-command",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "add-command",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors src/commands/add-command.ts:69-75).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-command",
            reason: format!("Work unit {} not found", args.work_unit_id),
        });
    }

    // Validate work unit not in done/blocked status
    // (mirrors src/commands/add-command.ts:78-83).
    let status = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "add-command",
            reason: format!("Work unit {} not found", args.work_unit_id),
        })?;
    if matches!(status, WorkUnitStatus::Done | WorkUnitStatus::Blocked) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-command",
            reason: format!(
                "Cannot add Event Storm items to work unit in {} state",
                status.as_str()
            ),
        });
    }

    let now = iso8601_now();

    // ---- mutate the work unit's eventStorm section ----
    let command_id: u64 = {
        let wu = data.work_units.get_mut(&args.work_unit_id).ok_or_else(|| {
            FspecCoreError::InvalidArgs {
                command: "add-command",
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
                command: "add-command",
                reason: "failed to initialize eventStorm section".to_string(),
            })?;

        let next_id = es.get("nextItemId").and_then(Value::as_u64).unwrap_or(0);

        // Build the command item in TS object-literal insertion order.
        let mut item = Map::new();
        item.insert("id".to_string(), Value::from(next_id));
        item.insert("type".to_string(), Value::String("command".to_string()));
        item.insert("color".to_string(), Value::String("blue".to_string()));
        item.insert("text".to_string(), Value::String(args.text.clone()));
        item.insert("deleted".to_string(), Value::Bool(false));
        item.insert("createdAt".to_string(), Value::String(now.clone()));

        // Optional fields, appended in TS order.
        if let Some(actor) = args.actor.as_ref() {
            if !actor.is_empty() {
                item.insert("actor".to_string(), Value::String(actor.clone()));
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

    // Bump meta.lastUpdated when present (mirrors src/commands/add-command.ts:124-126).
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    write_json_atomic(&path, &data)?;

    let result = AddCommandResult {
        success: true,
        command_id,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-command",
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
        let a: AddCommandArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","text":"PlaceOrder","actor":"Customer","boundedContext":"Sales"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.text, "PlaceOrder");
        assert_eq!(a.actor.as_deref(), Some("Customer"));
        assert_eq!(a.bounded_context.as_deref(), Some("Sales"));
        assert!(a.timestamp.is_none());
    }

    #[test]
    fn args_parse_fails_without_text() {
        let err =
            serde_json::from_str::<AddCommandArgs>(r#"{"workUnitId":"AUTH-001"}"#).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("text"));
    }
}
