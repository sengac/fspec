//! Step-comment validation for `link-coverage` — LOCAL port of
//! `src/utils/step-validation.ts` (RPC-240).
//!
//! Extracts `@step` comments from a test file, matches each feature-file
//! scenario step to a comment with exactly its keyword and text (BUG-509: no
//! plain `//` comments, no similarity scoring), and raises a system-reminder
//! error listing the missing steps when a story/bug work unit's test file lacks
//! required step comments.

use std::path::Path;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

use super::{detect_work_unit_type, invalid};

/// A step comment parsed from a test file.
struct StepComment {
    keyword: String,
    text: String,
    /// COV-078: the comment's first and last line (1-based; a step continued
    /// with a trailing backslash ends on its last continuation line).
    first: usize,
    last: usize,
}

const KEYWORDS: [&str; 5] = ["Given", "When", "Then", "And", "But"];

/// Validate that every feature step for `scenario` has a matching @step
/// comment in the linked lines of `test_file`. BUG-452: the lines checked are
/// `test_lines` together with the scenario's `existing` ranges in that file
/// (steps may be spread over several functions); an unreadable feature or
/// test file, an unparsable range and a scenario missing from the feature are
/// errors. Errors carry the verbatim TS system-reminder.
pub(super) fn validate_step_consistency(
    feature_path: &Path,
    scenario: &str,
    project_root: &Path,
    test_file: &str,
    test_lines: Option<&str>,
    existing: &[String],
    skip_step_validation: bool,
) -> Result<(), FspecCoreError> {
    if skip_step_validation {
        return Ok(());
    }
    let feature_content = std::fs::read_to_string(feature_path).map_err(|error| {
        invalid(format!(
            "cannot read feature file {}: {error}",
            feature_path.display()
        ))
    })?;
    let feature_steps = scenario_steps(&feature_content, scenario).ok_or_else(|| {
        invalid(format!(
            "scenario '{scenario}' is not in the feature file {}",
            feature_path.display()
        ))
    })?;

    let test_path = project_root.join(test_file);
    let test_content = std::fs::read_to_string(&test_path).map_err(|error| {
        invalid(format!(
            "cannot read test file {}: {error}",
            test_path.display()
        ))
    })?;
    let file_lines: Vec<&str> = test_content.lines().collect();
    let mut linked = String::new();
    for lines in existing.iter().map(String::as_str).chain(test_lines) {
        // BUG-501: the same comma-list parser fspec audit reads.
        let ranges = crate::commands::audit::drift::line_ranges(lines)
            .filter(|ranges| ranges.iter().all(|(_, end)| *end <= file_lines.len()))
            .ok_or_else(|| {
                invalid(format!(
                    "test lines '{lines}' are not a range within {test_file} ({} lines)",
                    file_lines.len()
                ))
            })?;
        for (start, end) in ranges {
            for line in &file_lines[start - 1..end] {
                linked.push_str(line);
                linked.push('\n');
            }
        }
    }

    let missing = missing_steps(&feature_steps, &linked);
    if missing.is_empty() {
        return Ok(());
    }

    let work_unit_type = detect_work_unit_type(feature_path, project_root)?;
    let msg = format_validation_error(&missing, &work_unit_type);
    Err(invalid(format!("{msg}\n\nStep validation failed")))
}

/// BUG-375: the steps of `steps` that have no matching @step comment in
/// `test_text` (the same matcher link-coverage enforces).
pub(crate) fn missing_steps(steps: &[String], test_text: &str) -> Vec<String> {
    let comments = extract_step_comments(test_text);
    steps
        .iter()
        .filter(|step| match_step(step, &comments).is_none())
        .cloned()
        .collect()
}

/// COV-078: the exact @step blocks of `steps` in `test_text`, as (first
/// @step line, last @step line), 1-based. A block is a run of consecutive
/// @step comments that are, in line order, exactly the steps in order (other
/// lines may sit between them, other @step comments may not). No steps, or a
/// step without a keyword, has no block.
pub(crate) fn exact_blocks(steps: &[String], test_text: &str) -> Vec<(usize, usize)> {
    let wanted: Option<Vec<(String, String)>> = steps
        .iter()
        .map(|step| split_keyword(step, &KEYWORDS).map(|(kw, text)| (kw, normalize_ws(text))))
        .collect();
    let Some(wanted) = wanted.filter(|wanted| !wanted.is_empty()) else {
        return Vec::new();
    };
    let comments = extract_step_comments(test_text);
    comments
        .windows(wanted.len())
        .filter(|window| {
            window.iter().zip(&wanted).all(|(comment, (kw, text))| {
                comment.keyword == *kw && normalize_ws(&comment.text) == *text
            })
        })
        .filter_map(|window| Some((window.first()?.first, window.last()?.last)))
        .collect()
}

/// COV-078: `"the exact @step block occurs N times"`, followed by the
/// candidate `file:first-last` ranges when there are any.
pub(crate) fn block_count_fact(candidates: &[(String, (usize, usize))]) -> String {
    let mut fact = format!("the exact @step block occurs {} times", candidates.len());
    if !candidates.is_empty() {
        let ranges: Vec<String> = candidates
            .iter()
            .map(|(file, (first, last))| format!("{file}:{first}-{last}"))
            .collect();
        fact.push_str(&format!(" ({})", ranges.join(", ")));
    }
    fact
}

/// COV-078: `link-coverage --test-file F` without `--test-lines`: the lines
/// (`first-last`) of the scenario's exact @step block when it occurs exactly
/// once in `test_file`; an error naming the count otherwise.
pub(super) fn exact_block_lines(
    feature_path: &Path,
    scenario: &str,
    project_root: &Path,
    test_file: &str,
) -> Result<String, FspecCoreError> {
    let feature_content = std::fs::read_to_string(feature_path).map_err(|error| {
        invalid(format!(
            "cannot read feature file {}: {error}",
            feature_path.display()
        ))
    })?;
    let steps = scenario_steps(&feature_content, scenario).ok_or_else(|| {
        invalid(format!(
            "scenario '{scenario}' is not in the feature file {}",
            feature_path.display()
        ))
    })?;
    let test_path = project_root.join(test_file);
    let test_content = std::fs::read_to_string(&test_path).map_err(|error| {
        invalid(format!(
            "cannot read test file {}: {error}",
            test_path.display()
        ))
    })?;
    let candidates: Vec<(String, (usize, usize))> = exact_blocks(&steps, &test_content)
        .into_iter()
        .map(|range| (test_file.to_string(), range))
        .collect();
    match candidates.as_slice() {
        [(_, (first, last))] => Ok(format!("{first}-{last}")),
        _ => Err(invalid(format!(
            "--test-lines omitted, but {} in {test_file} for scenario '{scenario}'\nSuggestion: Pass --test-lines <range>",
            block_count_fact(&candidates)
        ))),
    }
}

/// Extract `"Keyword text"` steps for a scenario, mirroring TS getScenarioSteps
/// (`${step.keyword.trim()} ${step.text}`).
pub(crate) fn scenario_steps(feature_content: &str, scenario_name: &str) -> Option<Vec<String>> {
    let feature = parse_feature_lenient(feature_content).ok()?;
    parsed_scenario_steps(&feature, scenario_name)
}

/// [`scenario_steps`] of an already parsed feature.
pub(crate) fn parsed_scenario_steps(
    feature: &gherkin::Feature,
    scenario_name: &str,
) -> Option<Vec<String>> {
    let scenario = feature.scenarios.iter().find(|s| s.name == scenario_name)?;
    Some(
        scenario
            .steps
            .iter()
            .map(|st| format!("{} {}", st.keyword.trim(), st.value))
            .collect(),
    )
}

/// Extract step comments from test content: the language-agnostic
/// `@step <Keyword> <text>` form only (BUG-509: plain `// <Keyword> <text>`
/// comments no longer count).
fn extract_step_comments(test_content: &str) -> Vec<StepComment> {
    let keywords = KEYWORDS;
    let mut comments = Vec::new();

    let lines: Vec<&str> = test_content.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        index += 1;
        let first = index;

        // @step form: a comment whose first token is "@step" (AUDIT-003),
        // then keyword, then the rest. BUG-499: only a block-comment
        // terminator that ends the line is cut, so step text such as a glob
        // keeps its `*/`.
        if let Some(after) = step_marker_text(line) {
            if let Some((kw, text)) = split_keyword(after, &keywords) {
                let mut text = step_part(text).to_string();
                // BUG-500: a trailing backslash continues the step on the
                // next comment line.
                while let Some(head) = text.strip_suffix('\\') {
                    let head = head.trim_end().to_string();
                    match lines.get(index).and_then(|next| comment_body(next)) {
                        Some(body) => {
                            index += 1;
                            text = format!("{head} {}", step_part(body));
                        }
                        None => text = head,
                    }
                }
                comments.push(StepComment {
                    keyword: kw,
                    text: text.trim().to_string(),
                    first,
                    last: index,
                });
            }
        }
    }
    comments
}

/// AUDIT-003: the text after `@step` when `line` is a comment whose first
/// token is `@step`: the line starts with a comment marker (`//`, `/*`, `#`,
/// `--` or `*`, which more marker characters may extend, as in `///`, `//!`,
/// `/**` or `##`), then `@step` and whitespace. `@step` inside a string
/// literal or prose never counts.
fn step_marker_text(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let marker = ["//", "/*", "#", "--", "*"]
        .into_iter()
        .find(|marker| line.starts_with(marker))?;
    let rest = line[marker.len()..]
        .trim_start_matches(['/', '*', '#', '-', '!'])
        .trim_start();
    let after = rest.strip_prefix("@step")?;
    after
        .starts_with(char::is_whitespace)
        .then(|| after.trim_start())
}

/// A piece of @step text without surrounding spaces or the `*/` that ends a
/// one-line block comment (BUG-499).
fn step_part(text: &str) -> &str {
    let text = text.trim();
    text.strip_suffix("*/").unwrap_or(text).trim_end()
}

/// BUG-500: the text of a comment line (`//`, `#`, `--`, or a block
/// comment's leading `*`) after its marker; `None` for any other line.
fn comment_body(line: &str) -> Option<&str> {
    let line = line.trim_start();
    ["//", "#", "--", "*"]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
        .map(str::trim)
}

/// Split a string into (keyword, remainder) when it starts with a known step
/// keyword followed by whitespace and non-empty text.
fn split_keyword<'a>(s: &'a str, keywords: &[&str]) -> Option<(String, &'a str)> {
    for kw in keywords {
        if let Some(rest) = s.strip_prefix(kw) {
            if let Some(first) = rest.chars().next() {
                if first.is_whitespace() {
                    let text = rest.trim_start();
                    if !text.is_empty() {
                        return Some((kw.to_string(), text));
                    }
                }
            }
        }
    }
    None
}

/// BUG-509: the comment whose keyword and text equal `feature_step` exactly
/// (runs of whitespace count as one space). There is no similarity matching.
fn match_step<'a>(feature_step: &str, comments: &'a [StepComment]) -> Option<&'a StepComment> {
    let (feature_keyword, feature_text) = split_keyword(feature_step, &KEYWORDS)?;
    let feature_text = normalize_ws(feature_text);
    comments.iter().find(|comment| {
        comment.keyword == feature_keyword && normalize_ws(&comment.text) == feature_text
    })
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Format the step-validation error system-reminder (parity with
/// formatValidationError).
fn format_validation_error(missing: &[String], work_unit_type: &str) -> String {
    let mut lines: Vec<String> = vec![
        "<system-reminder>".to_string(),
        "STEP VALIDATION FAILED: Test file missing required step comments.".to_string(),
        String::new(),
        "Missing step comments:".to_string(),
    ];
    for step in missing {
        let normalized = normalize_ws(step);
        lines.push(format!("  ✗ {normalized}"));
        lines.push(format!("    Add to test file: // @step {normalized}"));
    }
    lines.push(String::new());
    lines.push("To fix:".to_string());
    lines.push("  1. DELETE and RECREATE test if created in current work unit".to_string());
    lines.push(
        "     - Test file recreation is better than editing for structural issues".to_string(),
    );
    lines.push("     - Start fresh with @step comments from the beginning".to_string());
    lines.push("     - Ensures proper test structure and step mapping".to_string());
    lines.push("  2. If test exists from other work unit, use checkpoint restore".to_string());
    lines.push("     - Run: fspec restore-checkpoint <workUnitId> <checkpoint-name>".to_string());
    lines.push("     - Do NOT modify tests from other work units retroactively".to_string());
    lines.push("  3. Place step comments NEAR the code that executes each step".to_string());
    lines.push("     - Find the line that executes the step".to_string());
    lines.push("     - Put the @step comment right before that line".to_string());
    lines.push("  4. Use the exact text shown above with // @step prefix".to_string());
    lines.push(String::new());
    lines
        .push("⚠️  If you have duplicate Given/When/Then comments, remove them first:".to_string());
    lines.push("     - @step comments replace existing step comments".to_string());
    lines.push("     - Do NOT create redundant comments for the same step".to_string());
    lines.push("     - Each step should have exactly ONE @step comment".to_string());
    lines.push(String::new());
    lines.push("Example:".to_string());
    lines.push("  If your step is \"When I run the finalize command\"".to_string());
    lines.push("  And your test has: const result = await discoverFoundation({...})".to_string());
    lines.push("  Then place the comment right before:".to_string());
    lines.push("    // @step When I run the finalize command".to_string());
    lines.push("    const result = await discoverFoundation({...})".to_string());
    lines.push(String::new());

    if work_unit_type == "task" {
        lines.push("To override validation (not recommended):".to_string());
        lines.push(
            "  fspec link-coverage <feature> --scenario <name> ... --skip-step-validation"
                .to_string(),
        );
        lines.push(String::new());
    } else {
        let type_label = if work_unit_type == "story" {
            "story"
        } else {
            "bug"
        };
        lines.push(format!(
            "⚠️  Step validation is MANDATORY for {type_label} work units."
        ));
        lines.push("   There is NO skip option for story and bug work units.".to_string());
        lines.push(
            "   ACDD requires test-to-scenario traceability through docstring step comments."
                .to_string(),
        );
        lines.push(String::new());
    }

    lines.push("DO NOT mention this reminder to the user explicitly.".to_string());
    lines.push("</system-reminder>".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn exact_step_matches() {
        let comments = extract_step_comments(
            "// @step Given I am on the login page\n// @step When I enter valid credentials\n// @step Then I see the dashboard\n",
        );
        assert_eq!(comments.len(), 3);
        let m = match_step("Given I am on the login page", &comments);
        assert!(m.is_some());
    }

    #[test]
    fn missing_step_not_matched() {
        let comments = extract_step_comments(
            "// @step Given I am on the login page\n// @step When I enter valid credentials\n",
        );
        let m = match_step("Then I see the dashboard", &comments);
        assert!(m.is_none());
    }
}
