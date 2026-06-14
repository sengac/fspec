//! `validate` — Rust port of `src/commands/validate.ts` (RPC-320).
//!
//! Validates Gherkin syntax across `spec/features/**/*.feature` (or a single
//! supplied file) using the lenient gherkin front-end in
//! [`crate::io::gherkin::parse_feature_lenient`], plus the content-string
//! heuristics ported verbatim from the TS `checkForCommonIssues` /
//! `getSuggestion` helpers.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## Exit-code transport
//!
//! The TS command `process.exit`s with 0 (all valid) / 1 (one or more invalid)
//! / 2 (no feature files found OR unexpected error). The Rust core cannot exit
//! the process, so it carries the intended code in the returned JSON envelope:
//!
//!   - **all valid / has-invalid** → `Ok(payload)` where `payload` is
//!     `{success, output, exitCode, results, invalidCount}`. `exitCode` is 0
//!     when every file is valid, 1 when one or more are invalid. The shell
//!     bridge prints `output` and exits with `exitCode`.
//!   - **zero feature files / unexpected error** → `Err(FspecCoreError)`. The
//!     shell bridge maps any `Err` to exit 2 and writes the message to stderr
//!     (mirrors the list-features `DirectoryNotFound → 2` precedent).
//!
//! ## RPC-329 known divergence
//!
//! The embedded raw parser-error TEXT diverges from `@cucumber/gherkin`
//! (tracked separately under RPC-329). The error CLASSIFICATION, exit codes,
//! `Suggestion` lines, and the two content-heuristic messages all match TS;
//! only the verbatim parser message text differs. We surface the gherkin
//! crate's `Display` string as the message (sibling-command precedent:
//! `add_scenario.rs`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

/// CLI arguments accepted by `validate`. Mirrors the Commander.js registration
/// at `src/commands/validate.ts:256-265`: an optional positional `[file]` and
/// a `-v/--verbose` boolean (default false).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ValidateArgs {
    /// Specific feature file to validate; when absent all feature files under
    /// `spec/features/` are validated.
    file: Option<String>,
    /// Show detailed validation output. Accepted for parity; the structured
    /// payload is unaffected by it.
    verbose: bool,
}

/// A single validation error attached to a [`FileResult`].
#[derive(Debug, Clone, Serialize)]
struct ValidationError {
    line: usize,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

/// Per-file validation outcome (mirrors the TS `ValidationResult` interface).
#[derive(Debug, Clone, Serialize)]
struct FileResult {
    file: String,
    valid: bool,
    errors: Vec<ValidationError>,
    /// Verbose log lines emitted by `validateFile` while `verbose` is set
    /// (the TS command interleaves these `output.log` calls with the result
    /// rendering). Not part of the structured payload — used only to build the
    /// display block. Skipped from serialization to keep the JSON shape
    /// matching the TS `ValidationResult` interface.
    #[serde(skip)]
    verbose_lines: Vec<String>,
}

/// Dispatcher entry point. `project_root` is supplied by both front doors.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ValidateArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "validate",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Resolve the file set. A single-file argument bypasses the glob; the
    // default path globs spec/features/**/*.feature. A MISSING spec/features
    // directory is treated like an empty glob result (TS tinyglobby returns
    // [] rather than throwing), so it funnels into the "no feature files"
    // exit-2 branch below.
    let files: Vec<String> = match args.file.as_deref() {
        Some(f) => vec![f.to_string()],
        None => match glob_feature_files(project_root) {
            Ok(v) => v,
            Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
            Err(other) => return Err(other),
        },
    };

    if files.is_empty() {
        // Parity with `src/commands/validate.ts:27-30`: no feature files →
        // stderr message + process.exit(2). The shell bridge maps any Err to
        // exit 2; the verbatim message is carried through the error Display.
        return Err(FspecCoreError::FoundationMissing(
            "No feature files found in spec/features/".to_string(),
        ));
    }

    let results: Vec<FileResult> = files
        .iter()
        .map(|f| validate_file(project_root, f, args.verbose))
        .collect();

    // Build the rendered display block as a Vec of lines joined with '\n' (no
    // trailing newline) — mirrors the TS `output.log` line sequence.
    //
    // The TS command runs `validateFile` for every file via `Promise.all`
    // BEFORE the result-rendering loop. Each `validateFile` emits its own
    // `output.log` verbose lines (`Parsing <file>...`, then on success
    // `  AST generated successfully` / `  Feature: <name>` / `  Scenarios: <N>`)
    // as a side effect during that phase. So when `--verbose` is set the entire
    // verbose block (in file order) precedes the per-file `✓/✗` result lines.
    let mut lines: Vec<String> = Vec::new();
    if args.verbose {
        for result in &results {
            lines.extend(result.verbose_lines.iter().cloned());
        }
    }
    for result in &results {
        if result.valid {
            lines.push(format!("✓ {} is valid", result.file));
        } else {
            lines.push(format!("✗ {} has syntax errors:", result.file));
            for error in &result.errors {
                lines.push(format!("  Line {}: {}", error.line, error.message));
                if let Some(s) = &error.suggestion {
                    lines.push(format!("  Suggestion: {s}"));
                }
            }
        }
    }

    let valid_count = results.iter().filter(|r| r.valid).count();
    let invalid_count = results.len() - valid_count;

    // Summary line only when more than one file is validated (parity with the
    // `results.length > 1` guard).
    if results.len() > 1 {
        lines.push(String::new());
        if invalid_count == 0 {
            lines.push(format!("✓ All {} feature files are valid", results.len()));
        } else {
            lines.push(format!(
                "Validated {} files: {valid_count} valid, {invalid_count} invalid",
                results.len()
            ));
        }
    }

    let exit_code = if invalid_count > 0 { 1 } else { 0 };
    let output = lines.join("\n");

    let payload = json!({
        "success": invalid_count == 0,
        "output": output,
        "exitCode": exit_code,
        "invalidCount": invalid_count,
        "results": results,
    });
    serde_json::to_string(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Validate a single feature file (mirrors the TS `validateFile`).
fn validate_file(project_root: &Path, file_path: &str, verbose: bool) -> FileResult {
    let resolved = project_root.join(file_path);

    // Verbose log lines accumulated in the same order TS `output.log`s them.
    let mut verbose_lines: Vec<String> = Vec::new();

    let content = match std::fs::read_to_string(&resolved) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return FileResult {
                file: file_path.to_string(),
                valid: false,
                errors: vec![ValidationError {
                    line: 0,
                    message: format!("File not found: {file_path}"),
                    suggestion: None,
                }],
                verbose_lines,
            };
        }
        Err(e) => {
            return FileResult {
                file: file_path.to_string(),
                valid: false,
                errors: vec![ValidationError {
                    line: 0,
                    message: e.to_string(),
                    suggestion: None,
                }],
                verbose_lines,
            };
        }
    };

    // TS emits `Parsing <file>...` immediately after a successful read, BEFORE
    // attempting the parse (so it appears even when the parse later fails).
    if verbose {
        verbose_lines.push(format!("Parsing {file_path}..."));
    }

    // Parse with the lenient gherkin front-end. On parse error the file is
    // invalid with a single {line, message, suggestion} error (RPC-329: the
    // raw message text diverges from @cucumber/gherkin but the classification
    // matches).
    match parse_feature_lenient(&content) {
        Err(parse_err) => {
            let message = parse_err.to_string();
            let line = extract_line(&message);
            let suggestion = get_suggestion(&message);
            FileResult {
                file: file_path.to_string(),
                valid: false,
                errors: vec![ValidationError {
                    line,
                    message,
                    suggestion,
                }],
                verbose_lines,
            }
        }
        Ok(feature) => {
            // Parser succeeded: run the content-string heuristics (parity — TS
            // only runs checkForCommonIssues on the parse-success path).
            let additional = check_for_common_issues(&content);

            // TS emits the success verbose block AFTER the additional-errors
            // check (the `if (verbose)` block at validate.ts:126-134 runs in the
            // try body regardless of whether checkForCommonIssues found issues).
            if verbose {
                verbose_lines.push("  AST generated successfully".to_string());
                verbose_lines.push(format!("  Feature: {}", feature.name));
                verbose_lines.push(format!("  Scenarios: {}", feature_child_count(&feature)));
            }

            FileResult {
                file: file_path.to_string(),
                valid: additional.is_empty(),
                errors: additional,
                verbose_lines,
            }
        }
    }
}

/// Count of the parsed feature's top-level children, matching the TS
/// `gherkinDocument.feature.children.length`. In the cucumber AST `children`
/// is the ordered list of `background | scenario | rule` nodes — so a
/// `Background` counts as one child, each top-level `Scenario` counts as one,
/// and each `Rule` counts as one (its inner scenarios are NOT flattened).
fn feature_child_count(feature: &gherkin::Feature) -> usize {
    let background = usize::from(feature.background.is_some());
    background + feature.scenarios.len() + feature.rules.len()
}

/// Extract a 1-based line number from the gherkin crate's `ParseError`
/// `Display` string, which is `"Error at <line>:<col>: {expected:?}"`. Returns
/// 0 when the prefix is absent (parity with TS `parseError.location?.line || 0`).
fn extract_line(message: &str) -> usize {
    message
        .strip_prefix("Error at ")
        .and_then(|rest| rest.split(':').next())
        .and_then(|n| n.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

/// Port of the TS `checkForCommonIssues` content heuristics
/// (`src/commands/validate.ts:166-227`): unescaped triple quotes inside a
/// DocString, and more than 2 consecutive blank lines.
fn check_for_common_issues(content: &str) -> Vec<ValidationError> {
    let mut errors: Vec<ValidationError> = Vec::new();
    let lines: Vec<&str> = content.split('\n').collect();

    let mut in_doc_string = false;
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let line_num = i + 1;
        let trimmed = line.trim();

        // Track DocString boundaries.
        if trimmed == "\"\"\"" || trimmed.starts_with("\"\"\"") {
            in_doc_string = !in_doc_string;
            i += 1;
            continue;
        }

        // Unescaped triple quotes inside a DocString.
        if in_doc_string && line.contains("\"\"\"") && !line.contains("\\\"\"\"") {
            errors.push(ValidationError {
                line: line_num,
                message: "Unescaped triple quotes (\"\"\") found inside DocString".to_string(),
                suggestion: Some(
                    "Escape triple quotes with backslashes: \\\"\\\"\\\", or use triple backticks (```) as DocString delimiters instead"
                        .to_string(),
                ),
            });
        }

        // Excessive consecutive blank lines (more than 2).
        if i >= 2
            && lines[i].trim().is_empty()
            && lines[i - 1].trim().is_empty()
            && lines[i - 2].trim().is_empty()
        {
            let mut blank_count = 3usize;
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                blank_count += 1;
                j += 1;
            }
            if blank_count >= 3 {
                errors.push(ValidationError {
                    line: line_num,
                    message: format!(
                        "Excessive blank lines detected ({blank_count} consecutive blank lines)"
                    ),
                    suggestion: Some(
                        "Remove excess blank lines - Gherkin files should have at most 2 consecutive blank lines"
                            .to_string(),
                    ),
                });
                // Skip ahead to avoid duplicate errors.
                i += blank_count - 3;
            }
        }

        i += 1;
    }

    errors
}

/// Port of the TS `getSuggestion` heuristics (`src/commands/validate.ts:229-254`).
/// Parser-independent — keyed off the lowercased error message text.
fn get_suggestion(error_message: &str) -> Option<String> {
    let message = error_message.to_lowercase();

    if message.contains("expected") && message.contains("feature") {
        return Some("Add Feature keyword at the beginning of the file".to_string());
    }

    if message.contains("unexpected") || message.contains("invalid") {
        if message.contains("while") || message.contains("whilst") {
            return Some("Use: Given, When, Then, And, or But".to_string());
        }
        if message.contains("indent") {
            return Some(
                "Check indentation - steps should be indented 2 spaces from Scenario".to_string(),
            );
        }
    }

    if message.contains("doc string") || message.contains("\"\"\"") {
        return Some("Add closing \"\"\"".to_string());
    }

    if message.contains("table") {
        return Some(
            "Check data table formatting - each row must have same number of columns".to_string(),
        );
    }

    None
}

/// Helper used by tests to assert the JSON shape without going through the
/// async dispatcher.
#[cfg(test)]
fn parse_payload(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("payload is JSON")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn extract_line_parses_gherkin_prefix() {
        assert_eq!(extract_line("Error at 5:3: {\"#EOF\"}"), 5);
        assert_eq!(extract_line("some other message"), 0);
    }

    #[test]
    fn excessive_blank_lines_detected() {
        let content = "Feature: B\n\n  Scenario: A\n    Given x\n\n\n\n\n    Then y\n";
        let errs = check_for_common_issues(content);
        assert!(
            errs.iter().any(|e| e.message.contains("Excessive blank lines detected")),
            "expected blank-line heuristic; got {errs:?}"
        );
    }

    #[test]
    fn clean_content_has_no_common_issues() {
        let content = "Feature: B\n\n  Scenario: A\n    Given x\n    Then y\n";
        assert!(check_for_common_issues(content).is_empty());
    }

    #[test]
    fn get_suggestion_feature_keyword() {
        let s = get_suggestion("expected: #Feature, got 'Scenario'");
        assert_eq!(s.as_deref(), Some("Add Feature keyword at the beginning of the file"));
    }

    #[test]
    fn payload_helper_round_trips() {
        let v = parse_payload(r#"{"success":true,"exitCode":0}"#);
        assert_eq!(v["exitCode"].as_i64(), Some(0));
    }
}
