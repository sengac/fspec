//! Native draft-07-subset JSON-Schema validator for `tags.json`.
//!
//! Ports the subset of Ajv behaviour exercised by
//! `src/validators/json-schema.ts` (`validateTagsJson`) that the TS
//! `generate-tags-md` command runs BEFORE writing `TAGS.md`.
//!
//! The TS `generateTagsMdCommand` validates `spec/tags.json` against the
//! bundled `tags.schema.json` (Ajv, `allErrors: true`) and returns
//! `{ success: false, error: "tags.json has validation errors: ..." }`
//! WITHOUT writing the file when validation fails.
//!
//! This module reproduces that gate. The bundled schema is embedded via
//! `include_str!` so the validator needs no filesystem access. The schema
//! only uses these keywords: `type`, `required`, `properties`, `items`,
//! `pattern` (`^@[a-z0-9-]+$` and `^\d+%$`), `format` (`date-time`, `uri`),
//! and `minimum`. The error rendering mirrors Ajv's `instancePath: message`
//! form; the standalone command surfaces the joined list verbatim behind the
//! `tags.json has validation errors: ` prefix.

use serde_json::Value;

/// Embedded copy of `src/schemas/tags.schema.json`.
const TAGS_SCHEMA_SRC: &str = include_str!("tags.schema.json");

/// A single Ajv-style validation error rendered as `instancePath: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsSchemaError {
    pub instance_path: String,
    pub message: String,
}

/// Validate `data` against the bundled `tags.schema.json`.
///
/// Returns `Ok(())` when valid; otherwise the full ordered list of Ajv-style
/// errors (mirroring `allErrors: true`).
pub fn validate_tags(data: &Value) -> Result<(), Vec<TagsSchemaError>> {
    let schema: Value = serde_json::from_str(TAGS_SCHEMA_SRC).unwrap_or(Value::Null);
    let root = schema.clone();
    let mut errors = Vec::new();
    validate_node(data, &schema, &root, "", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Join schema errors into the TS-equivalent string used by
/// `generate-tags-md`: each error as `instancePath: message`, joined by
/// `", "`.
pub fn format_tags_errors(errors: &[TagsSchemaError]) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.instance_path, e.message))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve a local `#/...` `$ref` against the schema root.
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
    errors: &mut Vec<TagsSchemaError>,
) {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if let Some(target) = resolve_ref(reference, root) {
            validate_node(data, target, root, path, errors);
        }
        return;
    }

    // `type` — checked first. On mismatch Ajv reports and does not descend.
    if let Some(type_spec) = schema.get("type") {
        if !type_matches(data, type_spec) {
            errors.push(TagsSchemaError {
                instance_path: path.to_string(),
                message: format!("must be {}", type_name(type_spec)),
            });
            return;
        }
    }

    // String constraints.
    if let Some(s) = data.as_str() {
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            if !regex_lite_match(pattern, s) {
                errors.push(TagsSchemaError {
                    instance_path: path.to_string(),
                    message: format!("must match pattern \"{pattern}\""),
                });
            }
        }
        if let Some(fmt) = schema.get("format").and_then(Value::as_str) {
            if fmt == "date-time" && !is_date_time(s) {
                errors.push(TagsSchemaError {
                    instance_path: path.to_string(),
                    message: "must match format \"date-time\"".to_string(),
                });
            }
            if fmt == "uri" && !is_uri(s) {
                errors.push(TagsSchemaError {
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
                errors.push(TagsSchemaError {
                    instance_path: path.to_string(),
                    message: format!("must be >= {min}"),
                });
            }
        }
    }

    // Object constraints: required, then properties.
    if let Some(obj) = data.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for req in required {
                if let Some(key) = req.as_str() {
                    if !obj.contains_key(key) {
                        errors.push(TagsSchemaError {
                            instance_path: path.to_string(),
                            message: format!("must have required property '{key}'"),
                        });
                    }
                }
            }
        }

        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (key, subschema) in props {
                if let Some(child) = obj.get(key) {
                    let child_path = format!("{path}/{key}");
                    validate_node(child, subschema, root, &child_path, errors);
                }
            }
        }
    }

    // Array constraints: per-item.
    if let Some(arr) = data.as_array() {
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{path}/{i}");
                validate_node(item, items_schema, root, &item_path, errors);
            }
        }
    }
}

/// JSON-Schema `type` match (single-string form).
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

/// Ajv `type` error noun.
fn type_name(type_spec: &Value) -> String {
    type_spec.as_str().unwrap_or("value").to_string()
}

/// Minimal matcher for the only two `pattern`s in tags.schema.json:
/// `^@[a-z0-9-]+$` (tag name) and `^\d+%$` (percentage string).
fn regex_lite_match(pattern: &str, s: &str) -> bool {
    if pattern == "^@[a-z0-9-]+$" {
        let rest = match s.strip_prefix('@') {
            Some(r) => r,
            None => return false,
        };
        return !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    }
    if pattern == r"^\d+%$" {
        let rest = match s.strip_suffix('%') {
            Some(r) => r,
            None => return false,
        };
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    }
    // Unknown pattern: be permissive.
    true
}

/// Loose ISO-8601 date-time check (Ajv `date-time` format).
fn is_date_time(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

/// Minimal URI check (Ajv `uri` format): require a valid scheme followed by `:`.
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

    fn valid_tags() -> Value {
        json!({
            "categories": [
                { "name": "Phase Tags", "description": "d", "required": true,
                  "tags": [ { "name": "@critical", "description": "c" } ] }
            ],
            "combinationExamples": [
                { "title": "E", "tags": "@cli", "interpretation": ["CLI"] }
            ],
            "usageGuidelines": {
                "requiredCombinations": { "title": "R", "requirements": ["x"], "minimumExample": "@p" },
                "recommendedCombinations": { "title": "R", "includes": ["x"], "recommendedExample": "@p" },
                "orderingConvention": { "title": "O", "order": ["phase"], "example": "@p" }
            },
            "addingNewTags": {
                "process": [ { "step": "1", "description": "d" } ],
                "namingConventions": ["lower"],
                "antiPatterns": { "dont": [ { "description": "d" } ], "do": [ { "description": "d" } ] }
            },
            "queries": { "title": "Q", "examples": [ { "description": "d", "command": "c" } ] },
            "statistics": {
                "lastUpdated": "2025-01-15T10:30:00Z",
                "phaseStats": [ { "phase": "P", "total": 5, "complete": 5, "inProgress": 0, "planned": 0 } ],
                "componentStats": [ { "component": "@cli", "count": 1, "percentage": "100%" } ],
                "featureGroupStats": [ { "featureGroup": "@v", "count": 1, "percentage": "50%" } ],
                "updateCommand": "fspec tag-stats"
            },
            "validation": {
                "rules": [ { "rule": "R", "description": "d" } ],
                "commands": [ { "description": "d", "command": "c" } ]
            },
            "references": [ { "title": "T", "url": "https://example.com" } ]
        })
    }

    #[test]
    fn accepts_valid_tags() {
        assert!(validate_tags(&valid_tags()).is_ok());
    }

    #[test]
    fn rejects_missing_required_top_level() {
        let errs = validate_tags(&json!({ "categories": [] })).unwrap_err();
        let rendered = format_tags_errors(&errs);
        assert!(
            rendered.contains("must have required property 'combinationExamples'"),
            "got: {rendered}"
        );
    }

    #[test]
    fn rejects_bad_tag_name_pattern() {
        let mut t = valid_tags();
        t["categories"][0]["tags"][0]["name"] = json!("WIP");
        let errs = validate_tags(&t).unwrap_err();
        let rendered = format_tags_errors(&errs);
        assert!(
            rendered.contains("/categories/0/tags/0/name: must match pattern \"^@[a-z0-9-]+$\""),
            "got: {rendered}"
        );
    }

    #[test]
    fn rejects_bad_percentage_pattern() {
        let mut t = valid_tags();
        t["statistics"]["componentStats"][0]["percentage"] = json!("100");
        let errs = validate_tags(&t).unwrap_err();
        let rendered = format_tags_errors(&errs);
        assert!(rendered.contains("must match pattern"), "got: {rendered}");
    }
}
