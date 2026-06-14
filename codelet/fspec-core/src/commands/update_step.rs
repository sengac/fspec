//! `update-step` — Rust port of `src/commands/update-step.ts` (RPC-315).
//!
//! Updates the text and/or keyword of a single step in a scenario via a
//! line-based edit: parse the feature to locate the scenario and matching
//! step, then rewrite that source line in place (preserving indentation),
//! re-join and write.
//!
//! Mirrors the TypeScript reference byte-for-byte including:
//!   - the "at least one of --text/--keyword" gate
//!     (`No updates specified. Use --text and/or --keyword`);
//!   - feature-path resolution (`.feature` suffix, `spec/features/` prefix,
//!     bare name → `spec/features/<name>.feature`);
//!   - canonical soft-failure envelopes (`{success:false,error}` returned as
//!     `Ok(json)` so the dispatcher surfaces them as data);
//!   - step matching by text alone, by `keyword+text`, or by
//!     `keyword.trim()+" "+text` (TS src/commands/update-step.ts:104-112);
//!   - `--text` keyword-prefix stripping: a `--text "When I submit"` keeps
//!     only the `I submit` portion (keyword comes from `--keyword`/existing);
//!   - error texts: `Feature file not found: <path>`,
//!     `Scenario '<s>' not found in feature file`,
//!     `Step '<step>' not found in scenario '<s>'`.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `codelet/fspec/src/update_step.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateStepArgs {
    feature: String,
    scenario: String,
    current_step: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    keyword: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateStepArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-step",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- At least one update must be specified (TS:36-42) ----
    let text = non_empty(&args.text);
    let keyword = non_empty(&args.keyword);
    if text.is_none() && keyword.is_none() {
        return err_envelope("No updates specified. Use --text and/or --keyword".to_string());
    }

    // ---- Resolve feature file path ----
    let feature_rel = resolve_feature_rel(&args.feature);
    let feature_abs = project_root.join(&feature_rel);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&feature_abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return err_envelope(format!(
                "Feature file not found: {}",
                feature_abs.display()
            ));
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "update-step",
                source,
            });
        }
    };

    // ---- Parse Gherkin ----
    // TS `@cucumber/gherkin` PARSES an empty / comment-only document
    // successfully but yields `gherkinDocument.feature === null`, surfaced
    // as `Feature file does not contain a valid Feature`. The Rust parser
    // throws a syntax error on the same inputs, so replicate the TS
    // "null feature" classification explicitly first.
    if is_empty_or_comment_only(&content) {
        return err_envelope("Feature file does not contain a valid Feature".to_string());
    }
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            return err_envelope(format!("Invalid Gherkin syntax: {e}"));
        }
    };

    // ---- Find the scenario ----
    let scenario = feature.scenarios.iter().find(|s| s.name == args.scenario);
    let scenario = match scenario {
        Some(s) => s,
        None => {
            return err_envelope(format!(
                "Scenario '{}' not found in feature file",
                args.scenario
            ));
        }
    };

    // ---- Find the step (TS src/commands/update-step.ts:104-112) ----
    // gherkin-0.16: `Step.value` is the text without keyword, `Step.keyword`
    // is the raw keyword INCLUDING its trailing space (e.g. "Given ").
    let current_trimmed = args.current_step.trim();
    let step = scenario.steps.iter().find(|s| {
        let step_text = s.value.as_str();
        let full = format!("{}{}", s.keyword, s.value);
        step_text == args.current_step
            || full.trim() == current_trimmed
            || format!("{} {}", s.keyword.trim(), step_text) == current_trimmed
    });
    let step = match step {
        Some(s) => s,
        None => {
            return err_envelope(format!(
                "Step '{}' not found in scenario '{}'",
                args.current_step, args.scenario
            ));
        }
    };

    // ---- Line-based rewrite ----
    let step_line = step.position.line as usize;
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    let line_index = step_line.saturating_sub(1);
    if line_index >= lines.len() {
        return err_envelope("Could not parse step line".to_string());
    }

    let (indentation, current_keyword, current_text) = match parse_step_line(&lines[line_index]) {
        Some(parts) => parts,
        None => return err_envelope("Could not parse step line".to_string()),
    };

    // ---- Determine new keyword & text (TS:147-161) ----
    let new_keyword = keyword.unwrap_or(&current_keyword).to_string();
    let new_text: String = if let Some(t) = text {
        // If --text carries a keyword prefix, use only the text part.
        match strip_keyword_prefix(t) {
            Some(stripped) => stripped,
            None => t.to_string(),
        }
    } else {
        current_text
    };

    lines[line_index] = format!("{indentation}{new_keyword} {new_text}");
    let new_content = lines.join("\n");

    // ---- Validate rewritten content still parses ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return err_envelope(format!("Update would result in invalid Gherkin: {e}"));
    }

    // ---- Write ----
    std::fs::write(&feature_abs, &new_content).map_err(|source| FspecCoreError::Io {
        command: "update-step",
        source,
    })?;

    // ---- Success envelope ----
    let file_name = feature_abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&feature_rel)
        .to_string();
    let response = json!({
        "success": true,
        "message": format!(
            "Successfully updated step in scenario '{}' in {}",
            args.scenario, file_name
        ),
    });
    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-step",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Treat an empty string the same as an absent option (TS truthiness:
/// `if (!text && !keyword)`).
fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.is_empty())
}

/// True when the document is blank / comment-only (no `feature` node would
/// be produced by `@cucumber/gherkin`). A `# language:` directive disables
/// this fast path because TS treats a language-only doc as a parse error.
/// See `update_scenario::is_empty_or_comment_only` for the full rationale.
fn is_empty_or_comment_only(content: &str) -> bool {
    let mut saw_only_blank_or_comment = true;
    for line in content.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(after_hash) = trimmed.strip_prefix('#') {
            if is_language_directive(after_hash) {
                return false;
            }
            continue;
        }
        saw_only_blank_or_comment = false;
        break;
    }
    saw_only_blank_or_comment
}

/// Matches `@cucumber/gherkin`'s `LANGUAGE_PATTERN`
/// `\s*language\s*:\s*([a-zA-Z\-_]+)\s*` on the text following a leading `#`.
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
    rest[value.len()..].trim().is_empty()
}

/// Mirror of TS feature-path resolution.
fn resolve_feature_rel(feature: &str) -> String {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        feature.to_string()
    } else {
        format!("spec/features/{feature}.feature")
    }
}

/// Parse a step source line into `(indentation, keyword, text)`. Mirrors the
/// TS regex `^(\s*)(Given|When|Then|And|But)\s+(.+)$`.
fn parse_step_line(line: &str) -> Option<(String, String, String)> {
    let trimmed_start = line.trim_start();
    let indent_len = line.len() - trimmed_start.len();
    let indentation = line[..indent_len].to_string();

    for kw in ["Given", "When", "Then", "And", "But"] {
        if let Some(rest) = trimmed_start.strip_prefix(kw) {
            // Require at least one whitespace char then non-empty text.
            let after = rest.trim_start();
            if after.len() < rest.len() && !after.is_empty() {
                return Some((indentation, kw.to_string(), after.to_string()));
            }
        }
    }
    None
}

/// If `text` begins with a step keyword followed by whitespace, return the
/// text after the keyword. Mirrors the TS regex
/// `^(?:Given|When|Then|And|But)\s+(.+)$`.
fn strip_keyword_prefix(text: &str) -> Option<String> {
    for kw in ["Given", "When", "Then", "And", "But"] {
        if let Some(rest) = text.strip_prefix(kw) {
            let after = rest.trim_start();
            if after.len() < rest.len() && !after.is_empty() {
                return Some(after.to_string());
            }
        }
    }
    None
}

/// Build an `Ok(json)` soft-failure envelope `{success:false,error}`.
fn err_envelope(error: String) -> Result<String, FspecCoreError> {
    let v = json!({ "success": false, "error": error });
    serde_json::to_string(&v).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-step",
        reason: format!("failed to serialise error envelope: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: UpdateStepArgs = serde_json::from_str(
            r#"{"feature":"x.feature","scenario":"S","currentStep":"Given x","text":"y"}"#,
        )
        .unwrap();
        assert_eq!(a.feature, "x.feature");
        assert_eq!(a.scenario, "S");
        assert_eq!(a.current_step, "Given x");
        assert_eq!(a.text.as_deref(), Some("y"));
        assert_eq!(a.keyword, None);
    }

    #[test]
    fn step_line_parse() {
        assert_eq!(
            parse_step_line("    Given I am here"),
            Some(("    ".to_string(), "Given".to_string(), "I am here".to_string()))
        );
        assert_eq!(parse_step_line("    Scenario: x"), None);
        assert_eq!(parse_step_line("    Given"), None);
    }

    #[test]
    fn keyword_prefix_strip() {
        assert_eq!(strip_keyword_prefix("When I submit"), Some("I submit".to_string()));
        assert_eq!(strip_keyword_prefix("I submit"), None);
    }
}
