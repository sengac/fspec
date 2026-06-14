//! Foundation schema gate — thin wrapper over the shared, spec-compliant
//! JSON Schema engine ([`crate::validators::json_schema`]).
//!
//! Ports the Ajv-backed `validateFoundationJson` from
//! `src/validators/json-schema.ts`. The six Event-Storm foundation-mutation
//! commands call `generate-foundation-md` as a best-effort auto-regenerate
//! step; that helper validates `foundation.json` against
//! `generic-foundation.schema.json` and refuses to (re)write `FOUNDATION.md`
//! when validation fails. `validate-foundation-schema` and `update-foundation`
//! share the same gate.
//!
//! The bundled schema is embedded via `include_str!` and compiled exactly once
//! (cached in a [`OnceLock`]). Error strings mirror Ajv's
//! `"<instancePath>: <message>"` rendering so callers can surface them
//! verbatim. The actual validation — regex `pattern`s, `format` assertion,
//! `oneOf`/`const`, `minItems`, etc. — is performed by the [`jsonschema`]
//! crate, NOT by bespoke traversal code.

use serde_json::Value;

use crate::validators::json_schema::validate_against_schema;
pub use crate::validators::json_schema::{join_errors, SchemaError};

/// Embedded copy of `src/schemas/generic-foundation.schema.json`.
const FOUNDATION_SCHEMA_SRC: &str = include_str!("generic-foundation.schema.json");

/// Validate `data` against the bundled generic-foundation schema.
///
/// Returns `Ok(())` when valid; otherwise the full ordered list of Ajv-style
/// errors (mirroring `allErrors: true`).
pub fn validate_foundation(data: &Value) -> Result<(), Vec<SchemaError>> {
    validate_against_schema(data, FOUNDATION_SCHEMA_SRC)
}

/// Join schema errors into the TS `formatValidationErrors`-equivalent string
/// used by `generate-foundation-md`: each error as `instancePath: message`
/// (empty path stays empty), joined by `"; "`.
pub fn format_errors(errors: &[SchemaError]) -> String {
    join_errors(errors, "; ")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn valid_foundation() -> Value {
        json!({
            "version": "2.0.0",
            "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
            "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
        })
    }

    #[test]
    fn accepts_valid_minimal_foundation() {
        assert!(validate_foundation(&valid_foundation()).is_ok());
    }

    #[test]
    fn rejects_empty_capabilities() {
        let mut f = valid_foundation();
        f["solutionSpace"]["capabilities"] = json!([]);
        let errs = validate_foundation(&f).unwrap_err();
        assert!(errs.iter().any(|e| e.instance_path == "/solutionSpace/capabilities"
            && e.message == "must NOT have fewer than 1 items"));
    }

    #[test]
    fn rejects_bad_version_pattern() {
        let mut f = valid_foundation();
        f["version"] = json!("2.0");
        let errs = validate_foundation(&f).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.instance_path == "/version" && e.message.starts_with("must match pattern")));
    }

    #[test]
    fn rejects_missing_required_top_level() {
        let mut f = valid_foundation();
        f.as_object_mut().unwrap().remove("solutionSpace");
        let errs = validate_foundation(&f).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message == "must have required property 'solutionSpace'"));
    }

    #[test]
    fn rejects_additional_properties_at_root() {
        let mut f = valid_foundation();
        f["junk"] = json!(1);
        let errs = validate_foundation(&f).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message == "must NOT have additional properties"));
    }

    #[test]
    fn accepts_valid_event_storm_bounded_context() {
        let mut f = valid_foundation();
        f["eventStorm"] = json!({
            "level": "big_picture",
            "items": [{
                "id": 1, "type": "bounded_context", "text": "WM",
                "color": null, "deleted": false,
                "createdAt": "2024-01-01T00:00:00.000Z"
            }],
            "nextItemId": 2
        });
        assert!(validate_foundation(&f).is_ok());
    }

    #[test]
    fn rejects_bounded_context_with_non_null_color() {
        let mut f = valid_foundation();
        f["eventStorm"] = json!({
            "level": "big_picture",
            "items": [{
                "id": 1, "type": "bounded_context", "text": "WM",
                "color": "pink", "deleted": false,
                "createdAt": "2024-01-01T00:00:00.000Z"
            }],
            "nextItemId": 2
        });
        let errs = validate_foundation(&f).unwrap_err();
        let rendered = format_errors(&errs);
        assert!(rendered.contains("/eventStorm/items/0"));
    }
}
