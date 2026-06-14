//! `add-scenario` — Rust port of `src/commands/add-scenario.ts` (RPC-190).
//!
//! Inserts a new `Scenario:` block (with placeholder Given/When/Then steps)
//! into an existing feature file via line-based editing. Mirrors the TS
//! reference exactly:
//!
//!   - resolves a bare identifier to `spec/features/<id>.feature`;
//!   - returns a soft-failure envelope (`success:false`) for a missing file
//!     or invalid Gherkin (the dispatcher envelope itself still succeeds);
//!   - warns (but still inserts) on a duplicate scenario name;
//!   - inserts the new scenario BEFORE the first `Scenario Outline:` /
//!     `Scenario Template:` if present, else at end of file;
//!   - honours `dryRun` (validate + report without writing).
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/add_scenario.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddScenarioArgs {
    feature: String,
    scenario: String,
    #[serde(default)]
    dry_run: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddScenarioArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-scenario",
            reason: format!("failed to parse args: {e}"),
        })?;

    let rel = resolve_feature_rel(&args.feature);
    let feature_abs = project_root.join(&rel);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&feature_abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return soft_fail(json!({
                "success": false,
                "valid": false,
                "error": format!("Feature file not found: {}", feature_abs.to_string_lossy()),
                "suggestion": "Use 'fspec create-feature' to create a new feature file",
            }));
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "add-scenario",
                source,
            });
        }
    };

    // ---- Classify empty / comment-only documents as "null feature" ----
    // `@cucumber/gherkin` PARSES an empty / whitespace / comment-only
    // document WITHOUT error but yields `gherkinDocument.feature === null`,
    // which the TS command surfaces as
    // "Feature file does not contain a valid Feature". The Rust
    // `gherkin-0.16.0` parser instead THROWS a syntax error on those same
    // inputs, so we replicate the TS "null feature" classification
    // explicitly BEFORE delegating to the lenient parser (mirrors PARITY-4's
    // fix in `update_scenario.rs`). A `# language: <lang>` directive with no
    // following feature is a genuine parse error in TS, so it must NOT be
    // classified here — it falls through to the parser and surfaces as a
    // syntax error.
    if is_empty_or_comment_only(&content) {
        return soft_fail(json!({
            "success": false,
            "valid": false,
            "error": "Feature file does not contain a valid Feature",
            "suggestion": format!("Run 'fspec validate {}' to see syntax errors", args.feature),
        }));
    }

    // ---- Validate existing Gherkin syntax ----
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            // PARITY NOTE: the TS reference embeds `@cucumber/gherkin`'s
            // multi-line `Parser errors:\n(line:col): expected: …, got '…'`
            // message here. The Rust `gherkin-0.16.0` crate's `ParseError`
            // only carries a single `(line:col)` + expected-token set, so a
            // byte-identical reproduction of the syntax-error TEXT is not
            // possible without a bespoke cucumber-compatible error formatter.
            // This residual text gap is shared by every ported Gherkin
            // command. We surface the crate's `{e}` here so the message is at
            // least as informative as the sibling commands' form.
            return soft_fail(json!({
                "success": false,
                "valid": false,
                "error": format!("Feature file has invalid Gherkin syntax: {e}"),
                "suggestion": format!("Run 'fspec validate {}' to see syntax errors", args.feature),
            }));
        }
    };
    // Mirror the TS `!gherkinDocument.feature` guard — an empty parse with no
    // feature keyword / name / scenarios is treated as "not a valid Feature".
    if feature.keyword.is_empty() && feature.name.is_empty() && feature.scenarios.is_empty() {
        return soft_fail(json!({
            "success": false,
            "valid": false,
            "error": "Feature file does not contain a valid Feature",
            "suggestion": format!("Run 'fspec validate {}' to see syntax errors", args.feature),
        }));
    }

    // ---- Duplicate scenario name detection ----
    // TS only inspects children whose `scenario.keyword === 'Scenario'`
    // (plain scenarios) — Scenario Outlines are excluded from the
    // duplicate check.
    let duplicate_exists = feature
        .scenarios
        .iter()
        .filter(|s| s.keyword == "Scenario")
        .any(|s| s.name == args.scenario);
    let warning: Option<String> = if duplicate_exists {
        Some(format!(
            "A scenario named \"{}\" already exists in this feature",
            args.scenario
        ))
    } else {
        None
    };

    // ---- Build new scenario template (matches TS scenarioTemplate) ----
    let scenario_template = format!(
        "\n  Scenario: {}\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n",
        args.scenario
    );

    // ---- Find insertion point: before Scenario Outline/Template, or EOF ----
    let lines: Vec<&str> = content.split('\n').collect();
    let mut insert_index = lines.len();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("Scenario Outline:") || trimmed.starts_with("Scenario Template:") {
            insert_index = i;
            break;
        }
    }

    // ---- Splice (mirror TS join/concat exactly) ----
    let head = lines[..insert_index].join("\n");
    let tail = lines[insert_index..].join("\n");
    let new_content = format!("{head}{scenario_template}\n{tail}");

    // ---- Validate result is still parseable Gherkin ----
    let valid = parse_feature_lenient(&new_content).is_ok();

    // ---- Write unless dry run ----
    if !args.dry_run {
        std::fs::write(&feature_abs, &new_content).map_err(|source| FspecCoreError::Io {
            command: "add-scenario",
            source,
        })?;
    }

    let mut response = json!({
        "success": true,
        "valid": valid,
    });
    if let Some(w) = warning {
        response["warning"] = Value::String(w);
    }
    serialize(response)
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Mirror of TS feature-path resolution
/// (`src/commands/add-scenario.ts:30-37`). The TS reference keeps the
/// `.feature` suffix and `spec/features/` prefix checks as two separate
/// branches that both return the identifier verbatim; we collapse them
/// into one condition (clippy `if_same_then_else`) — behaviour is
/// identical.
fn resolve_feature_rel(feature: &str) -> String {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        feature.to_string()
    } else {
        format!("spec/features/{feature}.feature")
    }
}

/// True when the document contains no Gherkin content that
/// `@cucumber/gherkin` would turn into a `feature` node — i.e. every line is
/// blank, whitespace, or a comment (`#…`) AND none of the comment lines is a
/// `# language: <lang>` directive. In TS such a document parses without error
/// but produces `gherkinDocument.feature === null`.
///
/// The language directive regex mirrors `@cucumber/gherkin`'s
/// `GherkinClassicTokenMatcher` `LANGUAGE_PATTERN`:
/// `^\s*#\s*language\s*:\s*([a-zA-Z\-_]+)\s*$`. A document containing a
/// language directive (anywhere) but no feature is a parse error in TS, so we
/// must NOT classify it as "null feature".
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
        // parser (or the `feature` lookup) governs the outcome, not this
        // fast path.
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

/// A soft failure: the inner `success:false` envelope is still returned as a
/// successful dispatch result (the TS command returns the result object
/// rather than throwing).
fn soft_fail(value: Value) -> Result<String, FspecCoreError> {
    serialize(value)
}

fn serialize(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-scenario",
        reason: format!("failed to serialise response: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn resolve_paths() {
        assert_eq!(resolve_feature_rel("login"), "spec/features/login.feature");
        assert_eq!(
            resolve_feature_rel("spec/features/login.feature"),
            "spec/features/login.feature"
        );
        assert_eq!(resolve_feature_rel("x.feature"), "x.feature");
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

        // NOT null-feature (TS throws / parses a feature): directive or content.
        assert!(!is_empty_or_comment_only("# language: en\n"));
        assert!(!is_empty_or_comment_only("# a\n# language: en\n"));
        assert!(!is_empty_or_comment_only("@foo\n"));
        assert!(!is_empty_or_comment_only("random text\n"));
        assert!(!is_empty_or_comment_only("Feature: X\n"));
    }

    #[test]
    fn language_directive_matches_cucumber_pattern() {
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
