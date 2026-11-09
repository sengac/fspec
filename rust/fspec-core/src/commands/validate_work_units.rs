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

use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

mod fields;
use fields::{
    is_js_object_like, js_object_entries, ok, validate_item_array, validate_questions,
    validate_string_array,
};

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
    let path = crate::io::spec_paths::work_units(project_root)?;

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
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    let mut errors: Vec<String> = Vec::new();
    let mut checks: Vec<&'static str> = Vec::new();

    // ---- Check 1: schema shape ----
    // TS: `if (!workUnits || typeof workUnits !== 'object')` and the same for
    // `states`. In JS `typeof []` is `'object'`, `typeof null` is `'object'`
    // but `!null` is truthy — so the check fires for missing/null/scalars and
    // passes for objects AND arrays. `is_js_object_like` reproduces that, so an
    // empty `workUnits: []` is treated as valid (parity) instead of crashing.
    checks.push("schema");
    let work_units_val = data.get("workUnits");
    let states_val = data.get("states");
    if !is_js_object_like(work_units_val) {
        errors.push(
            "Invalid work units data structure: missing or invalid workUnits field".to_string(),
        );
    }
    if !is_js_object_like(states_val) {
        errors
            .push("Invalid work units data structure: missing or invalid states field".to_string());
    }

    // ---- TS crash parity (Check 2) ----
    // Check 2 calls `Object.keys(workUnitsData.workUnits)`, which throws ONLY
    // when `workUnits` is null/undefined → uncaught TypeError rendered by the
    // `.action` catch as `✗ Failed to validate work units: Cannot convert
    // undefined or null to object` (exit 1). For every other type JS coerces to
    // enumerable own entries (arrays → index keys, strings → char-index keys,
    // scalars → none), so we reproduce `Object.entries` semantics rather than
    // collapsing all non-object types to the throw.
    let work_units = match js_object_entries(work_units_val) {
        Some(entries) => entries,
        None => {
            return Err(FspecCoreError::Message(
                "Cannot convert undefined or null to object".to_string(),
            ));
        }
    };
    // `states` is only ever index-dereferenced (never `Object.keys`-ed), so a
    // nullish value does not throw here — it throws later at Check 5 and only if
    // there is a work unit to iterate. Coerce with the same JS-entries
    // semantics; a nullish `states` yields an empty map.
    let states_is_nullish = matches!(states_val, None | Some(Value::Null));
    let states = js_object_entries(states_val).unwrap_or_default();

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
                let Some(child_id) = child.as_str() else {
                    continue;
                };
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
    if states_is_nullish {
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
        validate_item_array(&mut errors, id, "rules", wu.get("rules"));
        validate_item_array(&mut errors, id, "examples", wu.get("examples"));
        validate_questions(&mut errors, id, wu.get("questions"));
        // BUG-633: assumptions are strings or stable-index items, like rules.
        validate_item_array(&mut errors, id, "assumptions", wu.get("assumptions"));
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
