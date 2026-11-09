//! `remove-assumption` — BUG-633: soft-delete an assumption by its stable id.
//!
//! Feature: spec/features/remove-and-restore-assumptions.feature
//!
//! Marks the assumption whose stable id is `index` `deleted: true` with a
//! `deletedAt` timestamp; `fspec restore-assumption` undoes it.
//! The work unit must exist and be in `specifying`. The card's string
//! assumptions are converted to stable-id items first
//! ([`super::assumption_items::convert`]). Idempotent: removing an assumption that is already deleted succeeds with
//! `message: "Item ID <id> already deleted"` and
//! performs no disk write.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::assumption_items;
use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::time::iso8601_now;

const COMMAND: &str = "remove-assumption";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Args {
    work_unit_id: String,
    index: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Outcome {
    success: bool,
    removed_assumption: String,
    remaining_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason,
    }
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;

    // BUG-477: hold the spec lock from the first read to the last write.
    let _spec_lock = crate::io::spec_lock::lock_spec_dir(project_root, COMMAND)?;
    let mut data = ensure_work_units_file(project_root)?;
    let id = args.work_unit_id.as_str();
    let wu = data
        .work_units
        .get_mut(id)
        .ok_or_else(|| invalid(format!("Work unit '{id}' does not exist")))?;
    if wu.status.as_str() != "specifying" {
        return Err(invalid(format!(
            "Can only remove assumptions during discovery/specification phase. {id} is in '{}' state.",
            wu.status.as_str()
        )));
    }
    let items = assumption_items::read(id, wu.extra.get(assumption_items::KEY)).map_err(invalid)?;
    if items.is_empty() {
        return Err(invalid(format!("Work unit {id} has no assumptions")));
    }
    let target = items
        .iter()
        .find(|item| item.id == args.index)
        .ok_or_else(|| invalid(format!("Assumption with ID {} not found", args.index)))?
        .clone();
    let count =
        |items: &[assumption_items::Assumption]| items.iter().filter(|item| !item.deleted).count();
    if target.deleted {
        let outcome = Outcome {
            success: true,
            removed_assumption: target.text,
            remaining_count: count(&items),
            message: Some(format!("Item ID {} already deleted", args.index)),
        };
        return serde_json::to_string(&outcome)
            .map_err(|e| invalid(format!("failed to serialize result: {e}")));
    }

    let now = iso8601_now();
    assumption_items::convert(id, &mut wu.extra, &now).map_err(invalid)?;
    if let Some(entries) = wu
        .extra
        .get_mut(assumption_items::KEY)
        .and_then(Value::as_array_mut)
    {
        for entry in entries.iter_mut().filter_map(Value::as_object_mut) {
            if entry.get("id").and_then(Value::as_u64) == Some(args.index) {
                entry.insert("deleted".to_owned(), Value::Bool(true));
                entry.insert("deletedAt".to_owned(), Value::String(now.clone()));
            }
        }
    }
    let items = assumption_items::read(id, wu.extra.get(assumption_items::KEY)).map_err(invalid)?;
    wu.updated_at = now;

    crate::io::board::write_work_units(project_root, &data)?;

    let outcome = Outcome {
        success: true,
        removed_assumption: target.text,
        remaining_count: count(&items),
        message: None,
    };
    serde_json::to_string(&outcome).map_err(|e| invalid(format!("failed to serialize result: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: Args = serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.index, 3);
    }
}
