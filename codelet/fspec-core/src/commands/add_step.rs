//! `add-step` — Rust port of `src/commands/add-step.ts` (RPC-192).
//!
//! Adds a Gherkin step to a named scenario via line-based editing. Mirrors
//! the TS reference exactly:
//!
//!   - validates the step type against `given|when|then|and|but`
//!     (case-insensitive), capitalising the keyword for output;
//!   - resolves a bare identifier to `spec/features/<id>.feature`;
//!   - returns soft-failure envelopes (`success:false`) for invalid type,
//!     missing file, invalid Gherkin, or unknown scenario (with an
//!     available-scenarios suggestion);
//!   - if a matching placeholder step (`[precondition]`/`[action]`/
//!     `[expected outcome]`) exists in the target scenario, REPLACES it
//!     in place; otherwise APPENDS the new step after the last existing
//!     step (before any trailing data-table / doc-string), inheriting the
//!     indentation of the first existing step;
//!   - honours `dryRun`.
//!
//! ## Two-front-doors
//! Both the dispatcher AND the clap subcommand call this single function.
//! The CLI bridge at `codelet/fspec/src/add_step.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

const VALID_STEP_TYPES: &[&str] = &["given", "when", "then", "and", "but"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddStepArgs {
    feature: String,
    scenario: String,
    #[serde(rename = "type")]
    step_type: String,
    text: String,
    #[serde(default)]
    dry_run: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddStepArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-step",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Normalise and validate step type ----
    let normalized = args.step_type.to_lowercase();
    if !VALID_STEP_TYPES.contains(&normalized.as_str()) {
        return soft_fail(json!({
            "success": false,
            "valid": false,
            "error": format!("Invalid step type: \"{}\"", args.step_type),
            "suggestion": format!("Valid step types are: {}", VALID_STEP_TYPES.join(", ")),
        }));
    }

    // Capitalise keyword: first char upper, rest as-is.
    let step_keyword = capitalize(&normalized);

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
                command: "add-step",
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
    // fix in `update_step.rs`). A `# language: <lang>` directive with no
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

    // ---- Validate existing Gherkin ----
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            // PARITY NOTE: see add_scenario.rs — the TS reference embeds
            // `@cucumber/gherkin`'s multi-line `Parser errors:` message,
            // whose exact TEXT the Rust `gherkin-0.16.0` crate cannot
            // reproduce byte-for-byte. Surface the crate's `{e}` (consistent
            // with sibling Gherkin commands) instead of a hard-coded
            // placeholder.
            return soft_fail(json!({
                "success": false,
                "valid": false,
                "error": format!("Feature file has invalid Gherkin syntax: {e}"),
                "suggestion": format!("Run 'fspec validate {}' to see syntax errors", args.feature),
            }));
        }
    };
    if feature.keyword.is_empty() && feature.name.is_empty() && feature.scenarios.is_empty() {
        return soft_fail(json!({
            "success": false,
            "valid": false,
            "error": "Feature file does not contain a valid Feature",
            "suggestion": format!("Run 'fspec validate {}' to see syntax errors", args.feature),
        }));
    }

    // ---- Find the target scenario ----
    // TS filters children to `scenario.keyword === 'Scenario'` (plain
    // scenarios only) before searching — Scenario Outlines are excluded
    // both from the lookup and the "Available scenarios" suggestion.
    let plain_scenarios: Vec<&gherkin::Scenario> = feature
        .scenarios
        .iter()
        .filter(|s| s.keyword == "Scenario")
        .collect();
    let target = plain_scenarios.iter().find(|s| s.name == args.scenario);
    let target = match target {
        Some(s) => *s,
        None => {
            let available: Vec<&str> = plain_scenarios.iter().map(|s| s.name.as_str()).collect();
            let available_str = if available.is_empty() {
                "none".to_string()
            } else {
                available.join(", ")
            };
            return soft_fail(json!({
                "success": false,
                "valid": false,
                "error": format!("Scenario not found: \"{}\"", args.scenario),
                "suggestion": format!("Available scenarios: {}", available_str),
            }));
        }
    };

    let scenario_line_index = target.position.line.saturating_sub(1);
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

    // ---- Determine indentation from the first existing step ----
    let mut step_indentation = "    ".to_string();
    if let Some(first_step) = target.steps.first() {
        let idx = first_step.position.line.saturating_sub(1);
        if let Some(line) = lines.get(idx) {
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            if indent_len > 0 {
                step_indentation = line[..indent_len].to_string();
            }
        }
    }

    // ---- Placeholder replacement search ----
    // Map step type → placeholder text (given/when/then only).
    let placeholder_text = match normalized.as_str() {
        "given" => Some("[precondition]"),
        "when" => Some("[action]"),
        "then" => Some("[expected outcome]"),
        _ => None,
    };

    let mut placeholder_line_index: Option<usize> = None;
    if let Some(ph) = placeholder_text {
        for step in &target.steps {
            if step.value == ph {
                placeholder_line_index = Some(step.position.line.saturating_sub(1));
                break;
            }
        }
    }

    let new_step = format!("{step_indentation}{step_keyword} {}", args.text);

    let new_content = if let Some(ph_idx) = placeholder_line_index {
        // Replace the placeholder line in place.
        if ph_idx < lines.len() {
            lines[ph_idx] = new_step;
        }
        lines.join("\n")
    } else {
        // Append after the last existing step (before a trailing data table /
        // doc string), else right after the scenario line.
        let insert_index = compute_append_index(&lines, target, scenario_line_index);
        lines.insert(insert_index, new_step);
        lines.join("\n")
    };

    // ---- Validate rewritten content ----
    let valid = parse_feature_lenient(&new_content).is_ok();

    // ---- Write unless dry run ----
    if !args.dry_run {
        std::fs::write(&feature_abs, &new_content).map_err(|source| FspecCoreError::Io {
            command: "add-step",
            source,
        })?;
    }

    soft_fail(json!({
        "success": true,
        "valid": valid,
    }))
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Compute the line index (0-based) at which a NEW step should be inserted
/// when there is no matching placeholder. Mirrors TS
/// `src/commands/add-step.ts:174-214`.
fn compute_append_index(
    lines: &[String],
    target: &gherkin::Scenario,
    scenario_line_index: usize,
) -> usize {
    if let Some(last_step) = target.steps.last() {
        let last_step_line_index = last_step.position.line.saturating_sub(1);
        // Default: right after the last step keyword line.
        let mut insert_index = last_step.position.line;
        // If the next line is a data table (`|`) or doc string (`"""`),
        // insert BEFORE it (i.e. right after the step keyword line).
        if last_step_line_index + 1 < lines.len() {
            let next = lines[last_step_line_index + 1].trim();
            if next.starts_with('|') || next.starts_with("\"\"\"") {
                insert_index = last_step_line_index + 1;
            }
        }
        insert_index
    } else {
        // No existing steps — scan forward from the scenario line.
        let mut insert_index = scenario_line_index + 1;
        let mut i = scenario_line_index + 1;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("Scenario:")
                || trimmed.starts_with("Scenario Outline:")
                || trimmed.is_empty()
            {
                insert_index = i;
                break;
            }
            insert_index = i + 1;
            i += 1;
        }
        insert_index
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Mirror of TS feature-path resolution (`src/commands/add-step.ts:47-55`).
/// The two verbatim-return branches are collapsed into one condition
/// (clippy `if_same_then_else`) — behaviour is identical to the TS.
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
        // parser (or the scenario lookup) governs the outcome, not this
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

/// A soft failure / success: the inner envelope is returned as a successful
/// dispatch result (the TS command returns the result object).
fn soft_fail(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-step",
        reason: format!("failed to serialise response: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn capitalize_keyword() {
        assert_eq!(capitalize("given"), "Given");
        assert_eq!(capitalize("and"), "And");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn resolve_paths() {
        assert_eq!(resolve_feature_rel("login"), "spec/features/login.feature");
        assert_eq!(
            resolve_feature_rel("spec/features/login.feature"),
            "spec/features/login.feature"
        );
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
