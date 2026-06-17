//! `generate-foundation-md` — Rust port of
//! `src/commands/generate-foundation-md.ts` (RPC-233).
//!
//! Reads `spec/foundation.json`, renders `FOUNDATION.md` via
//! [`crate::generators::generate_foundation_md`], and writes it to
//! `spec/FOUNDATION.md` (or a custom `--output` path). The written bytes are
//! the raw markdown string with NO trailing newline — matching the TS
//! `writeFile(foundationMdPath, markdown, 'utf-8')` call exactly.
//!
//! ## Framing A divergences from TypeScript
//!
//! * **JSON schema validation**: the TS command calls `validateFoundationJson`
//!   (Ajv) and aborts with a `foundation.json has validation errors: ...`
//!   message on failure. This port reproduces that gate with a native
//!   draft-07-subset validator
//!   ([`crate::generators::foundation_schema::validate_foundation`]) bundling
//!   the same `generic-foundation.schema.json`; errors render in Ajv's
//!   `${instancePath}: ${message}` form. Valid foundations produce
//!   byte-identical output.
//! * **Mermaid validation**: TS uses `mermaid.parse()` + jsdom. Rust uses the
//!   lightweight pure-string pre-check documented in `add_diagram.rs`
//!   (quoted-subgraph-title + invalid-identifier detection only). The
//!   generated diagrams always pass, so valid foundations render identically.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::generators::{format_errors, generate_foundation_md, validate_foundation, validate_mermaid};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct GenerateFoundationMdArgs {
    /// Custom output path (default: `spec/FOUNDATION.md`). When relative it is
    /// joined to the project root, mirroring TS `join(cwd, outputPath)`.
    output: Option<String>,
}

/// Dispatcher / CLI-bridge entry point. Returns `{ success, message }` JSON on
/// success, or a [`FspecCoreError`] whose rendered message matches the TS
/// `result.error` text on the foundation-missing / diagram-invalid paths.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateFoundationMdArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "generate-foundation-md",
            reason: format!("failed to parse args: {e}"),
        })?;

    let message = generate(project_root, args.output.as_deref())?;

    serde_json::to_string(&json!({ "success": true, "message": message })).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "generate-foundation-md",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Core generation routine shared by [`run`] and [`regenerate`].
///
/// On success returns the human-readable message
/// `Generated <outputRelative> from spec/foundation.json`. The `output_path`
/// is the optional `--output` value (relative paths are resolved against
/// `project_root`).
fn generate(project_root: &Path, output_path: Option<&str>) -> Result<String, FspecCoreError> {
    let foundation_json_path = project_root.join("spec").join("foundation.json");
    let foundation_md_path = match output_path {
        Some(p) => project_root.join(p),
        None => project_root.join("spec").join("FOUNDATION.md"),
    };

    // Check if foundation.json exists (TS `existsSync`).
    if !foundation_json_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "generate-foundation-md",
            reason: "foundation.json not found: spec/foundation.json".to_string(),
        });
    }

    // Read + parse foundation.json. (Schema validation is intentionally
    // skipped — see module docs.)
    let content =
        std::fs::read_to_string(&foundation_json_path).map_err(|source| FspecCoreError::Io {
            command: "generate-foundation-md",
            source,
        })?;
    let foundation: Value =
        serde_json::from_str(&content).map_err(|e| FspecCoreError::ParseJson {
            file: "foundation.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&content, &e),
        })?;

    // Validate foundation.json against the bundled generic-foundation schema
    // BEFORE rendering — mirroring the TS `validateFoundationJson` (Ajv) gate
    // in `generate-foundation-md.ts`. On failure the TS command returns
    // `{ success: false }` and writes nothing. The six Event-Storm
    // auto-regenerate callers ignore that result (best-effort), so an invalid
    // foundation simply leaves FOUNDATION.md untouched. Errors render as
    // `${instancePath}: ${message}` joined by "; " to match Ajv verbatim.
    if let Err(schema_errors) = validate_foundation(&foundation) {
        return Err(FspecCoreError::InvalidArgs {
            command: "generate-foundation-md",
            reason: format!(
                "foundation.json has validation errors: {}",
                format_errors(&schema_errors)
            ),
        });
    }

    // Validate all Mermaid diagrams in architectureDiagrams (lightweight
    // pre-check). Mirrors the TS loop that collects every failure.
    if let Some(diagrams) = foundation
        .get("architectureDiagrams")
        .and_then(Value::as_array)
    {
        let mut diagram_errors: Vec<String> = Vec::new();
        for (i, diagram) in diagrams.iter().enumerate() {
            let code = diagram
                .get("mermaidCode")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Err(e) = validate_mermaid(code) {
                let title = diagram
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("undefined");
                diagram_errors.push(format!(
                    "architectureDiagrams[{i}] (title: \"{title}\"): {e}"
                ));
            }
        }
        if !diagram_errors.is_empty() {
            return Err(FspecCoreError::InvalidArgs {
                command: "generate-foundation-md",
                reason: build_diagram_error_message(&diagram_errors),
            });
        }
    }

    // Render markdown.
    let markdown =
        generate_foundation_md(&foundation).map_err(|reason| FspecCoreError::InvalidArgs {
            command: "generate-foundation-md",
            reason,
        })?;

    // Ensure output directory exists, then write FOUNDATION.md (no trailing
    // newline — matches TS `writeFile(..., markdown, 'utf-8')`).
    if let Some(parent) = foundation_md_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "generate-foundation-md",
            source,
        })?;
    }
    std::fs::write(&foundation_md_path, markdown.as_bytes()).map_err(|source| {
        FspecCoreError::Io {
            command: "generate-foundation-md",
            source,
        }
    })?;

    let output_relative = output_path.unwrap_or("spec/FOUNDATION.md");
    Ok(format!(
        "Generated {output_relative} from spec/foundation.json"
    ))
}

/// Auto-regenerate `spec/FOUNDATION.md` after a foundation-mutation command,
/// mirroring the TS `await generateFoundationMdCommand({ cwd })` call the six
/// Event-Storm foundation commands make AFTER their atomic write.
///
/// The TS callers ignore the returned result object, so any failure here is
/// swallowed — the mutation already succeeded and FOUNDATION.md is
/// best-effort documentation.
pub fn regenerate(project_root: &Path) {
    let _ = generate(project_root, None);
}

/// Build the multi-line `Diagram validation failed!` message exactly as the
/// TS command joins it with `\n`.
fn build_diagram_error_message(errors: &[String]) -> String {
    let plural = if errors.len() > 1 { "s" } else { "" };
    let mut lines = vec![
        "Diagram validation failed!".to_string(),
        String::new(),
        format!("Found {} invalid diagram{plural}:", errors.len()),
    ];
    for err in errors {
        lines.push(format!("  - {err}"));
    }
    lines.push(String::new());
    lines.push("Fix the diagram(s) in spec/foundation.json".to_string());
    lines.push("Run 'fspec generate-foundation-md' again after fixing".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn write_foundation(root: &Path, value: Value) {
        let spec = root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(
            spec.join("foundation.json"),
            serde_json::to_string(&value).unwrap(),
        )
        .unwrap();
    }

    /// Schema-valid scaffold matching `generic-foundation.schema.json`'s
    /// required top-level shape. Tests merge their own fields onto this so
    /// the schema-validation gate added to `generate()` passes — the TS
    /// `validateFoundationJson` (Ajv) check rejects bare `{project:{...}}`.
    fn valid_base() -> serde_json::Map<String, Value> {
        json!({
            "version": "2.0.0",
            "project": { "name": "Demo", "vision": "V", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
            "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
        })
        .as_object()
        .cloned()
        .unwrap()
    }

    #[test]
    fn generate_writes_foundation_md_byte_exact() {
        // @step Given spec/foundation.json exists with a valid generic foundation
        let tmp = tempfile::tempdir().unwrap();
        let foundation = Value::Object(valid_base());
        write_foundation(tmp.path(), foundation.clone());
        // @step When I dispatch generate-foundation-md with no output override
        let message = generate(tmp.path(), None).unwrap();
        // @step Then generation succeeds
        // @step And the bytes written to spec/FOUNDATION.md exactly equal the rendered markdown
        let written =
            std::fs::read_to_string(tmp.path().join("spec").join("FOUNDATION.md")).unwrap();
        assert_eq!(written, generate_foundation_md(&foundation).unwrap());
        // @step And the response message is "Generated spec/FOUNDATION.md from spec/foundation.json"
        assert_eq!(
            message,
            "Generated spec/FOUNDATION.md from spec/foundation.json"
        );
    }

    #[test]
    fn generate_honours_custom_output_path() {
        // @step Given spec/foundation.json exists with a valid generic foundation
        let tmp = tempfile::tempdir().unwrap();
        write_foundation(tmp.path(), Value::Object(valid_base()));
        // @step When I dispatch generate-foundation-md with output='docs/FOUNDATION.md'
        let message = generate(tmp.path(), Some("docs/FOUNDATION.md")).unwrap();
        // @step Then generation succeeds
        // @step And the file docs/FOUNDATION.md is written relative to the project root
        assert!(tmp.path().join("docs").join("FOUNDATION.md").exists());
        // @step And the response message is "Generated docs/FOUNDATION.md from spec/foundation.json"
        assert_eq!(
            message,
            "Generated docs/FOUNDATION.md from spec/foundation.json"
        );
    }

    #[test]
    fn generate_fails_when_foundation_missing() {
        // @step Given an empty project root directory with no spec/foundation.json
        let tmp = tempfile::tempdir().unwrap();
        // @step When I dispatch generate-foundation-md with no output override
        let err = generate(tmp.path(), None).unwrap_err();
        // @step Then generation fails
        // @step And the error message contains the substring 'foundation.json not found: spec/foundation.json'
        assert!(err
            .to_string()
            .contains("foundation.json not found: spec/foundation.json"));
        // @step And the file spec/FOUNDATION.md is not created
        assert!(!tmp.path().join("spec").join("FOUNDATION.md").exists());
    }

    #[test]
    fn regenerate_is_best_effort_when_foundation_missing() {
        // @step Given a project root with no spec/foundation.json
        let tmp = tempfile::tempdir().unwrap();
        // @step When crate::commands::generate_foundation_md::regenerate is invoked
        regenerate(tmp.path());
        // @step Then no panic occurs and no FOUNDATION.md is written
        assert!(!tmp.path().join("spec").join("FOUNDATION.md").exists());
    }

    #[test]
    fn generate_renders_header_only_when_no_optional_sections() {
        // @step Given a foundation with a project name but no personas or diagrams
        let tmp = tempfile::tempdir().unwrap();
        write_foundation(tmp.path(), Value::Object(valid_base()));
        // @step When the markdown is generated
        generate(tmp.path(), None).unwrap();
        let md = std::fs::read_to_string(tmp.path().join("spec").join("FOUNDATION.md")).unwrap();
        // @step Then the title header is rendered
        assert!(md.contains("# Demo Project Foundation"));
        assert!(md.starts_with("<!-- THIS FILE IS AUTO-GENERATED"));
    }

    #[test]
    fn generate_falls_back_to_default_header_when_no_project_name() {
        // @step Given a name-less foundation, the generator falls back to the default header
        let mut base = valid_base();
        // A name-less foundation is schema-invalid (project.name has minLength 1
        // and project is required), so it would not regenerate via generate().
        // Exercise the renderer directly to assert the fallback header.
        base.remove("project");
        let foundation = Value::Object(base);
        // @step When the markdown is generated directly (bypassing the schema gate)
        let md = generate_foundation_md(&foundation).unwrap();
        // @step Then a fallback header is rendered
        assert!(md.contains("# Project Foundation"));
    }

    #[test]
    fn diagram_error_message_singular() {
        // @step Given architectureDiagrams contains one diagram that fails the Mermaid pre-check
        // @step When generation runs
        let msg = build_diagram_error_message(&[
            "architectureDiagrams[0] (title: \"X\"): bad".to_string()
        ]);
        // @step Then the error message starts with "Diagram validation failed!\n\nFound 1 invalid diagram:\n"
        assert!(msg.starts_with("Diagram validation failed!\n\nFound 1 invalid diagram:\n"));
        // @step And the error message ends with "Run 'fspec generate-foundation-md' again after fixing"
        assert!(msg.ends_with("Run 'fspec generate-foundation-md' again after fixing"));
    }

    #[test]
    fn diagram_error_message_plural() {
        // @step Given architectureDiagrams contains two diagrams that fail the Mermaid pre-check
        // @step When generation runs
        let msg = build_diagram_error_message(&["a".to_string(), "b".to_string()]);
        // @step Then the error message contains "Found 2 invalid diagrams:"
        assert!(msg.contains("Found 2 invalid diagrams:"));
    }

    #[test]
    fn reports_diagram_with_genuine_merman_syntax_error_as_invalid() {
        // Scenario: Reports a diagram with a genuine merman syntax error as invalid

        // @step Given spec/foundation.json has one architectureDiagram whose mermaidCode has an unterminated node label
        let tmp = tempfile::tempdir().unwrap();
        let mut base = valid_base();
        base.insert(
            "architectureDiagrams".to_string(),
            json!([
                { "title": "Broken", "mermaidCode": "flowchart TD\n  A[Start --> B[Done" }
            ]),
        );
        write_foundation(tmp.path(), Value::Object(base));

        // @step When I dispatch generate-foundation-md
        let err = generate(tmp.path(), None).unwrap_err();

        // @step Then generation fails reporting 'Found 1 invalid diagram:'
        assert!(
            err.to_string().contains("Found 1 invalid diagram:"),
            "got: {err}"
        );
        // The diagram was caught by merman, not the regex pre-check, and nothing was written.
        assert!(!tmp.path().join("spec").join("FOUNDATION.md").exists());
    }

    #[test]
    fn all_generator_emitted_diagrams_parse_cleanly_under_merman() {
        // Scenario: All generator-emitted diagrams parse cleanly under merman

        // @step Given a foundation with bounded contexts, aggregates, commands and events
        let tmp = tempfile::tempdir().unwrap();
        let mut base = valid_base();
        base.insert(
            "eventStorm".to_string(),
            json!({
                "level": "big_picture",
                "items": [
                    { "type": "bounded_context", "id": 1, "text": "Work Management", "color": null, "deleted": false, "createdAt": "2024-01-01T00:00:00.000Z" },
                    { "type": "bounded_context", "id": 2, "text": "Specification", "color": null, "deleted": false, "createdAt": "2024-01-01T00:00:00.000Z" },
                    { "type": "command", "id": 3, "text": "Create Work Unit", "boundedContextId": 1, "color": "blue", "deleted": false, "createdAt": "2024-01-01T00:00:00.000Z" },
                    { "type": "aggregate", "id": 4, "text": "Work Unit", "boundedContextId": 1, "color": "yellow", "deleted": false, "createdAt": "2024-01-01T00:00:00.000Z" },
                    { "type": "event", "id": 5, "text": "Work Unit Created", "boundedContextId": 1, "color": "orange", "deleted": false, "createdAt": "2024-01-01T00:00:00.000Z" }
                ],
                "nextItemId": 6
            }),
        );
        write_foundation(tmp.path(), Value::Object(base));

        // @step When the FOUNDATION.md generator emits the bounded-context map and per-context event-flow diagrams
        let message = generate(tmp.path(), None).unwrap();

        // @step Then every emitted diagram validates as Ok under merman so valid foundations still render byte-identically
        assert_eq!(
            message,
            "Generated spec/FOUNDATION.md from spec/foundation.json"
        );
        let md = std::fs::read_to_string(tmp.path().join("spec").join("FOUNDATION.md")).unwrap();
        assert!(md.contains("```mermaid"), "diagrams should be emitted");
        assert!(md.contains("## Bounded Context Map"));
        assert!(md.contains("### Event Flow"));
    }
}
