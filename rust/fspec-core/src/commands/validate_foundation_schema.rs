//! `validate-foundation-schema` — Rust port of
//! `src/commands/validate-foundation-schema.ts` (RPC-321).
//!
//! Reads `spec/foundation.json` (relative to the supplied `project_root`) and
//! validates it against the bundled `generic-foundation.schema.json` using the
//! hand-rolled structural validator in
//! [`crate::generators::foundation_schema`] — NO JSON-schema crate is added.
//!
//! ## Error-string parity with the TS Ajv path
//!
//! The TS command renders each Ajv error as `path: message` where
//! `path = err.instancePath || err.schemaPath`, with TWO special cases:
//!
//!   1. **minItems** violations are humanised as
//!      `Field <dotted.path> must have at least <limit> items (found <n>)`.
//!      The shared validator emits the generic Ajv message
//!      `must NOT have fewer than <limit> items`; this command layer detects
//!      that prefix, parses `<limit>`, walks the parsed foundation along the
//!      error's `instancePath` to recover the actual array length, and
//!      reformats — keeping the shared validator (used by
//!      `generate-foundation-md`) untouched.
//!   2. **root-level** errors whose `instancePath` is empty fall back to the
//!      Ajv `schemaPath`. For a missing top-level required property Ajv emits
//!      `#/required`, so the message renders as
//!      `#/required: must have required property 'X'` (captured byte-exact
//!      from `node dist/index.js validate-foundation-schema`).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant. The command never `process.exit`s; it returns a
//! structured `{success, output?/error?}` JSON envelope and the shell bridge
//! maps that to exit codes (0 success / 1 failure).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::generators::foundation_schema::{validate_foundation, SchemaError};

/// Dispatcher entry point. `project_root` is passed by both front doors so the
/// same binary can serve multiple working directories without touching
/// `std::env::current_dir()`.
pub async fn run(_args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let foundation_path = project_root.join("spec").join("foundation.json");

    // Read the foundation document. ENOENT → friendly "not found" message
    // (parity with the TS `errorMessage.includes('ENOENT')` branch); any other
    // read failure falls into the generic catch wrapper below.
    let content = match std::fs::read_to_string(&foundation_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(soft_error("foundation.json not found in spec/ directory"));
        }
        Err(e) => {
            return Ok(soft_error(&format!(
                "Failed to validate foundation schema: {e}"
            )));
        }
    };

    // Parse JSON. A malformed document is surfaced via the generic
    // `Failed to validate foundation schema: <message>` wrapper (parity with
    // the TS `JSON.parse` throwing into the outer catch).
    let foundation: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return Ok(soft_error(&format!(
                "Failed to validate foundation schema: {e}"
            )));
        }
    };

    match validate_foundation(&foundation) {
        Ok(()) => Ok(serde_json::to_string(&json!({
            "success": true,
            "output": "✓ foundation.json is valid according to the schema",
        }))
        .unwrap_or_default()),
        Err(errors) => {
            let messages: Vec<String> = errors
                .iter()
                .map(|e| format_error(e, &foundation))
                .collect();
            Ok(soft_error(&messages.join("\n")))
        }
    }
}

/// Build the `{success:false, error}` envelope as a JSON string.
fn soft_error(message: &str) -> String {
    serde_json::to_string(&json!({
        "success": false,
        "error": message,
    }))
    .unwrap_or_default()
}

/// Render a single [`SchemaError`] in the TS-equivalent friendly form.
///
/// Mirrors the `errors.map(...)` block at
/// `src/commands/validate-foundation-schema.ts:79-89`.
fn format_error(err: &SchemaError, foundation: &Value) -> String {
    // minItems special case: the shared validator emits
    // "must NOT have fewer than <limit> items".
    if let Some(limit) = err
        .message
        .strip_prefix("must NOT have fewer than ")
        .and_then(|rest| rest.strip_suffix(" items"))
    {
        // `arrayPath = path.replace(/^\//, '').replace(/\//g, '.')`
        let dotted = err.instance_path.trim_start_matches('/').replace('/', ".");
        let actual = resolve_instance(foundation, &err.instance_path)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        return format!("Field {dotted} must have at least {limit} items (found {actual})");
    }

    // General case: `path = err.instancePath || err.schemaPath`.
    let path = if err.instance_path.is_empty() {
        schema_path_fallback(&err.message)
    } else {
        err.instance_path.clone()
    };
    format!("{path}: {}", err.message)
}

/// Reconstruct the Ajv `schemaPath` for a root-level error (empty
/// `instancePath`). Ajv renders the keyword that failed as `#/<keyword>`; the
/// keyword is derived from the message text. Only the `required` case is
/// exercised by the captured fixtures (`#/required`); the remaining arms are
/// best-effort parity for other root-level keywords.
fn schema_path_fallback(message: &str) -> String {
    let keyword = if message.starts_with("must have required property") {
        "required"
    } else if message == "must NOT have additional properties" {
        "additionalProperties"
    } else if message.contains("must match exactly one schema in oneOf") {
        "oneOf"
    } else if message.starts_with("must match pattern") {
        "pattern"
    } else if message.starts_with("must match format") {
        "format"
    } else if message.starts_with("must be ") {
        "type"
    } else {
        ""
    };
    format!("#/{keyword}")
}

/// Walk the parsed foundation along a JSON-pointer-like `instance_path`
/// (e.g. `/solutionSpace/capabilities` or `/items/0`). Numeric segments index
/// arrays; everything else indexes objects.
fn resolve_instance<'a>(root: &'a Value, instance_path: &str) -> Option<&'a Value> {
    let mut current = root;
    for seg in instance_path.split('/').filter(|s| !s.is_empty()) {
        current = if let Ok(idx) = seg.parse::<usize>() {
            current.get(idx)?
        } else {
            current.get(seg)?
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn valid_foundation() -> Value {
        json!({
            "version": "2.0.0",
            "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
            "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
        })
    }

    #[test]
    fn min_items_reformats_with_dotted_path_and_actual_length() {
        let mut f = valid_foundation();
        f["solutionSpace"]["capabilities"] = json!([]);
        let err = SchemaError {
            instance_path: "/solutionSpace/capabilities".to_string(),
            message: "must NOT have fewer than 1 items".to_string(),
        };
        assert_eq!(
            format_error(&err, &f),
            "Field solutionSpace.capabilities must have at least 1 items (found 0)"
        );
    }

    #[test]
    fn root_required_falls_back_to_schema_path() {
        let f = valid_foundation();
        let err = SchemaError {
            instance_path: String::new(),
            message: "must have required property 'solutionSpace'".to_string(),
        };
        assert_eq!(
            format_error(&err, &f),
            "#/required: must have required property 'solutionSpace'"
        );
    }

    #[test]
    fn non_root_error_uses_instance_path() {
        let f = valid_foundation();
        let err = SchemaError {
            instance_path: "/version".to_string(),
            message: "must match pattern \"^\\d+\\.\\d+\\.\\d+$\"".to_string(),
        };
        assert_eq!(
            format_error(&err, &f),
            "/version: must match pattern \"^\\d+\\.\\d+\\.\\d+$\""
        );
    }

    #[test]
    fn resolve_instance_walks_objects_and_arrays() {
        let f = valid_foundation();
        let node = resolve_instance(&f, "/solutionSpace/capabilities/0/name").unwrap();
        assert_eq!(node.as_str(), Some("C"));
    }
}
