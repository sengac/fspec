//! `add-background` — Rust port of `src/commands/add-background.ts` (RPC-171).
//!
//! Adds or replaces the `Background: User Story` section of a Gherkin feature
//! file. Line-based mutation (NOT AST rewriting) to preserve the user's exact
//! formatting and comments upstream of the inserted block — mirrors the
//! TypeScript reference byte-for-byte.
//!
//! ## Behaviour parity with TypeScript (`src/commands/add-background.ts`)
//!
//! * Empty / whitespace-only text → `Background text cannot be empty`.
//! * Feature resolution: a `*.feature` suffix is treated as a project-root
//!   relative path; a bare name is matched against the basename (minus
//!   extension) of every `spec/features/**/*.feature` file (first match wins).
//! * Missing file → `Feature file not found: <feature>`.
//! * Invalid Gherkin in the source → `Invalid Gherkin syntax in feature file: <msg>`.
//! * No `Feature:` line → `No Feature line found in file`.
//! * The Background block is inserted after a Feature-line doc-string (or, if
//!   none, after the Feature line); an existing Background section is replaced
//!   in place.
//! * Generated content is re-validated; failure → `Generated invalid Gherkin: <msg>`.
//! * Success → `{success:true, message:"Added background to <feature>"}`.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/add_background.rs` is JSON marshalling only — no domain
//! logic.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddBackgroundArgs {
    feature: String,
    text: String,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddBackgroundArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-background",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Validate background text ----
    if args.text.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-background",
            reason: "Background text cannot be empty".to_string(),
        });
    }

    // ---- Resolve the feature file ----
    let feature_path = match resolve_feature_path(project_root, &args.feature)? {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-background",
                reason: format!("Feature file not found: {}", args.feature),
            });
        }
    };

    // ---- Read the feature file ----
    let content = std::fs::read_to_string(&feature_path).map_err(|source| FspecCoreError::Io {
        command: "add-background",
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
            command: "add-background",
            reason: "No Feature line found in file".to_string(),
        });
    }

    // ---- Parse to validate current Gherkin ----
    if let Err(e) = parse_feature_lenient(&content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-background",
            reason: format!("Invalid Gherkin syntax in feature file: {e}"),
        });
    }

    // ---- Mutate (line-based) ----
    let new_content = insert_or_replace_background(&content, &args.text)?;

    // ---- Re-validate generated content ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-background",
            reason: format!("Generated invalid Gherkin: {e}"),
        });
    }

    // ---- Write file ----
    std::fs::write(&feature_path, &new_content).map_err(|source| FspecCoreError::Io {
        command: "add-background",
        source,
    })?;

    let response = json!({
        "success": true,
        "message": format!("Added background to {}", args.feature),
    });

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-background",
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

/// Line-based insertion / replacement of the `Background: User Story` block.
/// Mirrors `src/commands/add-background.ts:88-211` exactly, including the
/// `split('\n')` → `join("\n")` round-trip that preserves the source's line
/// terminators.
fn insert_or_replace_background(content: &str, text: &str) -> Result<String, FspecCoreError> {
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

    // Find the Feature line.
    let feature_line_index = match lines
        .iter()
        .position(|l| l.trim_start().starts_with("Feature:"))
    {
        Some(i) => i,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-background",
                reason: "No Feature line found in file".to_string(),
            });
        }
    };

    // Find the end of the Feature-line doc string (if it exists).
    let mut doc_string_end_index = feature_line_index;
    let mut in_doc_string = false;
    let mut i = feature_line_index + 1;
    while i < lines.len() {
        let line = lines[i].trim();
        if !in_doc_string
            && (line.starts_with("Background:")
                || line.starts_with("Scenario:")
                || line.starts_with('@'))
        {
            break;
        }
        if line == "\"\"\"" {
            if !in_doc_string {
                in_doc_string = true;
            } else {
                doc_string_end_index = i;
                break;
            }
        }
        i += 1;
    }

    // Check for an existing Background section.
    let mut existing_background_start: isize = -1;
    let mut existing_background_end: isize = -1;
    let mut j = doc_string_end_index + 1;
    while j < lines.len() {
        let line = lines[j].trim();

        if existing_background_start == -1
            && (line.starts_with("Scenario:") || line.starts_with('@'))
        {
            break;
        }

        if line.starts_with("Background:") {
            existing_background_start = j as isize;
            j += 1;
            continue;
        }

        if existing_background_start != -1
            && (line.starts_with("Scenario:")
                || line.starts_with('@')
                || line.starts_with("Feature:"))
        {
            existing_background_end = j as isize - 1;
            while existing_background_end > existing_background_start
                && lines[existing_background_end as usize].trim().is_empty()
            {
                existing_background_end -= 1;
            }
            break;
        }
        j += 1;
    }

    // Background section started but didn't end (goes to EOF).
    if existing_background_start != -1 && existing_background_end == -1 {
        existing_background_end = lines.len() as isize - 1;
        while existing_background_end > existing_background_start
            && lines[existing_background_end as usize].trim().is_empty()
        {
            existing_background_end -= 1;
        }
    }

    // Build the new Background block.
    let mut background_lines: Vec<String> = vec!["  Background: User Story".to_string()];
    for text_line in text.split('\n') {
        background_lines.push(format!("    {text_line}"));
    }

    if existing_background_start != -1 && existing_background_end != -1 {
        // Replace existing Background (block + trailing blank line).
        let start = existing_background_start as usize;
        let end = existing_background_end as usize;
        let mut replacement = background_lines;
        replacement.push(String::new());
        lines.splice(start..=end, replacement);
    } else {
        // Insert new Background after the doc string (or Feature line).
        let at = doc_string_end_index + 1;
        let mut block: Vec<String> = Vec::with_capacity(background_lines.len() + 2);
        block.push(String::new());
        block.extend(background_lines);
        block.push(String::new());
        lines.splice(at..at, block);
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddBackgroundArgs =
            serde_json::from_str(r#"{"feature":"login","text":"As a user"}"#).unwrap();
        assert_eq!(a.feature, "login");
        assert_eq!(a.text, "As a user");
    }

    #[test]
    fn inserts_background_after_feature_line() {
        let src = "Feature: Login\n  Scenario: A\n    Given x\n";
        let out = insert_or_replace_background(src, "As a user").unwrap();
        assert!(out.lines().any(|l| l == "  Background: User Story"));
        assert!(out.lines().any(|l| l == "    As a user"));
    }

    #[test]
    fn replaces_existing_background() {
        let src =
            "Feature: Login\n\n  Background: User Story\n    As old\n\n  Scenario: A\n    Given x\n";
        let out = insert_or_replace_background(src, "As new").unwrap();
        assert!(out.lines().any(|l| l == "    As new"));
        assert!(!out.lines().any(|l| l == "    As old"));
        let count = out
            .lines()
            .filter(|l| l.trim() == "Background: User Story")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn no_feature_line_rejected() {
        let err = insert_or_replace_background("# comment\n", "As a user").unwrap_err();
        match err {
            FspecCoreError::InvalidArgs { reason, .. } => {
                assert!(reason.contains("No Feature line found in file"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
