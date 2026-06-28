//! `update-scenario` — Rust port of `src/commands/update-scenario.ts` (RPC-314).
//!
//! Renames a scenario in a Gherkin feature file by a line-based edit: parse
//! the feature to locate the scenario header line, replace the name in-place
//! (preserving indentation and the `Scenario`/`Scenario Outline` keyword),
//! re-join and write. Then rename the matching entry in the sibling
//! `.feature.coverage` file so test/impl mappings are preserved.
//!
//! Mirrors the TypeScript reference byte-for-byte including:
//!   - feature-path resolution (`.feature` suffix, `spec/features/` prefix,
//!     or bare name → `spec/features/<name>.feature`);
//!   - canonical envelopes (`{success:true,message}` /
//!     `{success:false,error}`) — soft failures return `Ok(json)` with
//!     `success:false` so the dispatcher surfaces them as data, NOT as a
//!     `DispatchResult` error;
//!   - error texts: `Feature file not found: <path>`,
//!     `Scenario '<old>' not found in feature file`,
//!     `Scenario '<new>' already exists in this feature`;
//!   - coverage rename: locate `scenarios[].name == old` and set it to the
//!     new name, leaving everything else (incl. test/impl mappings) intact;
//!     missing/invalid coverage is silently skipped (still succeeds).
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/update_scenario.rs` is JSON marshalling only — no
//! domain logic.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateScenarioArgs {
    feature: String,
    old_name: String,
    new_name: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateScenarioArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-scenario",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Resolve feature file path (TS src/commands/update-scenario.ts:27-35) ----
    let feature_rel = resolve_feature_rel(&args.feature);
    let feature_abs = project_root.join(&feature_rel);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&feature_abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return err_envelope(format!("Feature file not found: {}", feature_abs.display()));
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "update-scenario",
                source,
            });
        }
    };

    // ---- Parse Gherkin to validate the document & locate scenarios ----
    // TS `@cucumber/gherkin` PARSES an empty / comment-only document
    // successfully but yields `gherkinDocument.feature === null`, which the
    // TS command surfaces as `Feature file does not contain a valid Feature`.
    // The Rust `gherkin-0.16` parser instead throws a syntax error on the
    // same inputs, so we replicate the TS "null feature" classification
    // explicitly before delegating to the lenient parser.
    if is_empty_or_comment_only(&content) {
        return err_envelope("Feature file does not contain a valid Feature".to_string());
    }
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            return err_envelope(format!("Invalid Gherkin syntax: {e}"));
        }
    };

    // ---- Find the scenario to rename ----
    let target = feature.scenarios.iter().find(|s| s.name == args.old_name);
    let target = match target {
        Some(s) => s,
        None => {
            return err_envelope(format!(
                "Scenario '{}' not found in feature file",
                args.old_name
            ));
        }
    };

    // ---- Duplicate detection (new name already present?) ----
    if feature.scenarios.iter().any(|s| s.name == args.new_name) {
        return err_envelope(format!(
            "Scenario '{}' already exists in this feature",
            args.new_name
        ));
    }

    // ---- Line-based header rewrite (preserve indentation + keyword) ----
    // gherkin-0.16 `position.line` is 1-based, matching @cucumber/gherkin.
    let scenario_line = target.position.line as usize;
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    let line_index = scenario_line.saturating_sub(1);
    if line_index >= lines.len() {
        return err_envelope("Could not parse scenario header line".to_string());
    }
    let header = &lines[line_index];

    let (indentation, keyword) = match parse_scenario_header(header) {
        Some(parts) => parts,
        None => {
            return err_envelope("Could not parse scenario header line".to_string());
        }
    };
    lines[line_index] = format!("{indentation}{keyword}: {}", args.new_name);
    let new_content = lines.join("\n");

    // ---- Validate the rewritten content is still parseable Gherkin ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return err_envelope(format!("Renaming would result in invalid Gherkin: {e}"));
    }

    // ---- Write the updated feature file ----
    std::fs::write(&feature_abs, &new_content).map_err(|source| FspecCoreError::Io {
        command: "update-scenario",
        source,
    })?;

    // ---- Rename the coverage entry (best-effort; skip on any failure) ----
    let coverage_path = {
        let mut p = feature_abs.clone().into_os_string();
        p.push(".coverage");
        std::path::PathBuf::from(p)
    };
    if let Ok(cov_body) = std::fs::read_to_string(&coverage_path) {
        if let Ok(mut cov) = serde_json::from_str::<Value>(&cov_body) {
            let mut changed = false;
            if let Some(scenarios) = cov.get_mut("scenarios").and_then(Value::as_array_mut) {
                for entry in scenarios.iter_mut() {
                    if entry.get("name").and_then(Value::as_str) == Some(args.old_name.as_str()) {
                        entry["name"] = Value::String(args.new_name.clone());
                        changed = true;
                    }
                }
            }
            if changed {
                if let Ok(serialised) = serde_json::to_string_pretty(&cov) {
                    let _ = std::fs::write(&coverage_path, serialised);
                }
            }
        }
    }

    // ---- Success envelope ----
    let file_name = feature_abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&feature_rel)
        .to_string();
    let response = json!({
        "success": true,
        "message": format!(
            "Successfully renamed scenario to '{}' in {}",
            args.new_name, file_name
        ),
    });
    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-scenario",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// True when the document contains no Gherkin content that
/// `@cucumber/gherkin` would turn into a `feature` node — i.e. every line
/// is blank, whitespace, or a comment (`#…`) AND none of the comment lines
/// is a `# language: <lang>` directive. In TS such a document parses
/// without error but produces `gherkinDocument.feature === null`.
///
/// The language directive regex matches `@cucumber/gherkin`'s
/// `GherkinClassicTokenMatcher` `LANGUAGE_PATTERN`:
/// `^\s*#\s*language\s*:\s*([a-zA-Z\-_]+)\s*$`. A document containing a
/// language directive (anywhere) but no feature is a parse error in TS,
/// so we must NOT classify it as "null feature".
fn is_empty_or_comment_only(content: &str) -> bool {
    let mut saw_only_blank_or_comment = true;
    for line in content.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(after_hash) = trimmed.strip_prefix('#') {
            if is_language_directive(after_hash) {
                // Language directive with no following feature → TS throws.
                return false;
            }
            continue;
        }
        // A non-blank, non-comment line means there is real content; the
        // strict/lenient parser (or the `feature` lookup) governs the
        // outcome, not this fast path.
        saw_only_blank_or_comment = false;
        break;
    }
    saw_only_blank_or_comment
}

/// Returns true when the text after a leading `#` matches the
/// `@cucumber/gherkin` language directive `\s*language\s*:\s*([a-zA-Z\-_]+)\s*`.
fn is_language_directive(after_hash: &str) -> bool {
    let rest = after_hash.trim_start();
    let Some(rest) = rest.strip_prefix("language") else {
        return false;
    };
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let value: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '-' || *c == '_')
        .collect();
    if value.is_empty() {
        return false;
    }
    // Everything after the value must be trailing whitespace only.
    rest[value.len()..].trim().is_empty()
}

/// Mirror of TS feature-path resolution (`src/commands/update-scenario.ts:27-35`).
fn resolve_feature_rel(feature: &str) -> String {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        feature.to_string()
    } else {
        format!("spec/features/{feature}.feature")
    }
}

/// Parse a scenario header line, returning `(indentation, keyword)` where
/// keyword is `Scenario` or `Scenario Outline`. Mirrors the TS regex
/// `^(\s*)(Scenario|Scenario Outline):\s*(.+)$`.
fn parse_scenario_header(line: &str) -> Option<(String, String)> {
    let trimmed_start = line.trim_start();
    let indent_len = line.len() - trimmed_start.len();
    let indentation = line[..indent_len].to_string();

    // Longest keyword first so "Scenario Outline" wins over "Scenario".
    for keyword in ["Scenario Outline", "Scenario"] {
        if let Some(rest) = trimmed_start.strip_prefix(keyword) {
            if let Some(after_colon) = rest.strip_prefix(':') {
                // TS regex requires `\s*(.+)` — at least one non-empty char
                // after optional whitespace.
                if !after_colon.trim().is_empty() {
                    return Some((indentation, keyword.to_string()));
                }
            }
        }
    }
    None
}

/// Build an `Ok(json)` soft-failure envelope `{success:false,error}` so the
/// dispatcher surfaces the error inside `DispatchResult.data` (parity with
/// the TS `{ success:false, error }` return shape).
fn err_envelope(error: String) -> Result<String, FspecCoreError> {
    let v = json!({ "success": false, "error": error });
    serde_json::to_string(&v).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-scenario",
        reason: format!("failed to serialise error envelope: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: UpdateScenarioArgs = serde_json::from_str(
            r#"{"feature":"spec/features/x.feature","oldName":"A","newName":"B"}"#,
        )
        .unwrap();
        assert_eq!(a.feature, "spec/features/x.feature");
        assert_eq!(a.old_name, "A");
        assert_eq!(a.new_name, "B");
    }

    #[test]
    fn resolve_paths() {
        assert_eq!(resolve_feature_rel("x.feature"), "x.feature");
        assert_eq!(
            resolve_feature_rel("spec/features/x.feature"),
            "spec/features/x.feature"
        );
        assert_eq!(
            resolve_feature_rel("user-login"),
            "spec/features/user-login.feature"
        );
    }

    #[test]
    fn header_parse() {
        assert_eq!(
            parse_scenario_header("  Scenario: Foo"),
            Some(("  ".to_string(), "Scenario".to_string()))
        );
        assert_eq!(
            parse_scenario_header("  Scenario Outline: Bar"),
            Some(("  ".to_string(), "Scenario Outline".to_string()))
        );
        assert_eq!(parse_scenario_header("  Given x"), None);
        assert_eq!(parse_scenario_header("  Scenario:"), None);
    }

    #[test]
    fn empty_or_comment_only_classification() {
        // Null-feature inputs (TS parses OK, feature == null).
        assert!(is_empty_or_comment_only(""));
        assert!(is_empty_or_comment_only("   "));
        assert!(is_empty_or_comment_only("\n\n"));
        assert!(is_empty_or_comment_only("# c"));
        assert!(is_empty_or_comment_only("# c\n"));
        assert!(is_empty_or_comment_only("  # indented\n\t# tab\n"));
        assert!(is_empty_or_comment_only("# language\n")); // no colon → not a directive
        assert!(is_empty_or_comment_only("# language: \n")); // empty value
        assert!(is_empty_or_comment_only("# language: e n\n")); // space breaks value
        assert!(is_empty_or_comment_only("# language: 123\n")); // digits not [a-zA-Z-_]
        assert!(is_empty_or_comment_only("# this is the language file\n"));
        assert!(is_empty_or_comment_only("# language: en extra\n")); // trailing text breaks directive

        // NOT null-feature (TS throws): language directive or real content.
        assert!(!is_empty_or_comment_only("# language: en\n"));
        assert!(!is_empty_or_comment_only("# a\n# language: en\n"));
        assert!(!is_empty_or_comment_only("@foo\n"));
        assert!(!is_empty_or_comment_only("random text\n"));
        assert!(!is_empty_or_comment_only("Feature: X\n"));
    }

    #[test]
    fn language_directive_matches_cucumber_pattern() {
        // These match the LANGUAGE_PATTERN regex (directive recognised), so
        // `is_empty_or_comment_only` must treat them as NOT-null-feature.
        // (Whether the language code is *supported* is a separate parser
        // concern; the directive presence alone disables the fast path.)
        assert!(is_language_directive(" language: en"));
        assert!(is_language_directive("language:en"));
        assert!(is_language_directive(" language : en-US "));
        assert!(is_language_directive("language: e_n"));
        assert!(!is_language_directive(" language"));
        assert!(!is_language_directive(" language:"));
        assert!(!is_language_directive(" language: 1"));
        assert!(!is_language_directive(" lang: en"));
        assert!(!is_language_directive(" language: en extra"));
    }
}
