//! `validate-work-units` — Rust port of `src/commands/validate-work-units.ts`
//! (RPC-325).
//!
//! Validates the integrity of `spec/work-units.json`: schema shape, unique
//! ids, parent/child consistency, valid status values, state-array
//! consistency, Example-Mapping field shapes, and dependency-field shapes.
//!
//! ## Raw-Value validation (RPC-325 decision)
//! Validation runs over the RAW `serde_json::Value` rather than the typed
//! [`crate::types::work_unit::WorkUnitsData`] so that schema violations a
//! typed deserialise would silently default away (missing `workUnits`,
//! non-object `states`, a dependency field typed as a string, …) are
//! surfaced exactly as the TypeScript reference reports them.
//!
//! The file is read directly as an untyped `Value`. When `spec/work-units.json`
//! is MISSING we first call [`crate::io::ensure::ensure_work_units_file`] purely
//! for its create-the-canonical-empty-store side effect, then re-read the raw
//! bytes. When the file is PRESENT we deliberately skip the typed loader — it
//! would reject fixtures missing required fields (e.g. `createdAt`) at parse
//! time, BEFORE the raw-Value schema checks could surface them. Genuinely
//! malformed JSON still escalates as a parse error.
//!
//! ## Result envelope
//! Returns `{valid, checks, errors?}` via `Ok(json)`; the dispatcher derives
//! `success=true` from the `Ok`. The CLI bridge owns rendering + exit code
//! (`✓ All work units are valid` / `✗ Found N validation errors` to stderr,
//! exit 1).
//!
//! ## Two-front-doors
//! Both the LLM dispatcher AND the clap subcommand call this single function
//! (RPC-003 §7/§11).

use std::path::Path;

use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

const ALLOWED_STATES: &[&str] = &[
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

/// Dispatcher entry point. `args_json` carries the documented-only `--fix`
/// flag (no functional behaviour in either the TS reference or this port).
pub async fn run(_args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let path = project_root.join("spec").join("work-units.json");

    // Auto-create the canonical default ONLY when the file is missing — this is
    // the load-or-init side effect that keeps the empty-store path working.
    //
    // When the file already EXISTS we must NOT route through the typed
    // [`ensure_work_units_file`] loader: it deserialises into the strongly-typed
    // [`WorkUnitsData`], which rejects fixtures missing required fields (e.g.
    // `createdAt`) at parse time — BEFORE our raw-Value schema checks can run.
    // Validation has to inspect the RAW bytes so malformed/invalid data reaches
    // the checks instead of being rejected up front (RPC-325 decision).
    if !path.exists() {
        ensure_work_units_file(project_root)?;
    }

    // Read the on-disk bytes as an untyped Value for raw inspection. JSON-syntax
    // errors escalate (parse error); typed field presence is NOT required here.
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "validate-work-units",
        source,
    })?;
    let data: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "work-units.json".to_string(),
        reason: e.to_string(),
    })?;

    let mut errors: Vec<String> = Vec::new();
    let mut checks: Vec<&'static str> = Vec::new();

    // ---- Check 1: schema shape ----
    checks.push("schema");
    let work_units_opt = data.get("workUnits").and_then(Value::as_object);
    let states_opt = data.get("states").and_then(Value::as_object);

    // ---- TS crash parity (Check 2) ----
    // The TypeScript reference does NOT short-circuit after the schema check.
    // Check 2 calls `Object.keys(workUnitsData.workUnits)`, which throws when
    // `workUnits` is missing/non-object → uncaught TypeError rendered by the
    // `.action` catch as `✗ Failed to validate work units: Cannot convert
    // undefined or null to object` (exit 1). Reproduce that VERBATIM before any
    // structured error can be surfaced.
    let work_units: Map<String, Value> = match &work_units_opt {
        Some(m) => (*m).clone(),
        None => {
            return Err(FspecCoreError::Message(
                "Cannot convert undefined or null to object".to_string(),
            ));
        }
    };
    let states: Map<String, Value> = match &states_opt {
        Some(m) => (*m).clone(),
        None => {
            errors.push(
                "Invalid work units data structure: missing or invalid states field".to_string(),
            );
            Map::new()
        }
    };

    // ---- Check 2: unique ids (JSON object keys are inherently unique) ----
    checks.push("uniqueIds");

    // ---- Check 3: parent/child consistency ----
    checks.push("parentChild");
    for (id, wu) in &work_units {
        // Parent existence + back-link.
        if let Some(parent_id) = wu.get("parent").and_then(Value::as_str) {
            match work_units.get(parent_id) {
                None => {
                    // TS pushes the "non-existent parent" error but does NOT
                    // `continue`; it then dereferences
                    // `workUnitsData.workUnits[parent].children` on the
                    // `undefined` parent → TypeError. Reproduce that crash
                    // VERBATIM (the structured error is never surfaced).
                    return Err(FspecCoreError::Message(
                        "Cannot read properties of undefined (reading 'children')".to_string(),
                    ));
                }
                Some(parent_wu) => {
                    let listed = parent_wu
                        .get("children")
                        .and_then(Value::as_array)
                        .map(|c| c.iter().any(|x| x.as_str() == Some(id.as_str())))
                        .unwrap_or(false);
                    if !listed {
                        errors.push(format!(
                            "Work unit {id} has parent {parent_id}, but parent doesn't list it as a child"
                        ));
                    }
                }
            }
        }

        // Child existence + back-link.
        if let Some(children) = wu.get("children").and_then(Value::as_array) {
            for child in children {
                let Some(child_id) = child.as_str() else { continue };
                match work_units.get(child_id) {
                    None => {
                        errors.push(format!(
                            "Work unit {id} references non-existent child: {child_id}"
                        ));
                    }
                    Some(child_wu) => {
                        if child_wu.get("parent").and_then(Value::as_str) != Some(id.as_str()) {
                            errors.push(format!(
                                "Work unit {id} lists {child_id} as child, but child doesn't have it as parent"
                            ));
                        }
                    }
                }
            }
        }
    }

    // ---- Check 4: valid status values ----
    for (id, wu) in &work_units {
        let status = wu.get("status").and_then(Value::as_str).unwrap_or("");
        if !ALLOWED_STATES.contains(&status) {
            errors.push(format!(
                "Invalid status value for {id}: {status}. Allowed values: {}",
                ALLOWED_STATES.join(", ")
            ));
        }
    }

    // ---- Check 5: state-array consistency ----
    // TS Check 5 reads `workUnitsData.states[status]`. When `states` is
    // missing/non-object AND there is ≥1 work unit to iterate, the FIRST unit
    // triggers a TypeError → `Cannot read properties of undefined (reading
    // '<status>')`, where `<status>` is that unit's status value. With zero
    // work units the loop body never runs, so no crash (handled by the normal
    // structured "missing states" error). Reproduce the crash VERBATIM.
    if states_opt.is_none() {
        if let Some((_id, wu)) = work_units.iter().next() {
            let status = wu.get("status").and_then(Value::as_str).unwrap_or("");
            return Err(FspecCoreError::Message(format!(
                "Cannot read properties of undefined (reading '{status}')"
            )));
        }
    }
    for (id, wu) in &work_units {
        let status = wu.get("status").and_then(Value::as_str).unwrap_or("");
        let in_own = states
            .get(status)
            .and_then(Value::as_array)
            .map(|arr| arr.iter().any(|x| x.as_str() == Some(id.as_str())))
            .unwrap_or(false);
        if !in_own {
            errors.push(format!(
                "State consistency error: Work unit {id} has status '{status}' but is not in states.{status} array. Run 'fspec repair-work-units' to fix inconsistencies."
            ));
        }
        for (state_name, state_ids) in &states {
            if state_name == status {
                continue;
            }
            let present = state_ids
                .as_array()
                .map(|arr| arr.iter().any(|x| x.as_str() == Some(id.as_str())))
                .unwrap_or(false);
            if present {
                errors.push(format!(
                    "State consistency error: Work unit {id} has status '{status}' but is in '{state_name}' array. Run 'fspec repair-work-units' to fix inconsistencies."
                ));
            }
        }
    }

    // ---- Check 6: Example-Mapping field shapes ----
    checks.push("exampleMapping");
    for (id, wu) in &work_units {
        validate_string_array(&mut errors, id, "rules", wu.get("rules"));
        validate_string_array(&mut errors, id, "examples", wu.get("examples"));
        validate_questions(&mut errors, id, wu.get("questions"));
        validate_string_array(&mut errors, id, "assumptions", wu.get("assumptions"));
    }

    // ---- Check 7: dependency field shapes ----
    checks.push("dependencies");
    for (id, wu) in &work_units {
        validate_string_array(&mut errors, id, "blocks", wu.get("blocks"));
        validate_string_array(&mut errors, id, "blockedBy", wu.get("blockedBy"));
        validate_string_array(&mut errors, id, "dependsOn", wu.get("dependsOn"));
        validate_string_array(&mut errors, id, "relatesTo", wu.get("relatesTo"));
    }

    let valid = errors.is_empty();
    let mut envelope = json!({ "valid": valid, "checks": checks });
    if !valid {
        envelope["errors"] = json!(errors);
    }
    ok(envelope)
}

/// Validate that an optional field, when present and non-null, is an array of
/// non-empty strings. Mirrors the repeated TS pattern for `rules`, `examples`,
/// `assumptions`, `blocks`, `blockedBy`, `dependsOn`, and `relatesTo`.
fn validate_string_array(errors: &mut Vec<String>, id: &str, field: &str, value: Option<&Value>) {
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

/// Validate the `questions` field — an array of QuestionItem objects
/// `{text, selected, answer?}`. Mirrors the TS questions branch.
fn validate_questions(errors: &mut Vec<String>, id: &str, value: Option<&Value>) {
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
                let answer_ok = answer.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false);
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

/// Serialise the result envelope to the `Ok(String)` returned to the dispatcher.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate-work-units",
        reason: format!("failed to serialise response: {e}"),
    })
}
