//! `review` — Rust port of `src/commands/review.ts` (RPC-295).
//!
//! Performs a comprehensive review of a work unit: ACDD compliance, coding
//! standards scan of linked test files, coverage analysis, and a final
//! AI deep-review reminder formatted for the configured agent.
//!
//! `review` NEVER fails on findings — the only error path is a missing work
//! unit (`Work unit '<id>' does not exist`).

use std::collections::BTreeSet;
use std::path::Path;

use gherkin::Feature;
use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;

// ─────────────────────────────────────────────────────────────────────────
// Inlined agent registry (port of AGENT_REGISTRY) + getAgentConfig
// ─────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    Cli,
    Ide,
    Extension,
}

struct Agent {
    id: &'static str,
    supports_system_reminders: bool,
    category: Category,
}

use Category::{Cli, Extension, Ide};

/// Faithful port of the TS AGENT_REGISTRY capability flags (all 19 agents).
const AGENT_REGISTRY: &[Agent] = &[
    Agent { id: "claude", supports_system_reminders: true, category: Cli },
    Agent { id: "cursor", supports_system_reminders: false, category: Ide },
    Agent { id: "cline", supports_system_reminders: false, category: Extension },
    Agent { id: "aider", supports_system_reminders: false, category: Cli },
    Agent { id: "windsurf", supports_system_reminders: false, category: Ide },
    Agent { id: "copilot", supports_system_reminders: false, category: Extension },
    Agent { id: "gemini", supports_system_reminders: false, category: Cli },
    Agent { id: "qwen", supports_system_reminders: false, category: Cli },
    Agent { id: "kilocode", supports_system_reminders: false, category: Ide },
    Agent { id: "roo", supports_system_reminders: false, category: Ide },
    Agent { id: "codebuddy", supports_system_reminders: false, category: Cli },
    Agent { id: "amazonq", supports_system_reminders: false, category: Extension },
    Agent { id: "auggie", supports_system_reminders: false, category: Cli },
    Agent { id: "opencode", supports_system_reminders: false, category: Cli },
    Agent { id: "codex", supports_system_reminders: false, category: Cli },
    Agent { id: "factory", supports_system_reminders: false, category: Cli },
    Agent { id: "crush", supports_system_reminders: false, category: Cli },
    Agent { id: "codex-cli", supports_system_reminders: false, category: Cli },
    Agent { id: "antigravity", supports_system_reminders: true, category: Cli },
];

fn get_agent_by_id(id: &str) -> Option<&'static Agent> {
    AGENT_REGISTRY.iter().find(|a| a.id == id)
}

/// Resolved agent capabilities used to format the AI reminder.
pub(crate) struct ResolvedAgent {
    pub(crate) supports_system_reminders: bool,
    pub(crate) category: Category,
}

/// Port of `getAgentConfig`: FSPEC_AGENT env > spec/fspec-config.json > default.
pub(crate) fn get_agent_config(project_root: &Path) -> ResolvedAgent {
    // Priority 1: FSPEC_AGENT env var.
    if let Ok(env_agent) = std::env::var("FSPEC_AGENT") {
        if !env_agent.is_empty() {
            if let Some(a) = get_agent_by_id(&env_agent) {
                return ResolvedAgent {
                    supports_system_reminders: a.supports_system_reminders,
                    category: a.category,
                };
            }
        }
    }

    // Priority 2: spec/fspec-config.json.
    let config_path = project_root.join("spec").join("fspec-config.json");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(cfg) = serde_json::from_str::<Value>(&content) {
            if let Some(agent_id) = cfg.get("agent").and_then(Value::as_str) {
                if let Some(a) = get_agent_by_id(agent_id) {
                    return ResolvedAgent {
                        supports_system_reminders: a.supports_system_reminders,
                        category: a.category,
                    };
                }
            }
        }
    }

    // Priority 3: safe default (plain text, cli category).
    ResolvedAgent {
        supports_system_reminders: false,
        category: Cli,
    }
}

/// Port of `formatAgentOutput`:
/// - system-reminders → `<system-reminder>\n{msg}\n</system-reminder>`
/// - ide/extension → `**⚠️ IMPORTANT:** {msg}`
/// - cli/default → `**IMPORTANT:** {msg}`
pub(crate) fn format_agent_output(agent: &ResolvedAgent, message: &str) -> String {
    if agent.supports_system_reminders {
        return format!("<system-reminder>\n{message}\n</system-reminder>");
    }
    if agent.category == Ide || agent.category == Extension {
        return format!("**⚠️ IMPORTANT:** {message}");
    }
    format!("**IMPORTANT:** {message}")
}

// ─────────────────────────────────────────────────────────────────────────
// Args + finding structures
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ReviewArgs {
    work_unit_id: Option<String>,
}

struct CriticalIssue {
    issue: String,
    location: Option<String>,
    fix: String,
    action: String,
}

struct Warning {
    issue: String,
    location: Option<String>,
    fix: String,
    action: String,
}

struct Recommendation {
    recommendation: String,
    rationale: String,
    action: String,
}

/// A linked feature reference (file + scenarios), mirroring the subset of
/// `showWorkUnit().linkedFeatures` consumed by `review`.
struct LinkedFeature {
    file: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Dispatcher entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ReviewArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "review",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args.work_unit_id.ok_or(FspecCoreError::InvalidArgs {
        command: "review",
        reason: "missing required argument: workUnitId".to_string(),
    })?;

    // Detect agent for formatted output.
    let agent = get_agent_config(project_root);

    // Read work units (bare read — ENOENT escalates as TS readFile does).
    let work_units_file = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&work_units_file).map_err(|source| FspecCoreError::Io {
        command: "review",
        source,
    })?;
    let root: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "work-units.json".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    let wu = root
        .get("workUnits")
        .and_then(Value::as_object)
        .and_then(|m| m.get(&work_unit_id))
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "review",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        })?;

    let title = wu.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let status = wu.get("status").and_then(Value::as_str).unwrap_or("").to_string();

    let mut lines: Vec<String> = Vec::new();
    let mut critical_issues: Vec<CriticalIssue> = Vec::new();
    let mut warnings: Vec<Warning> = Vec::new();
    let mut recommendations: Vec<Recommendation> = Vec::new();

    // Build review header.
    lines.push("=".repeat(80));
    lines.push(format!("REVIEW: {work_unit_id} - {title}"));
    lines.push("=".repeat(80));
    lines.push(String::new());

    // Step 2: Read Feature Files (linked-feature lookup, implemented locally).
    let linked = scan_linked_features(project_root, &work_unit_id);
    let mut feature_file: Option<String> = None;
    if let Some(first) = linked.first() {
        feature_file = Some(first.file.clone());
        // Parity: TS reads the feature content and parses Gherkin, pushing a
        // warning only on parse failure. The lenient parser already accepted
        // the file during the scan, so no extra warning path is required here.
    } else {
        warnings.push(Warning {
            issue: "No linked feature files found".to_string(),
            location: Some(format!("Work unit {work_unit_id}")),
            fix: "Create feature file with acceptance criteria".to_string(),
            action: format!("fspec create-feature \"{title}\""),
        });
    }

    // Step 3: Analyze Test Coverage (read <feature>.coverage).
    let mut coverage_data: Option<Value> = None;
    if let Some(ff) = &feature_file {
        let coverage_path = project_root.join(format!("{ff}.coverage"));
        if let Ok(content) = std::fs::read_to_string(&coverage_path) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                coverage_data = Some(parsed);
            }
        }
    }

    // Step 4: Validate ACDD Workflow Compliance.
    let mut acdd_passed: Vec<String> = Vec::new();
    let mut acdd_failed: Vec<String> = Vec::new();

    let rules = wu.get("rules").and_then(Value::as_array);
    let rules_len = rules.map(Vec::len).unwrap_or(0);
    let examples_len = wu
        .get("examples")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let questions_answered = wu
        .get("questions")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|q| q.get("selected").and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);

    // Check Example Mapping.
    if rules_len > 0 {
        acdd_passed.push(format!(
            "Example Mapping completed ({rules_len} rules, {examples_len} examples, {questions_answered} questions answered)"
        ));
    } else if status != "backlog" {
        acdd_failed.push("No Example Mapping data found (missing rules/examples)".to_string());
        recommendations.push(Recommendation {
            recommendation: "Complete Example Mapping before specifying".to_string(),
            rationale:
                "Example Mapping clarifies requirements and prevents building the wrong feature"
                    .to_string(),
            action: format!(
                "fspec add-rule {work_unit_id} \"<rule>\" and fspec add-example {work_unit_id} \"<example>\""
            ),
        });
    }

    // Check feature file creation during specifying phase.
    let state_history = wu.get("stateHistory").and_then(Value::as_array);
    if feature_file.is_some() && state_history.is_some() {
        let has_specifying = state_history
            .map(|h| {
                h.iter()
                    .any(|e| e.get("state").and_then(Value::as_str) == Some("specifying"))
            })
            .unwrap_or(false);
        if has_specifying {
            acdd_passed.push("Feature file created during specifying phase".to_string());
        }
    } else if status != "backlog" && status != "specifying" {
        acdd_failed.push("Feature file should be created during specifying phase".to_string());
    }

    // Check test coverage.
    let coverage_percent = coverage_data
        .as_ref()
        .and_then(|c| c.get("stats"))
        .and_then(|s| s.get("coveragePercent"))
        .and_then(Value::as_f64);
    if let Some(pct) = coverage_percent {
        if pct == 100.0 {
            acdd_passed.push("All scenarios have test coverage (100%)".to_string());
        } else if pct > 0.0 {
            acdd_failed.push(format!("Incomplete test coverage ({}%)", fmt_num(pct)));
            let short = feature_file
                .as_deref()
                .map(strip_feature_name)
                .unwrap_or_default();
            recommendations.push(Recommendation {
                recommendation: "Add tests for uncovered scenarios".to_string(),
                rationale: "All acceptance criteria must have corresponding tests".to_string(),
                action: format!("fspec show-coverage {short} to see uncovered scenarios"),
            });
        }
    }

    // Check temporal ordering.
    if let Some(history) = state_history {
        if !history.is_empty() {
            acdd_passed.push(format!(
                "Temporal ordering verified ({} state transitions)",
                history.len()
            ));
        }
    }

    // Step 5: Validate Coding Standards (scan linked test files).
    if feature_file.is_some() {
        if let Some(scenarios) = coverage_data
            .as_ref()
            .and_then(|c| c.get("scenarios"))
            .and_then(Value::as_array)
        {
            for scenario in scenarios {
                let mappings = scenario.get("testMappings").and_then(Value::as_array);
                if let Some(mappings) = mappings {
                    for mapping in mappings {
                        let file = match mapping.get("file").and_then(Value::as_str) {
                            Some(f) => f,
                            None => continue,
                        };
                        let test_path = project_root.join(file);
                        let test_content = match std::fs::read_to_string(&test_path) {
                            Ok(c) => c,
                            Err(_) => continue, // Test file might not exist.
                        };

                        // Check for `: any`.
                        if test_content.contains(": any") {
                            critical_issues.push(CriticalIssue {
                                issue: "Use of `any` type detected".to_string(),
                                location: Some(file.to_string()),
                                fix: "Replace `any` with proper TypeScript types".to_string(),
                                action: "Review file and add proper type annotations".to_string(),
                            });
                        }

                        // Check for `require(`.
                        if test_content.contains("require(") {
                            critical_issues.push(CriticalIssue {
                                issue: "CommonJS `require()` detected".to_string(),
                                location: Some(file.to_string()),
                                fix: "Use ES6 import statements".to_string(),
                                action: "Replace require() with import".to_string(),
                            });
                        }

                        // Check for file-extension imports: /import .* from ['"].*\.(ts|js)['"]/
                        if has_extension_import(&test_content) {
                            critical_issues.push(CriticalIssue {
                                issue: "File extensions in imports".to_string(),
                                location: Some(file.to_string()),
                                fix: "Remove .ts/.js extensions from imports".to_string(),
                                action: "Vite handles file extensions automatically".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Build Issues Found section.
    lines.push("## Issues Found".to_string());
    lines.push(String::new());
    lines.push("### 🔴 Critical Issues".to_string());
    if !critical_issues.is_empty() {
        for (index, issue) in critical_issues.iter().enumerate() {
            lines.push(format!("{}. **Issue:** {}", index + 1, issue.issue));
            if let Some(loc) = &issue.location {
                lines.push(format!("   - **Location:** {loc}"));
            }
            lines.push(format!("   - **Fix:** {}", issue.fix));
            lines.push(format!("   - **Action:** {}", issue.action));
            lines.push(String::new());
        }
    } else {
        lines.push("No critical issues detected.".to_string());
        lines.push(String::new());
    }

    lines.push("### 🟡 Warnings".to_string());
    if !warnings.is_empty() {
        for (index, warning) in warnings.iter().enumerate() {
            lines.push(format!("{}. **Issue:** {}", index + 1, warning.issue));
            if let Some(loc) = &warning.location {
                lines.push(format!("   - **Location:** {loc}"));
            }
            lines.push(format!("   - **Fix:** {}", warning.fix));
            lines.push(format!("   - **Action:** {}", warning.action));
            lines.push(String::new());
        }
    } else {
        lines.push("No warnings detected.".to_string());
        lines.push(String::new());
    }

    // Recommendations section.
    if !recommendations.is_empty() {
        lines.push("## Recommendations".to_string());
        lines.push(String::new());
        lines.push("**IMPORTANT:** ACDD COMPLIANCE REVIEW".to_string());
        lines.push(String::new());
        for (index, rec) in recommendations.iter().enumerate() {
            lines.push(format!("{}. **Recommendation:** {}", index + 1, rec.recommendation));
            lines.push(format!("   - **Rationale:** {}", rec.rationale));
            lines.push(format!("   - **Action:** {}", rec.action));
            lines.push(String::new());
        }
    }

    // ACDD Compliance section.
    lines.push("## ACDD Compliance".to_string());
    lines.push(String::new());
    if !acdd_passed.is_empty() {
        lines.push("✅ **Passed:**".to_string());
        for item in &acdd_passed {
            lines.push(format!("- {item}"));
        }
        lines.push(String::new());
    }
    if !acdd_failed.is_empty() {
        lines.push("❌ **Failed:**".to_string());
        for item in &acdd_failed {
            lines.push(format!("- {item}"));
        }
        lines.push(String::new());
    }

    // Coverage Analysis section.
    lines.push("## Coverage Analysis".to_string());
    lines.push(String::new());
    if let Some(stats) = coverage_data.as_ref().and_then(|c| c.get("stats")) {
        let total = stats.get("totalScenarios").and_then(Value::as_f64).unwrap_or(0.0);
        let covered = stats.get("coveredScenarios").and_then(Value::as_f64).unwrap_or(0.0);
        let pct = stats.get("coveragePercent").and_then(Value::as_f64).unwrap_or(0.0);
        lines.push(format!("- **Total Scenarios:** {}", fmt_num(total)));
        lines.push(format!(
            "- **Covered Scenarios:** {} ({}%)",
            fmt_num(covered),
            fmt_num(pct)
        ));

        if let Some(scenarios) = coverage_data
            .as_ref()
            .and_then(|c| c.get("scenarios"))
            .and_then(Value::as_array)
        {
            let uncovered: Vec<String> = scenarios
                .iter()
                .filter(|s| {
                    s.get("testMappings")
                        .and_then(Value::as_array)
                        .map(Vec::is_empty)
                        .unwrap_or(true)
                })
                .filter_map(|s| s.get("name").and_then(Value::as_str).map(str::to_string))
                .collect();
            if !uncovered.is_empty() {
                lines.push(String::new());
                lines.push("**Uncovered Scenarios:**".to_string());
                for name in &uncovered {
                    lines.push(format!("  - {name}"));
                }
            }
        }
    } else {
        lines.push("- No coverage data available".to_string());
    }
    lines.push(String::new());

    // Summary section.
    lines.push("## Summary".to_string());
    lines.push(String::new());

    let assessment = if !critical_issues.is_empty() {
        "CRITICAL ISSUES"
    } else if !warnings.is_empty() || !acdd_failed.is_empty() {
        "NEEDS WORK"
    } else {
        "PASS"
    };

    lines.push(format!("**Overall Assessment:** {assessment}"));
    lines.push(String::new());

    if status != "done" {
        lines.push(format!("**Current State:** {status}"));
        lines.push(String::new());
    }

    lines.push("**Priority Actions:**".to_string());

    let mut priority_actions: Vec<String> = Vec::new();
    if !critical_issues.is_empty() {
        priority_actions.push(format!("Fix {} critical issue(s)", critical_issues.len()));
    }
    if !acdd_failed.is_empty() {
        priority_actions.push("Address ACDD compliance violations".to_string());
    }
    if let Some(pct) = coverage_percent {
        if pct < 100.0 {
            priority_actions.push("Complete test coverage for all scenarios".to_string());
        }
    }
    if status != "done" {
        priority_actions.push(format!("Continue {status} phase"));
    }
    if priority_actions.is_empty() {
        priority_actions.push("Work unit review complete - no critical actions needed".to_string());
    }
    for (index, action) in priority_actions.iter().enumerate() {
        lines.push(format!("{}. {}", index + 1, action));
    }
    lines.push(String::new());

    // Build AI-driven deep analysis reminder (includes ACDD recommendations).
    let system_reminder = build_ai_analysis_reminder(
        &work_unit_id,
        &title,
        coverage_data.as_ref(),
        &recommendations,
    );

    lines.push(format_agent_output(&agent, &system_reminder));
    lines.push(String::new());

    Ok(lines.join("\n"))
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Render a JSON number the way the TS template literal would: whole numbers
/// without a trailing `.0`, fractional numbers verbatim.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Strip the `spec/features/` prefix and `.feature` suffix to recover the
/// short feature name used by `fspec show-coverage`. Mirrors the TS
/// `.replace(/^spec\/features\//, '').replace(/\.feature$/, '')`.
fn strip_feature_name(file: &str) -> String {
    let mut s = file;
    if let Some(rest) = s.strip_prefix("spec/features/") {
        s = rest;
    }
    if let Some(rest) = s.strip_suffix(".feature") {
        s = rest;
    }
    s.to_string()
}

/// Detect a TS/JS file-extension import — equivalent to the TS regex
/// `/import .* from ['"].*\.(ts|js)['"]/`. We scan line by line for an
/// `import ... from '<...>.ts'`/`.js` (single OR double quotes).
fn has_extension_import(content: &str) -> bool {
    for line in content.lines() {
        let Some(import_idx) = line.find("import ") else {
            continue;
        };
        let after_import = &line[import_idx..];
        let Some(from_idx) = after_import.find(" from ") else {
            continue;
        };
        let after_from = &after_import[from_idx + " from ".len()..];
        let quote = match after_from.chars().next() {
            Some(c @ ('\'' | '"')) => c,
            _ => continue,
        };
        let rest = &after_from[1..];
        let Some(end) = rest.find(quote) else {
            continue;
        };
        let spec = &rest[..end];
        if spec.ends_with(".ts") || spec.ends_with(".js") {
            return true;
        }
    }
    false
}

/// Local linked-feature lookup (does NOT touch show_work_unit.rs). Returns the
/// feature files that reference the work-unit id via feature- or scenario-level
/// tags. Silently degrades to an empty Vec on any error (TS bare catch).
fn scan_linked_features(project_root: &Path, work_unit_id: &str) -> Vec<LinkedFeature> {
    let files = match glob_feature_files(project_root) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<LinkedFeature> = Vec::new();
    for rel in files {
        let abs = project_root.join(&rel);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let feature = match crate::io::gherkin::parse_feature_lenient(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if feature_references_work_unit(&feature, work_unit_id) {
            out.push(LinkedFeature { file: rel });
        }
    }
    out
}

/// True when the feature links to `work_unit_id` with a NON-EMPTY projected
/// scenario list — faithfully mirroring `extractWorkUnitTags` +
/// `showWorkUnit().linkedFeatures` (`src/utils/work-unit-tags.ts:46-124`,
/// `src/commands/show-work-unit.ts:117`).
///
/// TS projects scenarios per work-unit id as follows, then links the feature
/// ONLY if the resulting list is non-empty:
///   * feature-level tag → every TOP-LEVEL scenario that carries NO work-unit
///     tag of its own (a scenario with ANY valid `PREFIX-NNN` tag — even a
///     DIFFERENT id — is excluded from the feature-level projection);
///   * scenario-level tag → exactly the scenarios carrying that id.
///
/// Rule-nested scenarios are ignored (TS `work-unit-tags.ts:68` only collects
/// `child.scenario`, never `child.rule`), so we walk `feature.scenarios` only.
fn feature_references_work_unit(feature: &Feature, work_unit_id: &str) -> bool {
    let feature_level = feature
        .tags
        .iter()
        .filter_map(|t| extract_work_unit_id(t))
        .any(|id| id == work_unit_id);

    if feature_level {
        // Feature-level tag: a scenario counts only when it has NO work-unit
        // tag of its own (matching the `scenarioWorkUnits.length === 0` gate).
        let has_unclaimed_scenario = feature.scenarios.iter().any(|s| {
            !s.tags
                .iter()
                .any(|t| extract_work_unit_id(t).is_some())
        });
        if has_unclaimed_scenario {
            return true;
        }
    }

    // Scenario-level tag: link iff some scenario carries this exact id.
    feature.scenarios.iter().any(|s| {
        s.tags
            .iter()
            .filter_map(|t| extract_work_unit_id(t))
            .any(|id| id == work_unit_id)
    })
}

/// Extract a `PREFIX-NNN` ID from a tag (uppercase prefix 2–6 chars). Accepts
/// both `@PREFIX-NNN` and bare `PREFIX-NNN`.
fn extract_work_unit_id(tag: &str) -> Option<String> {
    let stripped = tag.strip_prefix('@').unwrap_or(tag);
    let (prefix, num) = stripped.split_once('-')?;
    if prefix.len() < 2 || prefix.len() > 6 {
        return None;
    }
    if !prefix.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(stripped.to_string())
}

/// Port of `buildAIAnalysisReminder`. Builds the AI deep-review reminder body
/// (BEFORE agent-formatting). Includes ACDD recommendations, optional impl-file
/// reading instructions, and the static analysis checklist.
fn build_ai_analysis_reminder(
    work_unit_id: &str,
    title: &str,
    coverage_data: Option<&Value>,
    recommendations: &[Recommendation],
) -> String {
    let mut lines: Vec<String> = Vec::new();

    if !recommendations.is_empty() {
        lines.push("ACDD COMPLIANCE REVIEW".to_string());
        lines.push(String::new());
        for (index, rec) in recommendations.iter().enumerate() {
            lines.push(format!("{}. **Recommendation:** {}", index + 1, rec.recommendation));
            lines.push(format!("   - **Rationale:** {}", rec.rationale));
            lines.push(format!("   - **Action:** {}", rec.action));
            lines.push(String::new());
        }
    }

    lines.push("AI-DRIVEN DEEP CODE REVIEW".to_string());
    lines.push(String::new());
    lines.push(format!("Work Unit: {work_unit_id} - {title}"));
    lines.push(String::new());

    // Collect all implementation files from coverage (insertion-ordered set).
    let mut impl_files: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    if let Some(scenarios) = coverage_data
        .and_then(|c| c.get("scenarios"))
        .and_then(Value::as_array)
    {
        for scenario in scenarios {
            if let Some(mappings) = scenario.get("testMappings").and_then(Value::as_array) {
                for mapping in mappings {
                    if let Some(impl_mappings) =
                        mapping.get("implMappings").and_then(Value::as_array)
                    {
                        for im in impl_mappings {
                            if let Some(file) = im.get("file").and_then(Value::as_str) {
                                if seen.insert(file.to_string()) {
                                    impl_files.push(file.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !impl_files.is_empty() {
        lines.push("STEP 1: Read Implementation Files".to_string());
        lines.push(String::new());
        lines.push("Use the Read tool to examine the following implementation files:".to_string());
        for file in &impl_files {
            lines.push(format!("  - {file}"));
        }
        lines.push(String::new());
    }

    lines.push("STEP 2: Analyze Code for Quality Issues".to_string());
    lines.push(String::new());
    lines.push("Perform deep analysis to analyze the code you read. Look for bugs:".to_string());
    lines.push(String::new());
    lines.push("  • Bugs and Logic Errors:".to_string());
    lines.push("    - Off-by-one errors, null pointer exceptions".to_string());
    lines.push("    - Incorrect edge case handling".to_string());
    lines.push("    - Logic flaws in conditionals or loops".to_string());
    lines.push(String::new());
    lines.push("  • Race Conditions:".to_string());
    lines.push("    - Async operations without proper locking".to_string());
    lines.push("    - File operations that could conflict".to_string());
    lines.push("    - Concurrent access to shared resources".to_string());
    lines.push(String::new());
    lines.push("  • Anti-Patterns:".to_string());
    lines.push("    - God functions (>100 lines, large functions that need refactoring)".to_string());
    lines.push("    - duplicated code across multiple files".to_string());
    lines.push("    - Tight coupling between modules".to_string());
    lines.push("    - Magic numbers without constants".to_string());
    lines.push(String::new());
    lines.push("  • Refactoring Opportunities:".to_string());
    lines.push("    - Similar code that could be extracted to shared utilities".to_string());
    lines.push("    - large functions that should be split".to_string());
    lines.push("    - Repeated validation logic that could be DRY".to_string());
    lines.push(String::new());

    lines.push("STEP 3: Check FOUNDATION.md Alignment".to_string());
    lines.push(String::new());
    lines.push("Read FOUNDATION.md or CLAUDE.md and verify code follows project principles:".to_string());
    lines.push("  - File size limits (e.g., keep files under 300 lines)".to_string());
    lines.push("  - Architectural patterns (e.g., use gitoxide NAPI-RS bindings not child_process)".to_string());
    lines.push("  - Coding standards (e.g., no any types, use ES6 imports)".to_string());
    lines.push("  - Project-specific conventions".to_string());
    lines.push(String::new());

    lines.push("STEP 4: Report Findings".to_string());
    lines.push(String::new());
    lines.push("After your analysis, report findings conversationally:".to_string());
    lines.push("  - List bugs found with file:line references".to_string());
    lines.push("  - Explain anti-patterns detected and why they're problematic".to_string());
    lines.push("  - Suggest specific refactoring with code examples if helpful".to_string());
    lines.push("  - Note FOUNDATION.md violations with exact principle violated".to_string());
    lines.push(String::new());
    lines.push("Example:".to_string());
    lines.push("  \"I found a potential race condition in src/file-ops/save.ts:15-20.".to_string());
    lines.push("   Two async writeFile calls happen without synchronization, which could".to_string());
    lines.push("   corrupt the file if both execute simultaneously. Consider using a".to_string());
    lines.push("   file locking pattern or atomic writes as mentioned in FOUNDATION.md.\"".to_string());
    lines.push(String::new());

    lines.push("NOTE: The static analysis above already caught basic issues (any types, etc.).".to_string());
    lines.push("Focus your analysis on deeper issues that require understanding context and logic.".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn parse(content: &str) -> Feature {
        crate::io::gherkin::parse_feature_lenient(content).expect("feature parses")
    }

    #[test]
    fn extract_work_unit_id_accepts_canonical_and_rejects_malformed() {
        assert_eq!(extract_work_unit_id("AUTH-001").as_deref(), Some("AUTH-001"));
        assert_eq!(extract_work_unit_id("@AUTH-001").as_deref(), Some("AUTH-001"));
        // Lowercase prefix rejected (TS pattern is [A-Z]{2,6}).
        assert_eq!(extract_work_unit_id("auth-001"), None);
        // Prefix too short / too long.
        assert_eq!(extract_work_unit_id("A-1"), None);
        assert_eq!(extract_work_unit_id("ABCDEFG-1"), None);
        // Non-digit suffix rejected.
        assert_eq!(extract_work_unit_id("AUTH-001x"), None);
        assert_eq!(extract_work_unit_id("AUTH-"), None);
    }

    #[test]
    fn feature_level_tag_with_scenarios_links() {
        let f = parse(
            "@AUTH-001\nFeature: F\n  Scenario: s\n    Given a\n    When b\n    Then c\n",
        );
        assert!(feature_references_work_unit(&f, "AUTH-001"));
    }

    #[test]
    fn feature_level_tag_with_no_scenarios_does_not_link() {
        // Parity with TS: matchingTag.scenarios.length === 0 → NOT linked.
        let f = parse("@AUTH-001\nFeature: F\n\n  Just a description, no scenarios.\n");
        assert!(!feature_references_work_unit(&f, "AUTH-001"));
    }

    #[test]
    fn feature_level_tag_excludes_scenarios_claimed_by_another_work_unit() {
        // The single scenario carries its OWN (different) work-unit tag, so the
        // feature-level projection for AUTH-001 is empty → NOT linked.
        let f = parse(
            "@AUTH-001\nFeature: F\n  @OTHER-002\n  Scenario: s\n    Given a\n    When b\n    Then c\n",
        );
        assert!(!feature_references_work_unit(&f, "AUTH-001"));
        // But OTHER-002 IS linked via the scenario-level tag.
        assert!(feature_references_work_unit(&f, "OTHER-002"));
    }

    #[test]
    fn scenario_level_tag_links_even_without_feature_tag() {
        let f = parse(
            "Feature: F\n  @AUTH-001\n  Scenario: s\n    Given a\n    When b\n    Then c\n",
        );
        assert!(feature_references_work_unit(&f, "AUTH-001"));
    }

    #[test]
    fn unrelated_work_unit_does_not_link() {
        let f = parse(
            "@AUTH-001\nFeature: F\n  Scenario: s\n    Given a\n    When b\n    Then c\n",
        );
        assert!(!feature_references_work_unit(&f, "NOPE-999"));
    }
}
