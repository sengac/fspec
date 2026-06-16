//! Step-comment validation for `link-coverage` — LOCAL port of
//! `src/utils/step-validation.ts` + `src/utils/similarity-algorithms.ts`
//! (RPC-240).
//!
//! Extracts `@step` (and plain `//`) step comments from a test file, matches
//! each feature-file scenario step to a comment via a weighted hybrid
//! similarity score with adaptive thresholds, and raises a system-reminder
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
}

/// Validate that every feature step for `scenario` has a matching @step comment
/// in `test_file`. No-op when `skip_step_validation` or the feature file is
/// missing. Errors carry the verbatim TS system-reminder.
pub(super) fn validate_step_consistency(
    feature_path: &Path,
    scenario: &str,
    project_root: &Path,
    test_file: &str,
    skip_step_validation: bool,
) -> Result<(), FspecCoreError> {
    if skip_step_validation {
        return Ok(());
    }
    // If the feature file doesn't exist, skip step validation silently
    // (forward planning).
    let feature_content = match std::fs::read_to_string(feature_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let feature_steps = match scenario_steps(&feature_content, scenario) {
        Some(steps) => steps,
        None => return Ok(()),
    };

    let test_path = project_root.join(test_file);
    let test_content = std::fs::read_to_string(&test_path).unwrap_or_default();

    let comments = extract_step_comments(&test_content);
    let missing: Vec<String> = feature_steps
        .iter()
        .filter(|fs| match_step(fs, &comments).is_none())
        .cloned()
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    let work_unit_type = detect_work_unit_type(feature_path, project_root);
    let msg = format_validation_error(&missing, &work_unit_type);
    Err(invalid(format!("{msg}\n\nStep validation failed")))
}

/// Extract `"Keyword text"` steps for a scenario, mirroring TS getScenarioSteps
/// (`${step.keyword.trim()} ${step.text}`).
fn scenario_steps(feature_content: &str, scenario_name: &str) -> Option<Vec<String>> {
    let feature = parse_feature_lenient(feature_content).ok()?;
    let scenario = feature.scenarios.iter().find(|s| s.name == scenario_name)?;
    Some(
        scenario
            .steps
            .iter()
            .map(|st| format!("{} {}", st.keyword.trim(), st.value))
            .collect(),
    )
}

/// Extract step comments from test content. Recognizes the language-agnostic
/// `@step <Keyword> <text>` form and the plain JS `// <Keyword> <text>` form.
fn extract_step_comments(test_content: &str) -> Vec<StepComment> {
    let keywords = ["Given", "When", "Then", "And", "But"];
    let mut comments = Vec::new();

    for raw in test_content.lines() {
        let line = raw.trim();

        // @step form: find "@step", then keyword, then the rest (trimming a
        // trailing block-comment terminator).
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
                        return Some((kw.to_string(), text));
                    }
                }
            }
        }
    }
    None
}

/// Find the best-matching comment for `feature_step`. Keyword must match; the
/// hybrid similarity must meet the adaptive threshold.
fn match_step<'a>(feature_step: &str, comments: &'a [StepComment]) -> Option<&'a StepComment> {
    let (feature_keyword, feature_text) = match split_keyword(feature_step, &["Given", "When", "Then", "And", "But"]) {
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
    lines.push("     - Test file recreation is better than editing for structural issues".to_string());
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
    lines.push("⚠️  If you have duplicate Given/When/Then comments, remove them first:".to_string());
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
        lines.push("  fspec link-coverage <feature> --scenario <name> ... --skip-step-validation".to_string());
        lines.push(String::new());
    } else {
        let type_label = if work_unit_type == "story" { "story" } else { "bug" };
        lines.push(format!("⚠️  Step validation is MANDATORY for {type_label} work units."));
        lines.push("   There is NO skip option for story and bug work units.".to_string());
        lines.push("   ACDD requires test-to-scenario traceability through docstring step comments.".to_string());
        lines.push(String::new());
    }

    lines.push("DO NOT mention this reminder to the user explicitly.".to_string());
    lines.push("</system-reminder>".to_string());
    lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────
// Similarity algorithms (port of similarity-algorithms.ts)
// ─────────────────────────────────────────────────────────────────────────
//
// `match_step` compares single step texts (one "step" string). We model each
// side as a Scenario { name: text, steps: [text] } to mirror the TS caller
// (matchStep builds `{ name, steps: [name] }`).

struct SimWeights {
    jaro_winkler: f64,
    token_set: f64,
    gherkin_structural: f64,
    trigram: f64,
    jaccard: f64,
}

const DEFAULT_WEIGHTS: SimWeights = SimWeights {
    jaro_winkler: 0.3,
    token_set: 0.25,
    gherkin_structural: 0.2,
    trigram: 0.15,
    jaccard: 0.1,
};

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
    let title_len = text1.trim().chars().count().min(text2.trim().chars().count());
    let w = if title_len < 20 { &SHORT_WEIGHTS } else { &DEFAULT_WEIGHTS };

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

fn token_set_ratio(text1: &str, text2: &str) -> f64 {
    use std::collections::HashSet;
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

fn trigram_similarity(s1: &str, s2: &str) -> f64 {
    use std::collections::HashSet;
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

fn jaccard_similarity(text1: &str, text2: &str) -> f64 {
    use std::collections::HashSet;
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
/// single category. Mirrors gherkinStructuralSimilarity collapsed to one step.
fn gherkin_structural(text1: &str, text2: &str) -> f64 {
    // With keyword stripped, both texts are treated as the same (then) bucket
    // weighting; effectively a jaccard over the lowercased step text.
    use std::collections::HashSet;
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

    #[test]
    fn hybrid_identical_is_one() {
        assert!((hybrid_similarity("i am on the login page", "i am on the login page") - 1.0).abs() < 1e-9);
    }
}
