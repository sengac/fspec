//! `add-architecture` — Rust port of `src/commands/add-architecture.ts` (RPC-167).
//!
//! Adds or replaces an architecture-notes doc-string (`"""…"""`) immediately
//! after the `Feature:` line of a Gherkin feature file. Line-based mutation
//! (NOT AST rewriting) to preserve the user's exact formatting — mirrors the
//! TypeScript reference byte-for-byte.
//!
//! ## Behaviour parity with TypeScript (`src/commands/add-architecture.ts`)
//!
//! * Empty / whitespace-only text → `Architecture text cannot be empty`.
//! * Feature resolution: a `*.feature` suffix is treated as a project-root
//!   relative path; a bare name is matched against the basename (minus
//!   extension) of every `spec/features/**/*.feature` file (first match wins).
//! * Missing file → `Feature file not found: <feature>`.
//! * Invalid Gherkin in the source → `Invalid Gherkin syntax in feature file: <msg>`.
//! * No `Feature:` line → `No Feature line found in file`.
//! * The doc-string is inserted right after the Feature line; an existing
//!   doc-string in that position is replaced in place.
//! * Generated content is re-validated; failure → `Generated invalid Gherkin: <msg>`.
//! * Success → `{success:true, message:"Added architecture documentation to <feature>"}`.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/add_architecture.rs` is JSON marshalling only — no
//! domain logic.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddArchitectureArgs {
    feature: String,
    text: String,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddArchitectureArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-architecture",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Validate architecture text ----
    if args.text.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-architecture",
            reason: "Architecture text cannot be empty".to_string(),
        });
    }

    // ---- Resolve the feature file ----
    let feature_path = match resolve_feature_path(project_root, &args.feature)? {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-architecture",
                reason: format!("Feature file not found: {}", args.feature),
            });
        }
    };

    // ---- Read the feature file ----
    let content = std::fs::read_to_string(&feature_path).map_err(|source| FspecCoreError::Io {
        command: "add-architecture",
        source,
    })?;

    // ---- No `Feature:` line guard (TS parity) ----
    //
    // The TS reference parses with cucumber-gherkin, which ACCEPTS a
    // comment-only / Feature-less file as valid, then falls through to the
    // explicit `No Feature line found in file` check. Our `parse_feature_lenient`
    // is stricter and would reject such a file at parse time with a generic
    // syntax error. To emit the identical canonical message TS produces for this
    // input, we perform the no-Feature-line check *before* parsing. Files that
    // DO contain a `Feature:` line still flow through the parser below, so
    // genuine syntax errors are still surfaced as `Invalid Gherkin syntax …`.
    if !content
        .split('\n')
        .any(|l| l.trim_start().starts_with("Feature:"))
    {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-architecture",
            reason: "No Feature line found in file".to_string(),
        });
    }

    // ---- Parse to validate current Gherkin ----
    if let Err(e) = parse_feature_lenient(&content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-architecture",
            reason: format!("Invalid Gherkin syntax in feature file: {e}"),
        });
    }

    // ---- Mutate (line-based) ----
    let new_content = insert_or_replace_doc_string(&content, &args.text)?;

    // ---- Re-validate generated content ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-architecture",
            reason: format!("Generated invalid Gherkin: {e}"),
        });
    }

    // ---- Write file ----
    std::fs::write(&feature_path, &new_content).map_err(|source| FspecCoreError::Io {
        command: "add-architecture",
        source,
    })?;

    let response = json!({
        "success": true,
        "message": format!("Added architecture documentation to {}", args.feature),
    });

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-architecture",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Resolve a feature reference to an absolute path.
///
/// * `*.feature` suffix → project-root-relative path (must exist).
/// * Bare name → first `spec/features/**/*.feature` whose basename (minus
///   extension) equals the input. Mirrors `show_feature::resolve_feature_path`.
fn resolve_feature_path(
    project_root: &Path,
    input: &str,
) -> Result<Option<PathBuf>, FspecCoreError> {
    if input.ends_with(".feature") {
        let p = project_root.join(input);
        if p.exists() {
            return Ok(Some(p));
        }
        return Ok(None);
    }
    let files = glob_feature_files(project_root)?;
    for rel in files {
        let basename = rel
            .rsplit('/')
            .next()
            .unwrap_or(&rel)
            .trim_end_matches(".feature");
        if basename == input {
            return Ok(Some(project_root.join(rel)));
        }
    }
    Ok(None)
}

/// Line-based insertion / replacement of a Feature-line architecture
/// doc-string. Mirrors `src/commands/add-architecture.ts:88-158` exactly,
/// including the `split('\n')` → `join("\n")` round-trip that preserves the
/// source's line terminators.
fn insert_or_replace_doc_string(content: &str, text: &str) -> Result<String, FspecCoreError> {
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

    // Find the Feature line.
    let feature_line_index = match lines
        .iter()
        .position(|l| l.trim_start().starts_with("Feature:"))
    {
        Some(i) => i,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-architecture",
                reason: "No Feature line found in file".to_string(),
            });
        }
    };

    // Detect an existing doc string after the Feature line.
    let mut existing_doc_string_start: isize = -1;
    let mut existing_doc_string_end: isize = -1;
    let mut i = feature_line_index + 1;
    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("Background:") || line.starts_with("Scenario:") || line.starts_with('@')
        {
            break;
        }

        if line == "\"\"\"" && existing_doc_string_start == -1 {
            existing_doc_string_start = i as isize;
            i += 1;
            continue;
        }

        if line == "\"\"\"" && existing_doc_string_start != -1 {
            existing_doc_string_end = i as isize;
            break;
        }
        i += 1;
    }

    // Build the new doc string block.
    let mut doc_string_lines: Vec<String> = vec!["  \"\"\"".to_string()];
    for text_line in text.split('\n') {
        doc_string_lines.push(format!("  {text_line}"));
    }
    doc_string_lines.push("  \"\"\"".to_string());

    if existing_doc_string_start != -1 && existing_doc_string_end != -1 {
        // Replace existing doc string.
        let start = existing_doc_string_start as usize;
        let end = existing_doc_string_end as usize;
        lines.splice(start..=end, doc_string_lines);
    } else {
        // Insert new doc string immediately after the Feature line.
        let at = feature_line_index + 1;
        lines.splice(at..at, doc_string_lines);
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddArchitectureArgs =
            serde_json::from_str(r#"{"feature":"login","text":"Uses bcrypt"}"#).unwrap();
        assert_eq!(a.feature, "login");
        assert_eq!(a.text, "Uses bcrypt");
    }

    #[test]
    fn inserts_doc_string_after_feature_line() {
        let src = "Feature: Login\n  Scenario: A\n    Given x\n";
        let out = insert_or_replace_doc_string(src, "Uses bcrypt").unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "Feature: Login");
        assert_eq!(lines[1].trim(), "\"\"\"");
        assert!(lines.contains(&"  Uses bcrypt"));
    }

    #[test]
    fn replaces_existing_doc_string() {
        let src = "Feature: Login\n  \"\"\"\n  Old\n  \"\"\"\n  Scenario: A\n    Given x\n";
        let out = insert_or_replace_doc_string(src, "New").unwrap();
        assert!(out.lines().any(|l| l == "  New"));
        assert!(!out.lines().any(|l| l == "  Old"));
        let fences = out.lines().filter(|l| l.trim() == "\"\"\"").count();
        assert_eq!(fences, 2);
    }

    #[test]
    fn multiline_doc_string() {
        let src = "Feature: Login\n  Scenario: A\n    Given x\n";
        let out = insert_or_replace_doc_string(src, "Uses bcrypt\nSessions in Redis").unwrap();
        assert!(out.lines().any(|l| l == "  Uses bcrypt"));
        assert!(out.lines().any(|l| l == "  Sessions in Redis"));
    }

    #[test]
    fn no_feature_line_rejected() {
        let err = insert_or_replace_doc_string("# comment\n", "Uses bcrypt").unwrap_err();
        match err {
            FspecCoreError::InvalidArgs { reason, .. } => {
                assert!(reason.contains("No Feature line found in file"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
