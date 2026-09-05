//! `add-capability` — Rust port of `src/commands/add-capability.ts` (RPC-173).
//!
//! Appends a capability `{ name, description }` to a foundation file's
//! `solutionSpace.capabilities` array. Capabilities describe WHAT the
//! system can do from the user perspective.
//!
//! ## Semantics (parity with TS)
//!
//! 1. **Draft precedence**: if `spec/foundation.json.draft` exists it is the
//!    write target; otherwise `spec/foundation.json`. (TS `fs.access` probe at
//!    `add-capability.ts:43-49`.)
//! 2. **Missing file**: if the chosen target does not exist → return the
//!    canonical `"foundation.json not found"` error WITHOUT auto-creating the
//!    file (TS `ENOENT` branch at `add-capability.ts:56-67`). This
//!    deliberately diverges from the auto-create `ensure_foundation_file`
//!    helper used by `add-diagram`.
//! 3. **Capabilities array**: seeded to `[]` when absent.
//! 4. **Placeholder pruning**: if the array contains ONLY placeholder
//!    capabilities (any field containing `[QUESTION:` or `[DETECTED:`) the
//!    whole array is cleared first and `removedCount` reports how many were
//!    dropped. A mixed real+placeholder array is left untouched
//!    (`hasOnlyPlaceholders` requires EVERY entry to be a placeholder).
//! 5. On success: push `{ name, description }`, write atomically with a
//!    trailing newline (TS `JSON.stringify(foundation, null, 2) + '\n'`).
//!
//! ## Result payload (becomes `DispatchResult.data`)
//!
//! `{ "success": true, "fileName": String, "name": String,
//!    "description": String, "removedCount": Number }`
//!
//! The CLI bridge at `rust/fspec/src/add_capability.rs` consumes this to
//! render the TS-canonical stdout block. ALL draft probing, placeholder
//! detection, and JSON mutation live HERE — the bridge is marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic_trailing_newline;

const COMMAND: &str = "add-capability";

/// CLI arguments accepted by `add-capability`. Mirrors the TS positional
/// args `<name> <description>`.
#[derive(Debug, Deserialize)]
struct AddCapabilityArgs {
    name: String,
    description: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddCapabilityArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: format!("failed to parse args: {e}"),
        })?;

    // [1] Draft precedence — draft wins when present.
    let draft_path = project_root.join("spec").join("foundation.json.draft");
    let final_path = project_root.join("spec").join("foundation.json");
    let (target_path, file_name) = if draft_path.exists() {
        (draft_path, "foundation.json.draft")
    } else {
        (final_path, "foundation.json")
    };

    // [2] Missing-file guard — do NOT auto-create.
    if !target_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: "foundation.json not found".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&target_path).map_err(|source| FspecCoreError::Io {
        command: COMMAND,
        source,
    })?;
    let mut data: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: file_name.to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    // Ensure top-level object. TS would throw a TypeError reading
    // `.solutionSpace` of a non-object; in practice foundation files are
    // always objects, so a non-object root is reported as a parse failure.
    let root = data
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: file_name.to_string(),
            reason: "top-level value must be a JSON object".to_string(),
        })?;

    // [3a] `foundation.solutionSpace.capabilities` — TS dereferences
    // `solutionSpace` BEFORE touching `capabilities`. Replicate the exact
    // JS runtime TypeErrors so malformed inputs fail identically:
    //   - missing `solutionSpace`  → reading 'capabilities' of undefined
    //   - `solutionSpace: null`    → reading 'capabilities' of null
    //   - `solutionSpace` primitive→ Cannot create property 'capabilities' on <type> '<value>'
    let solution = match root.get_mut("solutionSpace") {
        None => {
            return Err(js_type_error(
                "Cannot read properties of undefined (reading 'capabilities')",
            ));
        }
        Some(Value::Null) => {
            return Err(js_type_error(
                "Cannot read properties of null (reading 'capabilities')",
            ));
        }
        Some(v @ Value::String(_)) | Some(v @ Value::Number(_)) | Some(v @ Value::Bool(_)) => {
            return Err(js_type_error(&format!(
                "Cannot create property 'capabilities' on {} '{}'",
                js_typeof(v),
                js_primitive_display(v)
            )));
        }
        Some(v) => v,
    };
    let solution_obj = solution
        .as_object_mut()
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: file_name.to_string(),
            reason: "solutionSpace must be an object".to_string(),
        })?;

    // TS: `if (!foundation.solutionSpace.capabilities)` initialises only when
    // the value is FALSY (undefined / null / "" / 0 / false). A truthy
    // non-array (object/number/string/true) is LEFT IN PLACE, then
    // `hasOnlyPlaceholders` calls `.every` on it. The bundled (minified) TS
    // names that param `e`, so V8 throws `"e.every is not a function"`.
    let caps_entry = solution_obj.get("capabilities");
    let needs_init = match caps_entry {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f == 0.0).unwrap_or(false),
        Some(Value::String(s)) => s.is_empty(),
        Some(_) => false,
    };
    if needs_init {
        solution_obj.insert("capabilities".to_string(), Value::Array(Vec::new()));
    }
    // Truthy non-array → TS throws on `.every` during the placeholder check.
    let caps = match solution_obj.get_mut("capabilities") {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(js_type_error("e.every is not a function"));
        }
        None => {
            // Unreachable after init, but keep parity-safe behaviour.
            return Err(FspecCoreError::ParseJson {
                file: file_name.to_string(),
                reason: "solutionSpace.capabilities must be an array".to_string(),
            });
        }
    };

    // [4] Prune ONLY when every existing entry is a placeholder.
    let mut removed_count: u64 = 0;
    if !caps.is_empty() && caps.iter().all(is_placeholder_capability) {
        removed_count = caps.len() as u64;
        caps.clear();
    }

    // [5] Append the new capability and write.
    let mut new_cap = Map::new();
    new_cap.insert("name".to_string(), Value::String(args.name.clone()));
    new_cap.insert(
        "description".to_string(),
        Value::String(args.description.clone()),
    );
    caps.push(Value::Object(new_cap));

    write_json_atomic_trailing_newline(&target_path, &data)?;

    // DISC-003 rule 4: universal progress trailer on the success envelope.
    let next_steps = crate::foundation::guidance::draft_trailer(&data);

    serde_json::to_string(&json!({
        "success": true,
        "fileName": file_name,
        "name": args.name,
        "description": args.description,
        "removedCount": removed_count,
        "nextSteps": next_steps,
    }))
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Build the JS-runtime TypeError parity message as an `InvalidArgs` error.
/// The TS command does not guard malformed foundation files, so V8 throws a
/// `TypeError` whose `.message` is printed verbatim by
/// `register-add-capability` (`output.error(err.message)`). Returning it as
/// `InvalidArgs` lets the CLI bridge render the bare reason on stderr.
fn js_type_error(message: &str) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason: message.to_string(),
    }
}

/// JS `typeof` for the primitives we surface in TypeError messages.
fn js_typeof(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        _ => "object",
    }
}

/// Render a JSON primitive the way V8 interpolates it into the
/// `Cannot create property ... on <type> '<value>'` message: strings and
/// booleans print their literal text, numbers print without quotes.
fn js_primitive_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}

/// A capability is a placeholder when its `name` or `description` contains a
/// `[QUESTION:` or `[DETECTED:` marker (TS regex `/\[QUESTION:|\[DETECTED:/`
/// at `add-capability.ts:14`).
fn is_placeholder_capability(cap: &Value) -> bool {
    let field_has_marker = |key: &str| {
        cap.get(key)
            .and_then(Value::as_str)
            .map(|s| s.contains("[QUESTION:") || s.contains("[DETECTED:"))
            .unwrap_or(false)
    };
    field_has_marker("name") || field_has_marker("description")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_minimal() {
        let a: AddCapabilityArgs =
            serde_json::from_str(r#"{"name":"Login","description":"Authenticate"}"#).unwrap();
        assert_eq!(a.name, "Login");
        assert_eq!(a.description, "Authenticate");
    }

    #[test]
    fn placeholder_detection_matches_either_field() {
        assert!(is_placeholder_capability(
            &json!({"name": "[QUESTION: x]", "description": "ok"})
        ));
        assert!(is_placeholder_capability(
            &json!({"name": "ok", "description": "[DETECTED: y]"})
        ));
        assert!(!is_placeholder_capability(
            &json!({"name": "Login", "description": "Authenticate"})
        ));
    }

    #[test]
    fn js_typeof_and_primitive_display() {
        assert_eq!(js_typeof(&json!("x")), "string");
        assert_eq!(js_typeof(&json!(5)), "number");
        assert_eq!(js_typeof(&json!(true)), "boolean");
        assert_eq!(js_primitive_display(&json!("x")), "x");
        assert_eq!(js_primitive_display(&json!(123)), "123");
        assert_eq!(js_primitive_display(&json!(1.5)), "1.5");
        assert_eq!(js_primitive_display(&json!(false)), "false");
    }
}
