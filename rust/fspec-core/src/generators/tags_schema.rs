//! Tags schema gate — thin wrapper over the shared, spec-compliant JSON
//! Schema engine ([`crate::validators::json_schema`]).
//!
//! Ports the Ajv-backed `validateTagsJson` from
//! `src/validators/json-schema.ts` that the TS `generate-tags-md` command runs
//! before writing `TAGS.md`. On failure the command returns
//! `{ success: false, error: "tags.json has validation errors: ..." }` WITHOUT
//! writing the file.
//!
//! The bundled schema is embedded via `include_str!` and compiled exactly once
//! (cached in a [`OnceLock`]). Real regex `pattern`s (`^@[a-z0-9-]+$`,
//! `^\d+%$`), `format` (`date-time`, `uri`) and `minimum` are validated by the
//! [`jsonschema`] crate.

use serde_json::Value;

use crate::validators::json_schema::validate_against_schema;
pub use crate::validators::json_schema::{join_errors, SchemaError as TagsSchemaError};

/// Embedded copy of `src/schemas/tags.schema.json`.
const TAGS_SCHEMA_SRC: &str = include_str!("tags.schema.json");

/// Validate `data` against the bundled `tags.schema.json`.
///
/// Returns `Ok(())` when valid; otherwise the full ordered list of Ajv-style
/// errors (mirroring `allErrors: true`).
pub fn validate_tags(data: &Value) -> Result<(), Vec<TagsSchemaError>> {
    validate_against_schema(data, TAGS_SCHEMA_SRC)
}

/// Join schema errors into the TS-equivalent string used by `generate-tags-md`:
/// each error as `instancePath: message`, joined by `", "`.
pub fn format_tags_errors(errors: &[TagsSchemaError]) -> String {
    join_errors(errors, ", ")
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
