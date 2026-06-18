//! Step-docstring validation for `update-work-unit-status` — port of
//! `validateTestStepDocstrings` (`src/commands/update-work-unit-status.ts:863-1012`),
//! BUG-061 / BUG-093.
//!
//! Invoked when transitioning a non-task work unit to `implementing` or
//! `validating`. For every feature file whose first `@<PREFIX>-<N>` tag equals
//! the work unit id, it enforces:
//!
//! 1. A coverage file (`<feature>.coverage`) MUST exist
//!    (`update-work-unit-status.ts:889-897`).
//! 2. The coverage file MUST be valid JSON (`:902-912`).
//! 3. At least one test mapping MUST be linked (`:928-935`).
//! 4. At most ONE distinct test file per feature — 1:1 mapping (`:937-950`).
//! 5. Each linked test file MUST be non-empty (`:961-966`) and contain a
//!    matching `@step` comment for every scenario step (`:969-991`).
//!
//! The similarity matcher (Jaro-Winkler / Token-Set / Trigram / Jaccard /
//! Gherkin-structural hybrid) and the `@step` extractor are ported as a
//! PRIVATE module here, consistent with the existing per-command duplication
//! convention in this codebase (`commands/generate_scenarios.rs::similarity`
//! and `commands/link_coverage/step.rs`). Weights, configs, and adaptive
//! thresholds are reproduced EXACTLY — do not tune them.

use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

use super::msg;

/// A feature file matched to a work unit (relative path + scenarios).
struct MatchedFeature {
    /// Path relative to the project root, e.g. `spec/features/login.feature`.
    rel_path: String,
    /// Scenarios with their fully-rendered `"Keyword text"` steps.
    scenarios: Vec<MatchedScenario>,
}

/// A scenario plus its rendered step strings.
struct MatchedScenario {
    name: String,
    steps: Vec<String>,
}

/// Validate that the work unit's tagged feature files have coverage files whose
/// linked test files contain a complete set of `@step` comments. Mirrors
/// `validateTestStepDocstrings`. No-op when no feature file carries the
/// `@<id>` tag (parity with the empty `matchingFeatures` early return).
pub(super) fn validate_test_step_docstrings(
    project_root: &Path,
    id: &str,
    work_type: &str,
) -> Result<(), FspecCoreError> {
    let matching = matching_features(project_root, id);
    if matching.is_empty() {
        // No feature files found — OK, might be using linkedFeatures array.
        return Ok(());
    }

    let mut validation_errors: Vec<String> = Vec::new();

    for feature in &matching {
        // BUG-093: per-feature test file tracking for 1:1 validation.
        let coverage_path = project_root.join(format!("{}.coverage", feature.rel_path));

        if !coverage_path.exists() {
            return Err(msg(format!(
                "No coverage file found for feature {}.\n\n\
Coverage file expected at: {}.coverage\n\n\
Test files must be linked using coverage files.\n\
Use: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --test-lines <range>\n\n\
Before moving to implementing or validating, tests must be written and linked.",
                feature.rel_path, feature.rel_path
            )));
        }

        let coverage_content =
            std::fs::read_to_string(&coverage_path).map_err(|source| FspecCoreError::Io {
                command: "update-work-unit-status",
                source,
            })?;
        let coverage: Value = match serde_json::from_str(&coverage_content) {
            Ok(v) => v,
            Err(e) => {
                return Err(msg(format!(
                    "Malformed coverage file: {}\n\
JSON parse error: {e}\n\n\
Please check the coverage file for syntax errors.",
                    coverage_path.display()
                )));
            }
        };

        // Collect distinct linked test files (insertion order preserved).
        let mut feature_test_files: Vec<String> = Vec::new();
        if let Some(scenarios) = coverage.get("scenarios").and_then(Value::as_array) {
            for scenario in scenarios {
                if let Some(mappings) = scenario.get("testMappings").and_then(Value::as_array) {
                    for mapping in mappings {
                        if let Some(file) = mapping.get("file").and_then(Value::as_str) {
                            if !feature_test_files.iter().any(|f| f == file) {
                                feature_test_files.push(file.to_string());
                            }
                        }
                    }
                }
            }
        }

        // VAL-005: no test files linked to this feature.
        if feature_test_files.is_empty() {
            return Err(msg(format!(
                "No test files linked to feature {}.\n\n\
Coverage file exists but contains no test mappings for this feature.\n\
Use: fspec link-coverage <feature> --scenario \"<name>\" --test-file <path> --test-lines <range>\n\n\
Before moving to implementing or validating, tests must be written and linked.",
                feature.rel_path
            )));
        }

        // VAL-005: enforce 1:1 mapping (1 feature file = 1 test file).
        if feature_test_files.len() > 1 {
            let test_file_paths = feature_test_files.join(", ");
            return Err(msg(format!(
                "Multiple test files detected for feature file.\n\n\
Feature: {}\n\
Test files ({}): {}\n\n\
Design intent: 1 feature file = 1 test file (1:1 mapping)\n\n\
Split feature file into multiple smaller features, each with its own test file.\n\n\
Example:\n  - keyboard-navigation.feature → useKeyboardNavigation.test.ts\n  - layout-selector.feature → LayoutSelector.test.tsx",
                feature.rel_path,
                feature_test_files.len(),
                test_file_paths
            )));
        }

        // Step validation per feature: validate THIS feature's test file
        // against THIS feature's scenarios.
        for test_file_path in &feature_test_files {
            let absolute_test_path = project_root.join(test_file_path);
            let test_content = match std::fs::read_to_string(&absolute_test_path) {
                Ok(c) => c,
                Err(e) => {
                    return Err(msg(format!(
                        "Failed to read test file {test_file_path}: {e}"
                    )));
                }
            };

            if test_content.trim().is_empty() {
                // The TS empty-file `throw` happens INSIDE the try block, so it
                // is re-wrapped by the surrounding catch as
                // `Failed to read test file <p>: <message>`
                // (update-work-unit-status.ts:961-998).
                return Err(msg(format!(
                    "Failed to read test file {test_file_path}: Test file is empty: {test_file_path}\n\n\
Test files must contain step docstrings matching feature scenarios."
                )));
            }

            for scenario in &feature.scenarios {
                if scenario.steps.is_empty() {
                    continue;
                }
                let result = validate_steps(&scenario.steps, &test_content);
                if !result.valid {
                    let error_message = format_validation_error(&result.missing_steps, work_type);
                    validation_errors.push(format!(
                        "Test file: {test_file_path}\nScenario: {}\n{error_message}",
                        scenario.name
                    ));
                }
            }
        }
    }

    if !validation_errors.is_empty() {
        let summary = if validation_errors.len() == 1 {
            validation_errors[0].clone()
        } else {
            format!(
                "Multiple validation errors found ({} scenarios):\n\n{}",
                validation_errors.len(),
                validation_errors.join("\n\n---\n\n")
            )
        };
        return Err(msg(summary));
    }

    Ok(())
}

/// Enumerate TOP-LEVEL `spec/features/*.feature` files (parity with the TS
/// `parseAllFeatures` glob `*.feature`, which is non-recursive) and return the
/// ones whose first `@<PREFIX>-<N>` feature tag equals `id`.
fn matching_features(project_root: &Path, id: &str) -> Vec<MatchedFeature> {
    let features_dir = project_root.join("spec").join("features");
    let mut out: Vec<MatchedFeature> = Vec::new();

    let Ok(entries) = std::fs::read_dir(&features_dir) else {
        return out;
    };
    let mut files: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("feature"))
        .collect();
    files.sort();

    for path in files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(feature) = parse_feature_lenient(&content) else {
            continue;
        };
        // Extract work unit id from the first tag matching `^@?[A-Z]+-\d+$`.
        let Some(work_unit_id) = feature
            .tags
            .iter()
            .map(|t| t.trim_start_matches('@'))
            .find(|t| is_work_unit_tag(t))
        else {
            continue;
        };
        if work_unit_id != id {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let rel_path = format!("spec/features/{file_name}");
        let scenarios = feature
            .scenarios
            .iter()
            .map(|s| MatchedScenario {
                name: s.name.clone(),
                steps: s
                    .steps
                    .iter()
                    .map(|st| format!("{} {}", st.keyword.trim(), st.value))
                    .collect(),
            })
            .collect();
        out.push(MatchedFeature { rel_path, scenarios });
    }
    out
}

/// Whether `tag` (already `@`-stripped) matches `[A-Z]+-\d+` (e.g. `AUTH-001`).
fn is_work_unit_tag(tag: &str) -> bool {
    let Some((prefix, num)) = tag.split_once('-') else {
        return false;
    };
    !prefix.is_empty()
        && prefix.chars().all(|c| c.is_ascii_uppercase())
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit())
}

// ─────────────────────────────────────────────────────────────────────────
// Step-comment validation (port of src/utils/step-validation.ts)
// ─────────────────────────────────────────────────────────────────────────

/// A step comment parsed from a test file.
struct StepComment {
    keyword: String,
    text: String,
}

/// Result of [`validate_steps`].
struct StepValidationResult {
    valid: bool,
    missing_steps: Vec<String>,
}

/// Validate that all feature steps have a matching `@step` comment in the test
/// content (mirrors `validateSteps`).
fn validate_steps(feature_steps: &[String], test_content: &str) -> StepValidationResult {
    let comments = extract_step_comments(test_content);
    let missing_steps: Vec<String> = feature_steps
        .iter()
        .filter(|fs| match_step(fs, &comments).is_none())
        .cloned()
        .collect();
    StepValidationResult {
        valid: missing_steps.is_empty(),
        missing_steps,
    }
}

/// Extract step comments from test content. Recognizes the language-agnostic
/// `@step <Keyword> <text>` form and the plain JS `// <Keyword> <text>` form
/// (mirrors `extractStepComments`).
fn extract_step_comments(test_content: &str) -> Vec<StepComment> {
    let keywords = ["Given", "When", "Then", "And", "But"];
    let mut comments = Vec::new();

    for raw in test_content.lines() {
        let line = raw.trim();

        // `@step` form: find "@step", then keyword, then the rest (trimming a
        // trailing block-comment terminator `*/`).
        if let Some(pos) = line.find("@step") {
            let after = line[pos + "@step".len()..].trim_start();
            if let Some((kw, text)) = split_keyword(after, &keywords) {
                let mut text = text.to_string();
                if let Some(idx) = text.find("*/") {
                    text = text[..idx].to_string();
                }
                comments.push(StepComment {
                    keyword: kw,
                    text: text.trim().to_string(),
                });
                continue;
            }
        }

        // Plain form: `// <Keyword> <text>`.
        if let Some(rest) = line.strip_prefix("//") {
            let rest = rest.trim_start();
            if let Some((kw, text)) = split_keyword(rest, &keywords) {
                comments.push(StepComment {
                    keyword: kw,
                    text: text.trim().to_string(),
                });
            }
        }
    }
    comments
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
                        return Some(((*kw).to_string(), text));
                    }
                }
            }
        }
    }
    None
}

/// Find the best-matching comment for `feature_step` (mirrors `matchStep`).
/// Keyword must match; the hybrid similarity must meet the adaptive threshold.
fn match_step<'a>(feature_step: &str, comments: &'a [StepComment]) -> Option<&'a StepComment> {
    let keywords = ["Given", "When", "Then", "And", "But"];
    let (feature_keyword, feature_text) = match split_keyword(feature_step, &keywords) {
        Some((k, t)) => (k, normalize_ws(t)),
        None => return None,
    };
    let threshold = adaptive_threshold(&feature_text);

    let mut best: Option<(&StepComment, f64)> = None;
    for comment in comments {
        if comment.keyword != feature_keyword {
            continue;
        }
        let comment_text = normalize_ws(&comment.text);
        let score = hybrid_similarity(&feature_text, &comment_text);
        if score >= threshold && best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((comment, score));
        }
    }
    best.map(|(c, _)| c)
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn adaptive_threshold(text: &str) -> f64 {
    let len = text.chars().count();
    if len < 10 {
        0.85
    } else if len < 20 {
        0.80
    } else if len < 40 {
        0.75
    } else {
        0.70
    }
}

/// Format the step-validation error system-reminder (mirrors
/// `formatValidationError`).
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
    lines.push("     - Run: fspec restore-checkpoint <work-unit-id> <checkpoint-name>".to_string());
    lines.push("     - Do NOT modify tests from other work units retroactively".to_string());
    lines.push("  3. Place step comments NEAR the code that executes each step".to_string());
    lines.push("     - Find the line that executes the step".to_string());
    lines.push("     - Put the @step comment right before that line".to_string());
    lines.push("  4. Use the exact text shown above with // @step prefix".to_string());
    lines.push(String::new());
    lines.push(
        "⚠️  If you have duplicate Given/When/Then comments, remove them first:".to_string(),
    );
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
        let type_label = if work_unit_type == "story" { "story" } else { "bug" };
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

// ─────────────────────────────────────────────────────────────────────────
// Similarity algorithms (port of src/utils/similarity-algorithms.ts)
// ─────────────────────────────────────────────────────────────────────────
//
// `match_step` compares single step texts. Each side is modelled as a
// Scenario { name: text, steps: [text] } to mirror the TS `matchStep`, which
// builds `{ name, steps: [name] }`. Weights/configs match the TS source.

/// Hybrid algorithm weights (must sum to 1.0).
struct SimWeights {
    jaro_winkler: f64,
    token_set: f64,
    gherkin_structural: f64,
    trigram: f64,
    jaccard: f64,
}

/// `DEFAULT_SIMILARITY_CONFIG` (strings >= 20 chars).
const DEFAULT_WEIGHTS: SimWeights = SimWeights {
    jaro_winkler: 0.3,
    token_set: 0.25,
    gherkin_structural: 0.2,
    trigram: 0.15,
    jaccard: 0.1,
};

/// `SHORT_STRING_CONFIG` (strings < 20 chars).
const SHORT_WEIGHTS: SimWeights = SimWeights {
    jaro_winkler: 0.15,
    token_set: 0.35,
    gherkin_structural: 0.2,
    trigram: 0.1,
    jaccard: 0.2,
};

/// Hybrid similarity between two single-step texts (each treated as a scenario
/// whose name and sole step is the text). Auto-selects short-string weights.
fn hybrid_similarity(text1: &str, text2: &str) -> f64 {
    let title_len = text1
        .trim()
        .chars()
        .count()
        .min(text2.trim().chars().count());
    let w = if title_len < 20 {
        &SHORT_WEIGHTS
    } else {
        &DEFAULT_WEIGHTS
    };

    let jw = jaro_winkler(text1, text2);
    let ts = token_set_ratio(text1, text2);
    let gs = gherkin_structural(text1, text2);
    // Trigram on combined text "name steps" — here name == steps == text.
    let combined1 = format!("{text1} {text1}");
    let combined2 = format!("{text2} {text2}");
    let tg = trigram_similarity(&combined1, &combined2);
    let jc = jaccard_similarity(text1, text2);

    jw * w.jaro_winkler
        + ts * w.token_set
        + gs * w.gherkin_structural
        + tg * w.trigram
        + jc * w.jaccard
}

/// Jaro-Winkler similarity.
fn jaro_winkler(s1: &str, s2: &str) -> f64 {
    let a: Vec<char> = s1.to_lowercase().chars().collect();
    let b: Vec<char> = s2.to_lowercase().chars().collect();
    if a == b {
        return 1.0;
    }
    let (len1, len2) = (a.len(), b.len());
    if len1 == 0 && len2 == 0 {
        return 1.0;
    }
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }
    let match_distance = (len1.max(len2) / 2).saturating_sub(1) as isize;
    let mut a_matches = vec![false; len1];
    let mut b_matches = vec![false; len2];
    let mut matches = 0usize;

    for i in 0..len1 {
        let start = (i as isize - match_distance).max(0) as usize;
        let end = ((i as isize + match_distance + 1) as usize).min(len2);
        for j in start..end {
            if b_matches[j] || a[i] != b[j] {
                continue;
            }
            a_matches[i] = true;
            b_matches[j] = true;
            matches += 1;
            break;
        }
    }
    if matches == 0 {
        return 0.0;
    }
    let mut transpositions = 0usize;
    let mut k = 0usize;
    for i in 0..len1 {
        if !a_matches[i] {
            continue;
        }
        while !b_matches[k] {
            k += 1;
        }
        if a[i] != b[k] {
            transpositions += 1;
        }
        k += 1;
    }
    let m = matches as f64;
    let jaro = (m / len1 as f64 + m / len2 as f64 + (m - transpositions as f64 / 2.0) / m) / 3.0;
    let mut prefix = 0usize;
    for i in 0..len1.min(len2).min(4) {
        if a[i] == b[i] {
            prefix += 1;
        } else {
            break;
        }
    }
    jaro + prefix as f64 * 0.1 * (1.0 - jaro)
}

/// Remove standalone Gherkin keywords from a lowercased copy of `text`.
fn strip_keywords(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut out = String::new();
    for word in lower.split_whitespace() {
        if matches!(word, "given" | "when" | "then" | "and" | "but") {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// Token Set Ratio.
fn token_set_ratio(text1: &str, text2: &str) -> f64 {
    let t1 = strip_keywords(text1);
    let t2 = strip_keywords(text2);
    let tokens1: HashSet<&str> = t1.split_whitespace().filter(|t| !t.is_empty()).collect();
    let tokens2: HashSet<&str> = t2.split_whitespace().filter(|t| !t.is_empty()).collect();
    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }
    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }
    let intersection = tokens1.intersection(&tokens2).count();
    let diff1 = tokens1.difference(&tokens2).count();
    let diff2 = tokens2.difference(&tokens1).count();
    if diff1 == 0 && diff2 == 0 {
        return 1.0;
    }
    let union = tokens1.len() + tokens2.len() - intersection;
    intersection as f64 / union as f64
}

/// Trigram similarity with padding.
fn trigram_similarity(s1: &str, s2: &str) -> f64 {
    let a = s1.to_lowercase();
    let b = s2.to_lowercase();
    if a == b {
        return 1.0;
    }
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let extract = |text: &str| -> HashSet<String> {
        let padded: Vec<char> = format!("  {text}  ").chars().collect();
        let mut set = HashSet::new();
        if padded.len() >= 3 {
            for i in 0..padded.len() - 2 {
                set.insert(padded[i..i + 3].iter().collect::<String>());
            }
        }
        set
    };
    let set1 = extract(&a);
    let set2 = extract(&b);
    if set1.is_empty() && set2.is_empty() {
        return 1.0;
    }
    let intersection = set1.intersection(&set2).count();
    (2.0 * intersection as f64) / (set1.len() + set2.len()) as f64
}

/// Jaccard similarity over raw alphanumeric tokens.
fn jaccard_similarity(text1: &str, text2: &str) -> f64 {
    let extract = |text: &str| -> HashSet<String> {
        let stripped = strip_keywords(text);
        stripped
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
    let tokens1 = extract(text1);
    let tokens2 = extract(text2);
    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }
    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }
    let intersection = tokens1.intersection(&tokens2).count();
    let union = tokens1.union(&tokens2).count();
    intersection as f64 / union as f64
}

/// Single-step gherkin structural similarity. Both sides are bare step texts
/// (the keyword was already split off by `match_step`), so each parses to a
/// single category — effectively a Jaccard over the lowercased step text.
fn gherkin_structural(text1: &str, text2: &str) -> f64 {
    let a: HashSet<String> = std::iter::once(text1.to_lowercase()).collect();
    let b: HashSet<String> = std::iter::once(text2.to_lowercase()).collect();
    if a == b {
        return 1.0;
    }
    let intersection = a.intersection(&b).count();
    let union = a.union(&b).count();
    if union == 0 {
        return 1.0;
    }
    intersection as f64 / union as f64
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
        assert!(match_step("Given I am on the login page", &comments).is_some());
    }

    #[test]
    fn missing_step_not_matched() {
        let comments = extract_step_comments(
            "// @step Given I am on the login page\n// @step When I enter valid credentials\n",
        );
        assert!(match_step("Then I see the dashboard", &comments).is_none());
    }

    #[test]
    fn validate_steps_reports_missing() {
        let steps = vec![
            "Given I am on the login page".to_string(),
            "When I enter valid credentials".to_string(),
            "Then I should see the dashboard".to_string(),
        ];
        let test = "// @step Given I am on the login page\n// @step When I enter valid credentials\n";
        let result = validate_steps(&steps, test);
        assert!(!result.valid);
        assert_eq!(result.missing_steps.len(), 1);
    }

    #[test]
    fn work_unit_tag_detection() {
        assert!(is_work_unit_tag("AUTH-001"));
        assert!(is_work_unit_tag("BUG-42"));
        assert!(!is_work_unit_tag("wip"));
        assert!(!is_work_unit_tag("auth-001"));
        assert!(!is_work_unit_tag("AUTH-"));
    }
}
