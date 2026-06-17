//! `generate-tags-md` — Rust port of `src/commands/generate-tags-md.ts`
//! (RPC-236).
//!
//! Reads `spec/tags.json`, renders `TAGS.md` via
//! [`crate::generators::generate_tags_md`], and writes it to `spec/TAGS.md`
//! (or a custom `--output` path). The written bytes are the raw markdown
//! string, which carries a single trailing newline (the TS generator pushes a
//! trailing empty section before `join('\n')`) — matching the TS
//! `writeFile(tagsMdPath, markdown, 'utf-8')` call exactly.
//!
//! ## Framing A divergence from TypeScript
//!
//! * **JSON schema validation**: the TS command calls `validateTagsJson`
//!   (Ajv) and aborts with a `tags.json has validation errors: ...` message
//!   on failure. This port reproduces that gate with a native draft-07-subset
//!   validator ([`crate::generators::validate_tags`]) bundling the same
//!   `tags.schema.json`; errors render in Ajv's `${instancePath}: ${message}`
//!   form. Valid tag registries produce byte-identical output.
//!
//! ## Result envelope
//! Success returns `{ success: true, message }` JSON via `Ok(json)`. The
//! missing-tags-json and schema-validation paths return
//! [`FspecCoreError::InvalidArgs`] whose rendered `reason` matches the TS
//! `result.error` text — the dispatcher derives `success=false` and the CLI
//! bridge prints `Error: <reason>` to stderr.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::generators::{generate_tags_md as render_tags_md, validate_tags};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GenerateTagsMdArgs {
    /// Custom output path (default: `spec/TAGS.md`). When relative it is
    /// joined to the project root, mirroring TS `join(cwd, outputPath)`.
    output: Option<String>,
}

/// Dispatcher / CLI-bridge entry point. Returns `{ success, message }` JSON on
/// success, or a [`FspecCoreError`] whose rendered message matches the TS
/// `result.error` text on the tags-missing / schema-invalid paths.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateTagsMdArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "generate-tags-md",
            reason: format!("failed to parse args: {e}"),
        })?;

    let message = generate(project_root, args.output.as_deref())?;

    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "generate-tags-md",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Core generation routine. On success returns the human-readable message
/// `Generated <outputRelative> from spec/tags.json`. The `output_path` is the
/// optional `--output` value (relative paths are resolved against
/// `project_root`).
fn generate(project_root: &Path, output_path: Option<&str>) -> Result<String, FspecCoreError> {
    let tags_json_path = project_root.join("spec").join("tags.json");
    let tags_md_path = match output_path {
        Some(p) => project_root.join(p),
        None => project_root.join("spec").join("TAGS.md"),
    };

    // Check if tags.json exists (TS `existsSync`).
    if !tags_json_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "generate-tags-md",
            reason: "tags.json not found: spec/tags.json".to_string(),
        });
    }

    // Read + parse tags.json.
    let content =
        std::fs::read_to_string(&tags_json_path).map_err(|source| FspecCoreError::Io {
            command: "generate-tags-md",
            source,
        })?;
    let tags: Value = serde_json::from_str(&content).map_err(|e| FspecCoreError::ParseJson {
        file: "tags.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&content, &e),
    })?;

    // Validate tags.json against the bundled tags schema BEFORE rendering —
    // mirroring the TS `validateTagsJson` (Ajv) gate. On failure the TS
    // command returns `{ success: false }` and writes nothing.
    //
    // The TS message is `tags.json has validation errors: ${errors.join(', ')}`
    // where `errors` is `ErrorObject[]`. JS `Array.join` coerces each plain
    // object to the string `"[object Object]"`, so the rendered message is
    // simply N copies of `[object Object]` (N = error count) joined by `, ` —
    // byte-for-byte. We reproduce that exactly rather than emitting our own
    // Ajv-style `instancePath: message` text.
    if let Err(schema_errors) = validate_tags(&tags) {
        let joined = vec!["[object Object]"; schema_errors.len()].join(", ");
        return Err(FspecCoreError::InvalidArgs {
            command: "generate-tags-md",
            reason: format!("tags.json has validation errors: {joined}"),
        });
    }

    // Render markdown.
    let markdown = render_tags_md(&tags);

    // Ensure output directory exists, then write TAGS.md verbatim (the
    // rendered markdown carries a single trailing newline — matches TS
    // `writeFile(..., markdown, 'utf-8')`).
    if let Some(parent) = tags_md_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "generate-tags-md",
            source,
        })?;
    }
    std::fs::write(&tags_md_path, markdown.as_bytes()).map_err(|source| FspecCoreError::Io {
        command: "generate-tags-md",
        source,
    })?;

    let output_relative = output_path.unwrap_or("spec/TAGS.md");
    Ok(format!("Generated {output_relative} from spec/tags.json"))
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

    fn write_tags(root: &Path, value: &Value) {
        let spec = root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(
            spec.join("tags.json"),
            serde_json::to_string_pretty(value).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn generate_writes_tags_md_byte_exact() {
        let tmp = tempfile::tempdir().unwrap();
        write_tags(tmp.path(), &valid_tags());
        let message = generate(tmp.path(), None).unwrap();
        let written =
            std::fs::read_to_string(tmp.path().join("spec").join("TAGS.md")).unwrap();
        assert_eq!(written, render_tags_md(&valid_tags()));
        assert_eq!(message, "Generated spec/TAGS.md from spec/tags.json");
    }

    #[test]
    fn generate_honours_custom_output_path() {
        let tmp = tempfile::tempdir().unwrap();
        write_tags(tmp.path(), &valid_tags());
        let message = generate(tmp.path(), Some("docs/TAGS.md")).unwrap();
        assert!(tmp.path().join("docs").join("TAGS.md").exists());
        assert_eq!(message, "Generated docs/TAGS.md from spec/tags.json");
    }

    #[test]
    fn generate_fails_when_tags_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let err = generate(tmp.path(), None).unwrap_err();
        assert!(err
            .to_string()
            .contains("tags.json not found: spec/tags.json"));
        assert!(!tmp.path().join("spec").join("TAGS.md").exists());
    }

    #[test]
    fn generate_fails_when_schema_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        write_tags(tmp.path(), &json!({ "categories": [] }));
        let err = generate(tmp.path(), None).unwrap_err();
        assert!(err.to_string().contains("tags.json has validation errors:"));
        assert!(!tmp.path().join("spec").join("TAGS.md").exists());
    }
}
