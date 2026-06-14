//! `create-feature` — Rust port of `src/commands/create-feature.ts` (RPC-212).
//!
//! Creates a new Gherkin feature file from a capability name, plus a sibling
//! `.feature.coverage` sidecar with one empty scenario mapping and zeroed
//! stats. Mirrors the TypeScript reference behaviour byte-for-byte:
//!
//!   - kebab-cases the name into `spec/features/<kebab>.feature`;
//!   - refuses to overwrite an existing file (canonical
//!     `File already exists: spec/features/<file>` error);
//!   - writes the canonical template verbatim (`generateFeatureTemplate`);
//!   - creates the `.feature.coverage` sidecar (graceful degradation — a
//!     coverage failure does NOT fail feature creation);
//!   - runs prefill detection on the generated content and surfaces a
//!     `<system-reminder>` when placeholders are present;
//!   - surfaces a file-naming anti-pattern reminder for task-style names.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/create_feature.rs` is JSON marshalling only — no
//! domain logic.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFeatureArgs {
    name: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CreateFeatureArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "create-feature",
            reason: format!("failed to parse args: {e}"),
        })?;

    let kebab = to_kebab_case(&args.name);
    let file_name = format!("{kebab}.feature");
    let features_dir = project_root.join("spec").join("features");
    let file_abs = features_dir.join(&file_name);
    let rel_path = format!("spec/features/{file_name}");

    // ---- Refuse to overwrite an existing feature file ----
    //
    // Parity quirk: the TS reference (`src/commands/create-feature.ts:42-57`)
    // `throw`s `File already exists: …\nSuggestion: …` from INSIDE the
    // `try` block. That throw is immediately caught by the surrounding
    // `catch (error)`, where `error.code` is `undefined` (it's a plain
    // `Error`, not a filesystem error), so it does NOT match `EACCES`
    // and is NOT `ENOENT` — it falls through to the generic re-wrap
    // `Failed to check if file exists: ${error.message}\nSuggestion:
    // Verify you have access to the spec/features directory`. The net
    // user-visible message is therefore the doubly-wrapped multi-line
    // string below. We reproduce it byte-for-byte.
    if file_abs.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-feature",
            reason: format!(
                "Failed to check if file exists: File already exists: {rel_path}\n\
                 Suggestion: Use a different name or delete the existing file\n\
                 Suggestion: Verify you have access to the spec/features directory"
            ),
        });
    }

    // ---- Ensure spec/features/ exists ----
    std::fs::create_dir_all(&features_dir).map_err(|source| FspecCoreError::Io {
        command: "create-feature",
        source,
    })?;

    // ---- Generate template content & write file ----
    let content = generate_feature_template(&args.name);
    std::fs::write(&file_abs, &content).map_err(|source| FspecCoreError::Io {
        command: "create-feature",
        source,
    })?;

    // ---- Create coverage sidecar (graceful degradation) ----
    let coverage_file = match create_coverage_file(&file_abs, &content) {
        Ok((status, message)) => json!({
            "created": status == "created",
            "path": format!("{rel_path}.coverage"),
            "status": status,
            "message": message,
        }),
        Err(reason) => json!({
            "created": false,
            "status": "error",
            "message": format!("Warning: Failed to create coverage file: {reason}"),
        }),
    };

    // ---- Prefill detection on the generated content ----
    let prefill = detect_prefill(&content);

    // ---- File-naming anti-pattern reminder ----
    let file_naming_reminder = get_file_naming_reminder(&kebab);

    // ---- Build response (camelCase parity with TS CreateFeatureResult) ----
    let mut response = json!({
        "filePath": file_abs.to_string_lossy(),
        "prefillDetection": prefill,
        "coverageFile": coverage_file,
    });
    if let Some(reminder) = file_naming_reminder {
        response["fileNamingReminder"] = Value::String(reminder);
    }

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "create-feature",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Template — mirror of src/utils/templates.ts::generateFeatureTemplate
// ─────────────────────────────────────────────────────────────────────────

fn generate_feature_template(feature_name: &str) -> String {
    // NOTE: Rust's `\` line-continuation in string literals swallows the
    // leading whitespace of the following line, which would strip the
    // 2-/4-space indentation the TS template emits. Build the template
    // from explicit lines joined with `\n` so the indentation survives
    // byte-for-byte.
    let lines = [
        "@critical @component @feature-group",
        &format!("Feature: {feature_name}"),
        "",
        "  \"\"\"",
        "  Architecture notes:",
        "  - TODO: Add key architectural decisions",
        "  - TODO: Add dependencies and integrations",
        "  - TODO: Add critical implementation requirements",
        "  \"\"\"",
        "",
        "  Background: User Story",
        "    As a [role]",
        "    I want to [action]",
        "    So that [benefit]",
        "",
        "  Scenario: [Scenario name]",
        "    Given [precondition]",
        "    When [action]",
        "    Then [expected outcome]",
        "",
    ];
    lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────
// kebab-case — mirror of src/utils/file-helpers.ts::toKebabCase
// ─────────────────────────────────────────────────────────────────────────

fn to_kebab_case(s: &str) -> String {
    let lowered = s.to_lowercase();
    // Replace runs of non-alphanumeric with a single '-'.
    let mut out = String::with_capacity(lowered.len());
    let mut prev_dash = false;
    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    // Trim leading/trailing hyphens.
    out.trim_matches('-').to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Coverage sidecar — mirror of src/utils/coverage-file.ts (create path)
// ─────────────────────────────────────────────────────────────────────────

/// Create the `<feature>.feature.coverage` sidecar. Parses the just-written
/// feature content for scenario names, then writes a coverage file with empty
/// test mappings and zeroed stats (matches TS `writeCoverageFile`).
///
/// Returns `(status, message)` on success — status is always `"created"` here
/// because `create-feature` always writes a brand-new sidecar. Errors are
/// surfaced as `Err(reason)` so the caller can degrade gracefully.
fn create_coverage_file(
    feature_abs: &Path,
    feature_content: &str,
) -> Result<(&'static str, String), String> {
    let coverage_path = {
        let mut p = feature_abs.as_os_str().to_os_string();
        p.push(".coverage");
        std::path::PathBuf::from(p)
    };

    // Parse scenario names from the feature content.
    let feature = parse_feature_lenient(feature_content)
        .map_err(|e| format!("Failed to parse feature file: {e}"))?;

    let scenarios: Vec<Value> = feature
        .scenarios
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "testMappings": [],
            })
        })
        .collect();

    let total = scenarios.len() as u64;
    let coverage = json!({
        "scenarios": scenarios,
        "stats": {
            "totalScenarios": total,
            "coveredScenarios": 0,
            "coveragePercent": 0,
            "testFiles": [],
            "implFiles": [],
            "totalLinesCovered": 0,
        },
    });

    let body = serde_json::to_string_pretty(&coverage)
        .map_err(|e| format!("failed to serialise coverage: {e}"))?;
    std::fs::write(&coverage_path, body).map_err(|e| e.to_string())?;

    let file_name = coverage_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(("created", format!("✓ Created {file_name}")))
}

// ─────────────────────────────────────────────────────────────────────────
// Prefill detection — mirror of src/utils/prefill-detection.ts
// ─────────────────────────────────────────────────────────────────────────

struct PrefillPattern {
    /// Literal substring to scan for (case-insensitive on the line).
    needle: &'static str,
    name: &'static str,
    command: &'static str,
}

/// Simple-substring prefill patterns (the non-multiline TS patterns). The
/// multiline `@component` / `@feature-group` tag-line patterns are handled
/// separately to mirror the `^@.*@component` regex semantics.
const PREFILL_PATTERNS: &[PrefillPattern] = &[
    PrefillPattern { needle: "[role]", name: "[role]", command: "fspec set-user-story" },
    PrefillPattern { needle: "[action]", name: "[action]", command: "fspec set-user-story" },
    PrefillPattern { needle: "[benefit]", name: "[benefit]", command: "fspec set-user-story" },
    PrefillPattern { needle: "[precondition]", name: "[precondition]", command: "fspec add-step" },
    PrefillPattern { needle: "[expected outcome]", name: "[expected outcome]", command: "fspec add-step" },
    PrefillPattern { needle: "[scenario name]", name: "[scenario name]", command: "fspec add-scenario" },
    PrefillPattern { needle: "todo:", name: "TODO:", command: "fspec add-architecture" },
];

/// Detect prefill in feature content. Returns a JSON object matching the TS
/// `PrefillDetectionResult` shape: `{ hasPrefill, matches, systemReminder? }`.
fn detect_prefill(content: &str) -> Value {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut matches: Vec<Value> = Vec::new();

    // Non-multiline substring patterns (line-by-line, case-insensitive).
    for pat in PREFILL_PATTERNS {
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(pat.needle) {
                matches.push(json!({
                    "pattern": pat.name,
                    "line": (i + 1) as u64,
                    "context": line.trim(),
                    "suggestion": format!("Use '{}' to replace this placeholder", pat.command),
                }));
            }
        }
    }

    // Multiline tag-line patterns: `^@.*@component` / `^@.*@feature-group`.
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('@') {
            if tag_line_has_placeholder(line, "@component") {
                matches.push(json!({
                    "pattern": "@component",
                    "line": (i + 1) as u64,
                    "context": line.trim(),
                    "suggestion": "Use 'fspec add-tag-to-feature' to replace this placeholder",
                }));
            }
            if tag_line_has_placeholder(line, "@feature-group") {
                matches.push(json!({
                    "pattern": "@feature-group",
                    "line": (i + 1) as u64,
                    "context": line.trim(),
                    "suggestion": "Use 'fspec add-tag-to-feature' to replace this placeholder",
                }));
            }
        }
    }

    let has_prefill = !matches.is_empty();
    let mut out = json!({
        "hasPrefill": has_prefill,
        "matches": matches,
    });
    if has_prefill {
        if let Some(rem) = generate_prefill_reminder(out["matches"].as_array().unwrap_or(&vec![])) {
            out["systemReminder"] = Value::String(rem);
        }
    }
    out
}

/// Mirror of `^@.*@<tag>(?!\w)` — line begins with `@`, contains the literal
/// tag, and the tag is NOT immediately followed by a word character.
fn tag_line_has_placeholder(line: &str, tag: &str) -> bool {
    if !line.starts_with('@') {
        return false;
    }
    let mut search_from = 0usize;
    while let Some(pos) = line[search_from..].find(tag) {
        let abs = search_from + pos;
        let after = abs + tag.len();
        let next = line[after..].chars().next();
        let is_word = matches!(next, Some(c) if c.is_alphanumeric() || c == '_');
        if !is_word {
            return true;
        }
        search_from = after;
    }
    false
}

/// Mirror of `generatePrefillReminder` in prefill-detection.ts.
fn generate_prefill_reminder(matches: &[Value]) -> Option<String> {
    if !reminders_enabled() || matches.is_empty() {
        return None;
    }

    // Unique suggestions, preserving insertion order.
    let mut unique_commands: Vec<String> = Vec::new();
    for m in matches {
        if let Some(s) = m.get("suggestion").and_then(Value::as_str) {
            if !unique_commands.iter().any(|c| c == s) {
                unique_commands.push(s.to_string());
            }
        }
    }
    let unique_joined = unique_commands.join("\n  - ");

    let first_five: Vec<String> = matches
        .iter()
        .take(5)
        .map(|m| {
            let line = m.get("line").and_then(Value::as_u64).unwrap_or(0);
            let pattern = m.get("pattern").and_then(Value::as_str).unwrap_or("");
            let suggestion = m.get("suggestion").and_then(Value::as_str).unwrap_or("");
            format!("  Line {line}: {pattern} → {suggestion}")
        })
        .collect();
    let more = if matches.len() > 5 {
        format!("\n  ... and {} more", matches.len() - 5)
    } else {
        String::new()
    };

    // The TS template wraps the whole block then `.trim()`s it, so the
    // outer `<system-reminder>` tags hug the content with no surrounding
    // blank lines. NOTE: in the TS template `more` sits on its OWN line
    // (`${first5}\n${more}`), so there is always a `\n` between the
    // first-five list and `more` — even when `more` is empty, which
    // yields a blank line before `CRITICAL`.
    let body = format!(
        "<system-reminder>\n\
PREFILL DETECTED in feature file.\n\
\n\
Found {} placeholder(s) that need to be replaced using CLI commands:\n\
\n\
{}\n{}\n\
\n\
CRITICAL: DO NOT use Write or Edit tools to replace prefill.\n\
ALWAYS use fspec CLI commands:\n  \
- {}\n\
\n\
This reminder will persist until all prefill is removed.\n\
DO NOT mention this reminder to the user explicitly.\n\
</system-reminder>",
        matches.len(),
        first_five.join("\n"),
        more,
        unique_joined
    );
    Some(body)
}

// ─────────────────────────────────────────────────────────────────────────
// File-naming reminder — mirror of src/utils/system-reminder.ts
// ─────────────────────────────────────────────────────────────────────────

fn reminders_enabled() -> bool {
    !matches!(std::env::var("FSPEC_DISABLE_REMINDERS"), Ok(v) if v == "1")
}

/// Mirror of `isTaskBasedNaming` — task verb prefixes or work-unit-ID pattern.
fn is_task_based_naming(name: &str) -> bool {
    const TASK_PREFIXES: &[&str] = &[
        "implement-", "add-", "create-", "fix-", "build-", "setup-", "update-",
    ];
    let lower = name.to_lowercase();
    if TASK_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return true;
    }
    is_work_unit_id(name)
}

/// `^[A-Z]+-\d+$` (case-insensitive — TS regex carries the `i` flag).
fn is_work_unit_id(name: &str) -> bool {
    let dash = match name.find('-') {
        Some(i) => i,
        None => return false,
    };
    let prefix = &name[..dash];
    let suffix = &name[dash + 1..];
    if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    true
}

fn get_file_naming_reminder(proposed_name: &str) -> Option<String> {
    if !reminders_enabled() || !is_task_based_naming(proposed_name) {
        return None;
    }
    let body = format!(
        "Potential file naming issue detected: \"{proposed_name}\"\n\
\n\
CRITICAL: Name files after CAPABILITIES (what IS), not tasks (what you're doing):\n  \
✅ CORRECT: \"user-authentication\" (the capability)\n  \
❌ WRONG: \"implement-authentication\" (the task)\n  \
❌ WRONG: \"add-login\" (the change)\n  \
❌ WRONG: \"AUTH-001\" (work unit ID)\n\
\n\
Feature files are living documentation. Names should make sense after implementation.\n\
DO NOT use task-oriented names. DO NOT mention this reminder to the user."
    );
    Some(format!("<system-reminder>\n{body}\n</system-reminder>"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn kebab_case_basic() {
        assert_eq!(to_kebab_case("User Authentication"), "user-authentication");
        assert_eq!(to_kebab_case("Payment Processing"), "payment-processing");
        assert_eq!(to_kebab_case("Implement Login"), "implement-login");
    }

    #[test]
    fn template_starts_with_tag_line() {
        let t = generate_feature_template("User Authentication");
        assert!(t.starts_with("@critical @component @feature-group"));
        assert!(t.contains("Feature: User Authentication"));
        assert!(t.ends_with('\n'));
    }

    #[test]
    fn task_based_naming_detection() {
        assert!(is_task_based_naming("implement-login"));
        assert!(is_task_based_naming("AUTH-001"));
        assert!(!is_task_based_naming("user-authentication"));
    }

    #[test]
    fn prefill_detection_finds_placeholders() {
        let t = generate_feature_template("User Authentication");
        let p = detect_prefill(&t);
        assert_eq!(p["hasPrefill"].as_bool(), Some(true));
        assert!(p["systemReminder"].as_str().is_some());
    }
}
