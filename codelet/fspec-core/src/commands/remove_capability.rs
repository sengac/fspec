//! `remove-capability` — Rust port of `src/commands/remove-capability.ts`
//! (RPC-269).
//!
//! Removes the first capability whose `name` is an EXACT, case-sensitive
//! match from a foundation file's `solutionSpace.capabilities` array.
//!
//! ## Semantics (parity with TS)
//!
//! 1. **Draft precedence**: `spec/foundation.json.draft` wins over
//!    `spec/foundation.json` when present (TS `fs.access` probe at
//!    `remove-capability.ts:18-24`).
//! 2. **Missing file**: chosen target missing → canonical
//!    `"foundation.json not found"` error WITHOUT auto-creating.
//! 3. **No capabilities**: missing or empty `capabilities` → error message
//!    `Capability "<name>" not found` + detail line
//!    `No capabilities exist in foundation`.
//! 4. **Not found**: name has no exact case-sensitive match → error message
//!    `Capability "<name>" not found` + detail line
//!    `Available capabilities: <comma-joined names>`.
//! 5. On success: remove the FIRST matching entry (TS `findIndex` + `splice`),
//!    write atomically with a trailing newline.
//!
//! ## Result payload (becomes `DispatchResult.data`)
//!
//! `{ "success": true, "fileName": String }`
//!
//! The CLI bridge at `codelet/fspec/src/remove_capability.rs` consumes this to
//! render the TS-canonical stdout line. ALL draft probing, matching, and JSON
//! mutation live HERE — the bridge is marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic_trailing_newline;

const COMMAND: &str = "remove-capability";

/// CLI arguments accepted by `remove-capability`. Mirrors the TS positional
/// arg `<name>`.
#[derive(Debug, Deserialize)]
struct RemoveCapabilityArgs {
    name: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveCapabilityArgs =
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
        reason: e.to_string(),
    })?;

    let name = &args.name;

    // TS dereferences `foundation.solutionSpace.capabilities` directly. Missing
    // or null `solutionSpace` throws a TypeError BEFORE any `output.error`
    // line is emitted; `register-remove-capability` swallows the rethrow and
    // only `process.exit(1)`, so the shell sees EMPTY stderr + exit 1. Mirror
    // those JS TypeErrors as bare `InvalidArgs` reasons that the bridge maps to
    // a silent failure.
    let solution = data.get_mut("solutionSpace");
    let solution = match solution {
        None => {
            return Err(js_silent_type_error(
                "Cannot read properties of undefined (reading 'capabilities')",
            ));
        }
        Some(Value::Null) => {
            return Err(js_silent_type_error(
                "Cannot read properties of null (reading 'capabilities')",
            ));
        }
        Some(v) => v,
    };

    // A primitive `solutionSpace` (string/number/bool) yields `undefined`
    // capabilities → falls into the falsy "No capabilities exist" branch below.
    // [3] TS: `if (!caps || caps.length === 0)`. The `!caps` guard fires for
    // falsy values (undefined / null / "" / 0 / false); `caps.length === 0`
    // fires only for an EMPTY ARRAY (other objects/strings have a defined but
    // non-zero — or undefined — length and so fall through).
    let no_capabilities = match solution.as_object().and_then(|o| o.get("capabilities")) {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f == 0.0).unwrap_or(false),
        Some(Value::String(s)) => s.is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(_)) => false,
    };
    if no_capabilities {
        return Err(FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: format!(
                "Capability \"{name}\" not found\n  No capabilities exist in foundation"
            ),
        });
    }

    // A truthy non-array `capabilities` (object / number / non-empty string)
    // reaches `capabilities.findIndex(...)` in TS → TypeError thrown before any
    // `output.error`, so the shell sees EMPTY stderr + exit 1.
    let is_array = matches!(
        solution.as_object().and_then(|o| o.get("capabilities")),
        Some(Value::Array(_))
    );
    if !is_array {
        return Err(js_silent_type_error(
            "foundation.solutionSpace.capabilities.findIndex is not a function",
        ));
    }

    let caps = solution
        .as_object_mut()
        .and_then(|s| s.get_mut("capabilities"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| FspecCoreError::ParseJson {
            file: file_name.to_string(),
            reason: "solutionSpace.capabilities must be an array".to_string(),
        })?;

    // [4] First exact case-sensitive match (TS findIndex).
    let index = caps
        .iter()
        .position(|c| c.get("name").and_then(Value::as_str) == Some(name.as_str()));
    let Some(index) = index else {
        // TS builds the list with `caps.map(c => c.name).join(', ')`.
        // `Array.prototype.join` coerces each element via String(): missing /
        // null names render as "", numbers/booleans as their literal text, and
        // nested objects/arrays via JS's own toString. Mirror that coercion so
        // a malformed `name` produces identical output.
        let available: Vec<String> = caps.iter().map(|c| js_join_coerce(c.get("name"))).collect();
        return Err(FspecCoreError::InvalidArgs {
            command: COMMAND,
            reason: format!(
                "Capability \"{name}\" not found\n  Available capabilities: {}",
                available.join(", ")
            ),
        });
    };

    // [5] Remove the first match and write.
    caps.remove(index);

    write_json_atomic_trailing_newline(&target_path, &data)?;

    serde_json::to_string(&json!({
        "success": true,
        "fileName": file_name,
    }))
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Sentinel prefix marking a reason that mirrors an unguarded JS `TypeError`.
///
/// `register-remove-capability` wraps the whole body in `try/catch` and the
/// catch ONLY calls `process.exit(1)` — it never prints the message. So when
/// `removeCapability` throws a TypeError (malformed `solutionSpace` /
/// `capabilities`) the shell sees EMPTY stderr and exit 1. We surface those as
/// `InvalidArgs` carrying this prefix so the CLI bridge can suppress output.
pub const SILENT_TYPE_ERROR_PREFIX: &str = "\u{0}silent-type-error\u{0}";

/// Build a silent (output-suppressed) JS TypeError parity error.
fn js_silent_type_error(message: &str) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: COMMAND,
        reason: format!("{SILENT_TYPE_ERROR_PREFIX}{message}"),
    }
}

/// Coerce a capability `name` exactly as `Array.prototype.join` does via
/// `String(value)`:
///   - missing / null  → `""`
///   - string          → itself
///   - number/bool     → literal text (`42`, `3.5`, `true`)
///   - array           → its elements join-coerced and comma-joined
///   - object          → `"[object Object]"`
fn js_join_coerce(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| js_join_coerce(Some(v)))
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_minimal() {
        let a: RemoveCapabilityArgs = serde_json::from_str(r#"{"name":"Search"}"#).unwrap();
        assert_eq!(a.name, "Search");
    }

    #[test]
    fn join_coerce_matches_js_string() {
        assert_eq!(js_join_coerce(None), "");
        assert_eq!(js_join_coerce(Some(&Value::Null)), "");
        assert_eq!(js_join_coerce(Some(&json!("S"))), "S");
        assert_eq!(js_join_coerce(Some(&json!(42))), "42");
        assert_eq!(js_join_coerce(Some(&json!(3.5))), "3.5");
        assert_eq!(js_join_coerce(Some(&json!(true))), "true");
        assert_eq!(js_join_coerce(Some(&json!({"a":1}))), "[object Object]");
        assert_eq!(js_join_coerce(Some(&json!([1, 2, 3]))), "1,2,3");
        assert_eq!(
            js_join_coerce(Some(&json!([1, [2, 3], {"x": 1}]))),
            "1,2,3,[object Object]"
        );
    }
}
