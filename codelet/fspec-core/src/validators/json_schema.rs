//! Shared, spec-compliant JSON Schema validation engine.
//!
//! This is the single Rust analogue of the TypeScript
//! `src/validators/json-schema.ts` helper, which wraps **Ajv** (+
//! `ajv-formats`). Here we wrap the [`jsonschema`] crate — the de-facto
//! standard, spec-compliant Rust JSON Schema validator — so that EVERY fspec
//! command that needs schema validation (`validate-foundation-schema`,
//! `generate-foundation-md`, `update-foundation`, `generate-tags-md`) shares
//! ONE real validator instead of duplicating bespoke traversal logic.
//!
//! ## Why not hand-roll?
//! A hand-rolled "draft-07 subset" validator only understands the keywords and
//! literal regexes that happen to appear in today's schemas — any new
//! `pattern`/`format`/keyword silently passes. Delegating to [`jsonschema`]
//! gives real regex matching, real `format` assertion and full draft-07
//! coverage for free.
//!
//! ## Ajv error-string parity
//! Downstream commands render errors as `"<instancePath>: <message>"` where the
//! message text is byte-compatible with Ajv's default messages (e.g.
//! `must NOT have fewer than 1 items`, `must have required property 'x'`). The
//! [`jsonschema`] crate exposes a structured [`ValidationErrorKind`]; we map
//! each kind back to the exact Ajv wording in [`ajv_message`] so the existing
//! command-layer formatting (and captured TS fixtures) keep matching.
//!
//! ## Compilation
//! The schemas are small, bundled (`include_str!`) draft-07 documents with no
//! external `$ref`s, so [`validate_against_schema`] compiles them per call.
//! For a one-shot CLI this is negligible (microseconds) and keeps the function
//! pure and panic-free: a (theoretically impossible) compile failure of a
//! bundled schema surfaces as a structured `internal:` error rather than an
//! `expect`/panic.

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::{JsonType, Validator};
use serde_json::Value;

/// A single Ajv-style validation error rendered as `instance_path` (a JSON
/// pointer, empty for the document root) plus an Ajv-compatible `message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub instance_path: String,
    pub message: String,
}

impl SchemaError {
    /// Construct a root-level `internal:` error (used only for the
    /// theoretically-impossible bundled-schema compile failure).
    fn internal(message: String) -> Vec<Self> {
        vec![SchemaError {
            instance_path: String::new(),
            message,
        }]
    }
}

/// Validate `data` against the JSON Schema in `schema_src`, returning the full
/// ordered list of Ajv-style errors (mirroring Ajv's `allErrors: true`).
///
/// `schema_src` is a trusted, bundled draft-07 document. A parse/compile
/// failure is an internal invariant violation rather than user input, so it is
/// surfaced as a single root-level `internal:` [`SchemaError`] instead of
/// panicking — keeping the crate clippy-clean (no `expect`/`unwrap`) and never
/// silently passing invalid data.
pub fn validate_against_schema(data: &Value, schema_src: &str) -> Result<(), Vec<SchemaError>> {
    let schema: Value = serde_json::from_str(schema_src)
        .map_err(|e| SchemaError::internal(format!("internal: invalid bundled schema JSON: {e}")))?;
    let validator = jsonschema::draft7::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|e| {
            SchemaError::internal(format!("internal: bundled schema failed to compile: {e}"))
        })?;
    let errors = collect_errors(&schema, &validator, data);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate `data` against an already-compiled [`Validator`], mapping every
/// [`jsonschema`] error into an Ajv-style [`SchemaError`].
///
/// Two parity adjustments are applied so the output matches the TypeScript
/// Ajv reference byte-for-byte:
///
///  1. **Original `pattern` text.** The crate reports the *compiled* regex in
///     [`ValidationErrorKind::Pattern`] (e.g. `\d` is normalised to `[0-9]`).
///     Ajv echoes the *schema's* literal pattern, so we recover it from the
///     parsed `schema` via the error's `schema_path()` (the keyword location).
///  2. **Ajv error ordering.** Ajv's `allErrors` surfaces shallower instance
///     locations before deeper ones (root-level `required`/
///     `additionalProperties` before per-property errors). A *stable* sort by
///     instance-path depth reproduces that while preserving the engine's
///     declaration order within a depth.
fn collect_errors(schema: &Value, validator: &Validator, data: &Value) -> Vec<SchemaError> {
    let mut errors: Vec<SchemaError> = validator
        .iter_errors(data)
        .map(|err| {
            let message = match err.kind() {
                ValidationErrorKind::Pattern { pattern } => {
                    let literal = original_pattern(schema, err.schema_path().as_str())
                        .unwrap_or_else(|| pattern.clone());
                    format!("must match pattern \"{literal}\"")
                }
                other => ajv_message(other),
            };
            SchemaError {
                instance_path: err.instance_path().to_string(),
                message,
            }
        })
        .collect();
    errors.sort_by_key(|e| e.instance_path.matches('/').count());
    errors
}

/// Recover the literal `pattern` string from the parsed `schema` by walking the
/// keyword location reported in the error's `schema_path` (a JSON pointer such
/// as `/properties/version/pattern`). Returns `None` if the location cannot be
/// resolved to a string (e.g. behind a `$ref`), in which case the caller falls
/// back to the engine's compiled pattern.
fn original_pattern(schema: &Value, schema_path: &str) -> Option<String> {
    let mut node = schema;
    for raw in schema_path.split('/').filter(|s| !s.is_empty()) {
        // Unescape JSON-pointer tokens (`~1` → `/`, `~0` → `~`).
        let token = raw.replace("~1", "/").replace("~0", "~");
        node = match node {
            Value::Object(map) => map.get(&token)?,
            Value::Array(arr) => arr.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    node.as_str().map(str::to_string)
}

/// Join schema errors into a single string using `separator`, rendering each
/// as `"<instance_path>: <message>"` — the shape the TS commands build before
/// surfacing schema failures.
pub fn join_errors(errors: &[SchemaError], separator: &str) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.instance_path, e.message))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Map a [`ValidationErrorKind`] to the exact Ajv default message text.
///
/// Only the keywords reachable from fspec's bundled schemas need precise
/// parity; the remaining arms reproduce Ajv's standard wording so future
/// schema changes keep producing sensible, parity-shaped messages.
fn ajv_message(kind: &ValidationErrorKind) -> String {
    match kind {
        ValidationErrorKind::AdditionalItems { limit } => {
            format!("must NOT have more than {limit} items")
        }
        ValidationErrorKind::AdditionalProperties { .. } => {
            "must NOT have additional properties".to_string()
        }
        ValidationErrorKind::AnyOf { .. } => "must match a schema in anyOf".to_string(),
        ValidationErrorKind::Constant { expected_value } => {
            if expected_value.is_null() {
                "must be null".to_string()
            } else {
                "must be equal to constant".to_string()
            }
        }
        ValidationErrorKind::Contains => "must contain at least 1 valid item(s)".to_string(),
        ValidationErrorKind::Enum { .. } => {
            "must be equal to one of the allowed values".to_string()
        }
        ValidationErrorKind::ExclusiveMaximum { limit } => format!("must be < {limit}"),
        ValidationErrorKind::ExclusiveMinimum { limit } => format!("must be > {limit}"),
        ValidationErrorKind::Format { format } => format!("must match format \"{format}\""),
        ValidationErrorKind::MaxItems { limit } => {
            format!("must NOT have more than {limit} items")
        }
        ValidationErrorKind::Maximum { limit } => format!("must be <= {limit}"),
        ValidationErrorKind::MaxLength { limit } => {
            format!("must NOT have more than {limit} characters")
        }
        ValidationErrorKind::MaxProperties { limit } => {
            format!("must NOT have more than {limit} properties")
        }
        ValidationErrorKind::MinItems { limit } => {
            format!("must NOT have fewer than {limit} items")
        }
        ValidationErrorKind::Minimum { limit } => format!("must be >= {limit}"),
        ValidationErrorKind::MinLength { limit } => {
            format!("must NOT have fewer than {limit} characters")
        }
        ValidationErrorKind::MinProperties { limit } => {
            format!("must NOT have fewer than {limit} properties")
        }
        ValidationErrorKind::MultipleOf { multiple_of } => {
            format!("must be multiple of {multiple_of}")
        }
        ValidationErrorKind::Not { .. } => "must NOT be valid".to_string(),
        ValidationErrorKind::OneOfMultipleValid { .. }
        | ValidationErrorKind::OneOfNotValid { .. } => {
            "must match exactly one schema in oneOf".to_string()
        }
        ValidationErrorKind::Pattern { pattern } => format!("must match pattern \"{pattern}\""),
        ValidationErrorKind::Required { property } => {
            let name = property.as_str().unwrap_or_default();
            format!("must have required property '{name}'")
        }
        ValidationErrorKind::Type { kind } => format!("must be {}", type_kind_name(kind)),
        ValidationErrorKind::UniqueItems => "must NOT have duplicate items".to_string(),
        // Keywords not present in fspec's schemas — render the failing keyword
        // generically so nothing is silently swallowed.
        other => format!("must satisfy keyword '{}'", other.keyword()),
    }
}

/// Render the Ajv `type` noun for a [`TypeKind`] (e.g. `object`, `string`,
/// `null`, or `string,number` for the multi-type form).
fn type_kind_name(kind: &TypeKind) -> String {
    match kind {
        TypeKind::Single(t) => t.as_str().to_string(),
        TypeKind::Multiple(set) => set
            .into_iter()
            .map(JsonType::as_str)
            .collect::<Vec<_>>()
            .join(","),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = r#"{
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "items"],
        "properties": {
            "name": { "type": "string", "pattern": "^[a-z]+$" },
            "items": { "type": "array", "minItems": 1 },
            "url": { "type": "string", "format": "uri" }
        }
    }"#;

    #[test]
    fn valid_instance_yields_no_errors() {
        assert!(validate_against_schema(&json!({ "name": "ok", "items": [1] }), SCHEMA).is_ok());
    }

    #[test]
    fn required_property_uses_ajv_wording() {
        let errs = validate_against_schema(&json!({ "items": [1] }), SCHEMA).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message == "must have required property 'name'"));
    }

    #[test]
    fn min_items_uses_ajv_wording() {
        let errs = validate_against_schema(&json!({ "name": "ok", "items": [] }), SCHEMA).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.instance_path == "/items"
                && e.message == "must NOT have fewer than 1 items"));
    }

    #[test]
    fn pattern_uses_real_regex_matching() {
        let errs =
            validate_against_schema(&json!({ "name": "BAD", "items": [1] }), SCHEMA).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.instance_path == "/name"
                && e.message == "must match pattern \"^[a-z]+$\""));
    }

    #[test]
    fn additional_properties_uses_ajv_wording() {
        let errs = validate_against_schema(&json!({ "name": "ok", "items": [1], "junk": 1 }), SCHEMA)
            .unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.message == "must NOT have additional properties"));
    }

    #[test]
    fn format_uri_is_actively_validated() {
        let errs =
            validate_against_schema(&json!({ "name": "ok", "items": [1], "url": "not a uri" }), SCHEMA)
                .unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.instance_path == "/url" && e.message == "must match format \"uri\""));
    }

    #[test]
    fn pattern_message_echoes_schema_literal_not_compiled_regex() {
        // The jsonschema engine normalises `\d` to `[0-9]` in its own error;
        // Ajv echoes the schema's literal pattern, so we recover the original.
        const DIGIT_SCHEMA: &str = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": { "version": { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+$" } }
        }"#;
        let errs =
            validate_against_schema(&json!({ "version": "2.0" }), DIGIT_SCHEMA).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].instance_path, "/version");
        assert_eq!(
            errs[0].message, "must match pattern \"^\\d+\\.\\d+\\.\\d+$\"",
            "expected the schema's literal \\d pattern, got {errs:?}"
        );
    }

    #[test]
    fn errors_are_ordered_shallowest_instance_path_first() {
        // Ajv surfaces root-level keyword errors (required, additionalProperties)
        // before descending into per-property errors. A stable depth sort
        // reproduces that ordering.
        const NESTED: &str = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "additionalProperties": false,
            "required": ["a"],
            "properties": {
                "b": { "type": "string", "pattern": "^x$" }
            }
        }"#;
        // Missing required `a` (root), bad pattern at `/b` (depth 1), and an
        // extra `junk` property (root additionalProperties).
        let errs =
            validate_against_schema(&json!({ "b": "nope", "junk": 1 }), NESTED).unwrap_err();
        let depths: Vec<usize> = errs.iter().map(|e| e.instance_path.matches('/').count()).collect();
        assert!(
            depths.windows(2).all(|w| w[0] <= w[1]),
            "errors must be non-decreasing in instance-path depth: {errs:?}"
        );
    }
}
