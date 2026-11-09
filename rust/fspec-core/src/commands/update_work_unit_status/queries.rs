//! Work-unit field accessors for `update-work-unit-status` (typed fields live in `extra`).

use serde_json::Value;

use super::transitions::msg;
use crate::error::FspecCoreError;

pub(super) type Data = crate::types::work_unit::WorkUnitsData;

/// BUG-385: an array-valued `extra` field. Absent means none (fspec omits
/// empty arrays); present with another shape is an error naming the field.
pub(super) fn array_field<'a>(
    data: &'a Data,
    id: &str,
    key: &str,
) -> Result<&'a [Value], FspecCoreError> {
    match data.work_units.get(id).and_then(|wu| wu.extra.get(key)) {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(malformed(id, key, "expected an array")),
    }
}

/// The error for a malformed work-unit field.
pub(super) fn malformed(id: &str, key: &str, problem: &str) -> FspecCoreError {
    msg(format!(
        "Work unit {id} has a malformed {key} field: {problem}. Fix spec/work-units.json."
    ))
}

/// Length of an array-valued `extra` field.
pub(super) fn array_len(data: &Data, id: &str, key: &str) -> Result<usize, FspecCoreError> {
    array_field(data, id, key).map(<[Value]>::len)
}

/// String items of an array-valued `extra` field; a non-string item is an
/// error naming the field (BUG-385).
pub(super) fn string_array<'a>(
    data: &'a Data,
    id: &str,
    key: &str,
) -> Result<Vec<&'a str>, FspecCoreError> {
    array_field(data, id, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or_else(|| malformed(id, key, "expected an array of strings"))
        })
        .collect()
}

pub(super) fn has_examples(data: &Data, id: &str) -> Result<bool, FspecCoreError> {
    Ok(array_len(data, id, "examples")? > 0)
}

pub(super) fn has_estimate(data: &Data, id: &str) -> bool {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get("estimate"))
        .map(|v| !v.is_null())
        .unwrap_or(false)
}

pub(super) fn is_parent_work_unit(data: &Data, id: &str) -> Result<bool, FspecCoreError> {
    Ok(array_len(data, id, "children")? > 0)
}

/// Active blockers: `blockedBy` entries whose target work unit is not `done`.
/// Each formatted as `ID (status: S)` to mirror the TS message.
pub(super) fn collect_active_blockers(
    data: &Data,
    id: &str,
) -> Result<Vec<String>, FspecCoreError> {
    let mut out = Vec::new();
    for blocker_id in string_array(data, id, "blockedBy")? {
        if let Some(blocker) = data.work_units.get(blocker_id) {
            let status = blocker.status.as_str();
            if status != "done" {
                out.push(format!("{blocker_id} (status: {status})"));
            }
        }
    }
    Ok(out)
}

/// Incomplete children: `children` entries whose work unit is not `done`.
pub(super) fn incomplete_children(data: &Data, id: &str) -> Result<Vec<String>, FspecCoreError> {
    let mut out = Vec::new();
    for child_id in string_array(data, id, "children")? {
        if let Some(child) = data.work_units.get(child_id) {
            let status = child.status.as_str();
            if status != "done" {
                out.push(format!("{child_id} (status: {status})"));
            }
        }
    }
    Ok(out)
}

/// Soft-dependency warning for `dependsOn` entries that are not `done`.
pub(super) fn soft_dependency_warning(
    data: &Data,
    id: &str,
) -> Result<Option<String>, FspecCoreError> {
    let incomplete: Vec<String> = string_array(data, id, "dependsOn")?
        .into_iter()
        .filter_map(|dep_id| {
            data.work_units.get(dep_id).and_then(|dep| {
                let status = dep.status.as_str();
                (status != "done").then(|| format!("{dep_id} (status: {status})"))
            })
        })
        .collect();
    if incomplete.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "Work unit has soft dependencies that are not complete: {}. Consider completing dependencies first for better workflow.",
        incomplete.join(", ")
    )))
}

/// Block the transition if any non-deleted, non-selected questions remain.
pub(super) fn check_unanswered_questions(
    data: &Data,
    id: &str,
    current: &str,
    new: &str,
) -> Result<(), FspecCoreError> {
    let questions = array_field(data, id, "questions")?;
    let mut lines = Vec::new();
    for (idx, q) in questions.iter().enumerate() {
        let deleted = q.get("deleted").and_then(Value::as_bool).unwrap_or(false);
        let selected = q.get("selected").and_then(Value::as_bool).unwrap_or(false);
        if !deleted && !selected {
            let text = q.get("text").and_then(Value::as_str).ok_or_else(|| {
                malformed(id, "questions", &format!("question {idx} has no text"))
            })?;
            lines.push(format!("  - [{idx}] {text}"));
        }
    }
    if !lines.is_empty() {
        return Err(msg(format!(
            "Unanswered questions prevent state transition from '{current}' to '{new}':\n{}\n\nAnswer questions with 'fspec answer-question {id} <index>' before moving to testing.",
            lines.join("\n")
        )));
    }
    Ok(())
}
