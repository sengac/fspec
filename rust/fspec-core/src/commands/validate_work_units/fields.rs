//! Field-shape helpers for `validate-work-units` (split from the command
//! module to keep it under 300 lines).

use serde_json::{Map, Value};

use crate::error::FspecCoreError;

/// Validate that an optional field, when present and non-null, is an array of
/// non-empty strings. Mirrors the repeated TS pattern for `rules`, `examples`,
/// `blocks`, `blockedBy`, `dependsOn`, and `relatesTo`.
pub(super) fn validate_string_array(
    errors: &mut Vec<String>,
    id: &str,
    field: &str,
    value: Option<&Value>,
) {
    let Some(value) = value else { return };
    if value.is_null() {
        return;
    }
    let Some(arr) = value.as_array() else {
        errors.push(format!("Work unit {id}: {field} must be an array"));
        return;
    };
    for (i, el) in arr.iter().enumerate() {
        let empty_or_non_string = match el.as_str() {
            Some(s) => s.trim().is_empty(),
            None => true,
        };
        if empty_or_non_string {
            errors.push(format!(
                "Work unit {id}: {field} array contains empty strings or non-strings at index {i}"
            ));
        }
    }
}

/// BUG-286: validate `rules` / `examples`, which fspec stores as stable-index
/// objects `{id, text, deleted?, ...}`. A legacy plain string is still
/// accepted; an object needs non-empty `text` unless it is soft-deleted.
pub(super) fn validate_item_array(
    errors: &mut Vec<String>,
    id: &str,
    field: &str,
    value: Option<&Value>,
) {
    let Some(arr) = value.and_then(|value| (!value.is_null()).then_some(value)) else {
        return;
    };
    let Some(arr) = arr.as_array() else {
        errors.push(format!("Work unit {id}: {field} must be an array"));
        return;
    };
    for (i, el) in arr.iter().enumerate() {
        if let Some(text) = el.as_str() {
            if text.trim().is_empty() {
                errors.push(format!(
                    "Work unit {id}: {field} array contains empty strings or non-strings at index {i}"
                ));
            }
            continue;
        }
        let Some(item) = el.as_object() else {
            errors.push(format!(
                "Work unit {id}: {field}[{i}] must be a string or a stable-index object"
            ));
            continue;
        };
        if item.get("deleted").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let has_text = item
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty());
        if !has_text {
            errors.push(format!(
                "Work unit {id}: {field}[{i}] text must be a non-empty string"
            ));
        }
    }
}

/// Validate the `questions` field — an array of QuestionItem objects
/// `{text, selected, answer?}`. Mirrors the TS questions branch.
pub(super) fn validate_questions(errors: &mut Vec<String>, id: &str, value: Option<&Value>) {
    let Some(value) = value else { return };
    if value.is_null() {
        return;
    }
    let Some(arr) = value.as_array() else {
        errors.push(format!("Work unit {id}: questions must be an array"));
        return;
    };
    for (i, q) in arr.iter().enumerate() {
        let Some(obj) = q.as_object() else {
            errors.push(format!(
                "Work unit {id}: questions[{i}] must be a QuestionItem object with {{text, selected, answer?}}, got {}",
                json_type_name(q)
            ));
            continue;
        };
        let text_ok = obj
            .get("text")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !text_ok {
            errors.push(format!(
                "Work unit {id}: questions[{i}].text must be a non-empty string"
            ));
        }
        if obj.get("selected").and_then(Value::as_bool).is_none() {
            errors.push(format!(
                "Work unit {id}: questions[{i}].selected must be a boolean"
            ));
        }
        if let Some(answer) = obj.get("answer") {
            if !answer.is_null() {
                let answer_ok = answer
                    .as_str()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                if !answer_ok {
                    errors.push(format!(
                        "Work unit {id}: questions[{i}].answer must be a non-empty string if provided"
                    ));
                }
            }
        }
    }
}

/// JavaScript-style `typeof` name for an error message (objects/null already
/// handled by the caller's `as_object` guard, so this covers the scalar arms).
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "object",
        Value::Object(_) => "object",
    }
}

/// JS `!!value && typeof value === 'object'` — true for objects and arrays
/// (matching `typeof [] === 'object'`), false for missing/null/scalars (the
/// `!value` guard rejects `null` even though `typeof null === 'object'`).
pub(super) fn is_js_object_like(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Object(_)) | Some(Value::Array(_)))
}

/// Reproduce JS `Object.entries(value)` enumerable-own-property semantics.
///
/// Returns `None` only for `null`/`undefined` (where `Object.keys` throws);
/// otherwise the coerced entry map: objects as-is, arrays/strings keyed by
/// stringified index, scalars (number/bool) as the empty map.
pub(super) fn js_object_entries(value: Option<&Value>) -> Option<Map<String, Value>> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Object(m)) => Some(m.clone()),
        Some(Value::Array(arr)) => Some(
            arr.iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
        ),
        Some(Value::String(s)) => Some(
            s.chars()
                .enumerate()
                .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
                .collect(),
        ),
        Some(_) => Some(Map::new()),
    }
}

/// Serialise the result envelope to the `Ok(String)` returned to the dispatcher.
pub(super) fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate-work-units",
        reason: format!("failed to serialise response: {e}"),
    })
}
