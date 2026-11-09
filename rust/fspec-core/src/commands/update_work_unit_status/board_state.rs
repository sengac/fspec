//! Board-state updates for `update-work-unit-status`: state-array moves and done compaction.
//! Status hooks (virtual, HOOK-019; global, HOOK-020) run in `crate::command_hooks::run_status_hooks`.

use serde_json::{json, Value};

use super::queries::Data;
use super::transitions::{msg, status_from_str};
use crate::error::FspecCoreError;

pub(super) fn move_state(
    data: &mut crate::types::work_unit::WorkUnitsData,
    id: &str,
    from: &str,
    to: &str,
) {
    if from == to {
        return;
    }
    if let Some(arr) = states_column(&mut data.states, from) {
        arr.retain(|v| v != id);
    }
    if let Some(arr) = states_column(&mut data.states, to) {
        if !arr.iter().any(|v| v == id) {
            arr.push(id.to_string());
        }
    }
}

/// Mutable handle to the named status column on the typed `WorkUnitStates`.
pub(super) fn states_column<'a>(
    states: &'a mut crate::types::work_unit::WorkUnitStates,
    name: &str,
) -> Option<&'a mut Vec<String>> {
    match name {
        "backlog" => Some(&mut states.backlog),
        "specifying" => Some(&mut states.specifying),
        "testing" => Some(&mut states.testing),
        "implementing" => Some(&mut states.implementing),
        "validating" => Some(&mut states.validating),
        "done" => Some(&mut states.done),
        "blocked" => Some(&mut states.blocked),
        _ => None,
    }
}

/// Inline compaction: drop soft-deleted Example-Mapping items and renumber.
pub(super) fn compact_done(data: &mut crate::types::work_unit::WorkUnitsData, id: &str) {
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    for key in ["rules", "examples", "questions", "architectureNotes"] {
        if let Some(arr) = wu.extra.get_mut(key).and_then(Value::as_array_mut) {
            arr.retain(|item| {
                !item
                    .get("deleted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
            for (i, item) in arr.iter_mut().enumerate() {
                if let Some(obj) = item.as_object_mut() {
                    obj.insert("id".to_string(), json!(i as u64));
                }
            }
        }
    }
}

/// Record the new status on the work unit: status, `updatedAt`,
/// `blockedReason` and a `stateHistory` entry. A WT-018 discard of a
/// session records `discardOf` and the reason `discard` (a restored
/// blocked status keeps its blocked reason).
pub(super) fn apply_status_change(
    data: &mut Data,
    id: &str,
    new_status: &str,
    blocked_reason: Option<&str>,
    reason: Option<&str>,
    discard_of: Option<&str>,
    now: &str,
) -> Result<(), FspecCoreError> {
    let wu = data
        .work_units
        .get_mut(id)
        .ok_or_else(|| msg(format!("Work unit {id} does not exist")))?;

    // BUG-385: validate before mutating; an unknown status or a non-array
    // history fails the move instead of being ignored or replaced.
    let parsed = status_from_str(new_status)
        .ok_or_else(|| msg(format!("Cannot move {id} to unknown status '{new_status}'")))?;
    if wu
        .extra
        .get("stateHistory")
        .is_some_and(|history| !history.is_array())
    {
        return Err(super::queries::malformed(
            id,
            "stateHistory",
            "expected an array",
        ));
    }
    wu.status = parsed;
    wu.updated_at = now.to_string();

    if new_status == "blocked" {
        if let Some(reason) = blocked_reason {
            wu.extra.insert("blockedReason".to_string(), json!(reason));
        }
    } else {
        wu.extra.remove("blockedReason");
    }

    let mut entry = serde_json::Map::new();
    entry.insert("state".to_string(), Value::String(new_status.to_string()));
    entry.insert("timestamp".to_string(), Value::String(now.to_string()));
    // `reason` mirrors TS: options.reason wins unless moving to blocked,
    // in which case the blockedReason is recorded as the reason.
    if let Some(r) = reason {
        entry.insert("reason".to_string(), Value::String(r.to_string()));
    }
    if let Some(session) = discard_of {
        entry.insert("reason".to_string(), Value::String("discard".to_string()));
        entry.insert("discardOf".to_string(), Value::String(session.to_string()));
    }
    if new_status == "blocked" {
        if let Some(r) = blocked_reason {
            entry.insert("reason".to_string(), Value::String(r.to_string()));
        }
    }
    let entry = Value::Object(entry);
    match wu
        .extra
        .entry("stateHistory".to_string())
        .or_insert_with(|| Value::Array(Vec::new()))
    {
        Value::Array(history) => history.push(entry),
        _ => {
            return Err(super::queries::malformed(
                id,
                "stateHistory",
                "expected an array",
            ))
        }
    }
    Ok(())
}
