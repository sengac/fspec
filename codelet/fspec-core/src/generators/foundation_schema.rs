//! Native draft-07-subset JSON-Schema validator for `generic-foundation`.
//!
//! Ports the subset of Ajv behaviour exercised by
//! `src/validators/json-schema.ts` (`validateFoundationJson`) that the TS
//! `generate-foundation-md` command runs BEFORE writing `FOUNDATION.md`.
//!
//! The six Event-Storm foundation-mutation commands call
//! `generateFoundationMdCommand({ cwd })` as a best-effort auto-regenerate
//! step; that helper validates `foundation.json` against
//! `generic-foundation.schema.json` (Ajv, `allErrors: true`) and returns
//! `{ success: false }` WITHOUT writing the file when validation fails. The
//! callers ignore the result, so the only observable effect is that
//! `FOUNDATION.md` is NOT (re)written for a schema-invalid foundation.
//!
//! This module reproduces that gate. The bundled schema is embedded via
//! `include_str!` so the validator needs no filesystem access. Error strings
//! and their ordering match Ajv's `${instancePath}: ${message}` rendering so
//! the standalone `generate-foundation-md` command can surface them verbatim.

use serde_json::Value;

/// Embedded copy of `src/schemas/generic-foundation.schema.json`.
const FOUNDATION_SCHEMA_SRC: &str = include_str!("generic-foundation.schema.json");

/// A single Ajv-style validation error rendered as `instancePath: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub instance_path: String,
    pub message: String,
}

/// Validate `data` against the bundled generic-foundation schema.
///
/// Returns `Ok(())` when valid; otherwise the full ordered list of Ajv-style
/// errors (mirroring `allErrors: true`).
pub fn validate_foundation(data: &Value) -> Result<(), Vec<SchemaError>> {
    let schema: Value = serde_json::from_str(FOUNDATION_SCHEMA_SRC)
        .unwrap_or(Value::Null);
    let root = schema.clone();
    let mut errors = Vec::new();
    validate_node(data, &schema, &root, "", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Join schema errors into the TS `formatValidationErrors`-equivalent string
/// used by `generate-foundation-md`: each error as `instancePath: message`
/// (empty path stays empty), joined by `"; "`.
pub fn format_errors(errors: &[SchemaError]) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.instance_path, e.message))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Resolve a local `#/definitions/...` `$ref` against the schema root.
fn resolve_ref<'a>(reference: &str, root: &'a Value) -> Option<&'a Value> {
    let path = reference.strip_prefix("#/")?;
    let mut current = root;
    for seg in path.split('/') {
        current = current.get(seg)?;
    }
    Some(current)
}

/// Core recursive validator. Appends Ajv-style errors to `errors`.
fn validate_node(
    data: &Value,
    schema: &Value,
    root: &Value,
    path: &str,
    errors: &mut Vec<SchemaError>,
) {
    // $ref indirection first (Ajv inlines the referenced schema).
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if let Some(target) = resolve_ref(reference, root) {
            validate_node(data, target, root, path, errors);
        }
        return;
    }

    // allOf — every subschema must pass; errors accumulate in order.
    if let Some(subs) = schema.get("allOf").and_then(Value::as_array) {
        for sub in subs {
            validate_node(data, sub, root, path, errors);
        }
    }

    // oneOf — Ajv emits each branch's errors (in order) plus a final
    // "must match exactly one schema in oneOf" when not exactly one passes.
    // When exactly one branch matches, Ajv discards all branch errors.
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let mut pass_count = 0;
        let mut collected: Vec<SchemaError> = Vec::new();
        for branch in branches {
            let mut branch_errors = Vec::new();
            validate_node(data, branch, root, path, &mut branch_errors);
            if branch_errors.is_empty() {
                pass_count += 1;
            } else {
                collected.append(&mut branch_errors);
            }
        }
        if pass_count != 1 {
            errors.append(&mut collected);
            errors.push(SchemaError {
                instance_path: path.to_string(),
                message: "must match exactly one schema in oneOf".to_string(),
            });
        }
        return;
    }

    validate_keywords(data, schema, root, path, errors);
}

/// Validate the leaf keywords of a (non-`$ref`, non-combinator) schema node
/// against `data`, in Ajv's evaluation order.
fn validate_keywords(
    data: &Value,
    schema: &Value,
    root: &Value,
    path: &str,
    errors: &mut Vec<SchemaError>,
) {
    // `type` — checked first. On mismatch Ajv reports and does not descend
    // into property/items keywords for this node.
    if let Some(type_spec) = schema.get("type") {
        if !type_matches(data, type_spec) {
            errors.push(SchemaError {
                instance_path: path.to_string(),
                message: format!("must be {}", type_name(type_spec)),
            });
            return;
        }
    }

    // `const`
    if let Some(expected) = schema.get("const") {
        if data != expected {
            // Ajv special-cases a null const as "must be null".
            let msg = if expected.is_null() {
                "must be null".to_string()
            } else {
                "must be equal to constant".to_string()
            };
            errors.push(SchemaError {
                instance_path: path.to_string(),
                message: msg,
            });
        }
    }

    // `enum`
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.iter().any(|v| v == data) {
            errors.push(SchemaError {
                instance_path: path.to_string(),
                message: "must be equal to one of the allowed values".to_string(),
            });
        }
    }

    // String constraints.
    if let Some(s) = data.as_str() {
        if let Some(min) = schema.get("minLength").and_then(Value::as_u64) {
            if (s.chars().count() as u64) < min {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: format!("must NOT have fewer than {min} characters"),
                });
            }
        }
        if let Some(max) = schema.get("maxLength").and_then(Value::as_u64) {
            if (s.chars().count() as u64) > max {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: format!("must NOT have more than {max} characters"),
                });
            }
        }
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            if !regex_lite_match(pattern, s) {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: format!("must match pattern \"{pattern}\""),
                });
            }
        }
        if let Some(fmt) = schema.get("format").and_then(Value::as_str) {
            if fmt == "date-time" && !is_date_time(s) {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: "must match format \"date-time\"".to_string(),
                });
            }
            if fmt == "uri" && !is_uri(s) {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: "must match format \"uri\"".to_string(),
                });
            }
        }
    }

    // Numeric constraints.
    if let Some(n) = data.as_i64() {
        if let Some(min) = schema.get("minimum").and_then(Value::as_i64) {
            if n < min {
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: format!("must be >= {min}"),
                });
            }
        }
    }

    // Object constraints: required, additionalProperties, then properties.
    if let Some(obj) = data.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for req in required {
                if let Some(key) = req.as_str() {
                    if !obj.contains_key(key) {
                        errors.push(SchemaError {
                            instance_path: path.to_string(),
                            message: format!("must have required property '{key}'"),
                        });
                    }
                }
            }
        }

        let props = schema.get("properties").and_then(Value::as_object);
        let additional = schema.get("additionalProperties");
        let allows_additional = match additional {
            Some(Value::Bool(false)) => false,
            None => true,
            _ => true,
        };
        if !allows_additional {
            if let Some(props) = props {
                if obj.keys().any(|k| !props.contains_key(k)) {
                    errors.push(SchemaError {
                        instance_path: path.to_string(),
                        message: "must NOT have additional properties".to_string(),
                    });
                }
            }
        }

        if let Some(props) = props {
            for (key, subschema) in props {
                if let Some(child) = obj.get(key) {
                    let child_path = format!("{path}/{key}");
                    validate_node(child, subschema, root, &child_path, errors);
                }
            }
        }
    }

    // Array constraints: minItems then per-item.
    if let Some(arr) = data.as_array() {
        if let Some(min) = schema.get("minItems").and_then(Value::as_u64) {
            if (arr.len() as u64) < min {
                // Ajv's `minItems` message always uses the plural "items",
                // even when the limit is 1 (e.g. "must NOT have fewer than 1
                // items").
                errors.push(SchemaError {
                    instance_path: path.to_string(),
                    message: format!("must NOT have fewer than {min} items"),
                });
            }
        }
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{path}/{i}");
                validate_node(item, items_schema, root, &item_path, errors);
            }
        }
    }
}

/// JSON-Schema `type` match (handles the single-string form used here).
fn type_matches(data: &Value, type_spec: &Value) -> bool {
    match type_spec.as_str() {
        Some("object") => data.is_object(),
        Some("array") => data.is_array(),
        Some("string") => data.is_string(),
        Some("integer") => data.is_i64() || data.is_u64(),
        Some("number") => data.is_number(),
        Some("boolean") => data.is_boolean(),
        Some("null") => data.is_null(),
        _ => true,
    }
}

/// Ajv `type` error noun (e.g. "object", "string", "null").
fn type_name(type_spec: &Value) -> String {
    type_spec.as_str().unwrap_or("value").to_string()
}

/// Minimal matcher for the only `pattern` in the schema:
/// `^\d+\.\d+\.\d+$` (semver-like version). Returns true if `s` is three
/// dot-separated runs of ASCII digits.
fn regex_lite_match(pattern: &str, s: &str) -> bool {
    if pattern == r"^\d+\.\d+\.\d+$" {
        let parts: Vec<&str> = s.split('.').collect();
        return parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    }
    if pattern == r"\.foundation\.json$" {
        return s.ends_with(".foundation.json");
    }
    // Unknown pattern: be permissive (no schema uses others in practice).
    true
}

/// Loose ISO-8601 date-time check matching Ajv's `full` date-time format
/// closely enough for the createdAt timestamps fspec emits.
fn is_date_time(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

/// Minimal URI check (Ajv `uri` format): require a scheme followed by `:`.
fn is_uri(s: &str) -> bool {
    if let Some(colon) = s.find(':') {
        let scheme = &s[..colon];
        !scheme.is_empty()
            && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    } else {
        false
    }
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
        assert_eq!(
            format_errors(&errs),
            "/solutionSpace/capabilities: must NOT have fewer than 1 items"
        );
    }

    #[test]
    fn rejects_bad_version_pattern() {
        let mut f = valid_foundation();
        f["version"] = json!("2.0");
        let errs = validate_foundation(&f).unwrap_err();
        assert_eq!(
            format_errors(&errs),
            "/version: must match pattern \"^\\d+\\.\\d+\\.\\d+$\""
        );
    }

    #[test]
    fn rejects_missing_required_top_level() {
        let mut f = valid_foundation();
        f.as_object_mut().unwrap().remove("solutionSpace");
        let errs = validate_foundation(&f).unwrap_err();
        assert_eq!(
            format_errors(&errs),
            ": must have required property 'solutionSpace'"
        );
    }

    #[test]
    fn rejects_additional_properties_at_root() {
        let mut f = valid_foundation();
        f["junk"] = json!(1);
        let errs = validate_foundation(&f).unwrap_err();
        assert_eq!(format_errors(&errs), ": must NOT have additional properties");
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
        assert!(rendered.contains("/eventStorm/items/0: must match exactly one schema in oneOf"));
        assert!(rendered.contains("/eventStorm/items/0/color: must be null"));
    }
}
