//! `show-feature` — Rust port of `src/commands/show-feature.ts` (RPC-304).
//!
//! ## Deferred parity gap (B7, supervisor parity-fix orchestration)
//!
//! The TS implementation serializes the gherkin AST via
//! `@cucumber/messages` — i.e. the full GherkinDocument shape with `id`,
//! `location: {line, column}`, `keywordType`, `description`, `tags[]` with
//! `location` etc. The Rust port uses the `gherkin` crate's much-leaner
//! `Feature` AST and emits a minimal projection (`{keyword, name, tags:
//! [{name}], line, children}`).
//!
//! Bringing this to byte-for-byte parity requires either:
//!   1. Adding a `gherkin-messages` (or `cucumber-messages`) crate
//!      dependency, OR
//!   2. Hand-rolling the GherkinDocument shape from the existing
//!      `gherkin::Feature` AST (rebuilding `id` UUIDs, two-tuple
//!      `location: {line, column}` objects, etc.).
//!
//! Both options are several hundred lines of code and outside the scope
//! of the current parity-fix pass. The text-format parity (the higher-
//! value surface for CLI users) IS at byte-parity; the JSON shape gap
//! is documented here as a follow-up and tracked in the report.
//!
//! Resolves a feature reference (either a bare name or a direct
//! `*.feature` path), reads it, parses with the `gherkin` crate, extracts
//! `@PREFIX-NNN` work-unit tags, enriches with `spec/work-units.json`,
//! and renders either text (original source body + Work Units block) or
//! JSON (gherkin AST + workUnits array).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## Error envelope (TS-parity)
//!
//! All recoverable errors are surfaced inside the `{success, ...}`
//! payload (parity with `list_feature_tags`/`list_scenario_tags`). The
//! outer `Result<String, FspecCoreError>` envelope is only used for
//! `args_json` parse failures — matching the canonical pattern.
//!
//! ## Behaviour parity with TypeScript (`src/commands/show-feature.ts`)
//!
//! * Bare-name lookup (no `.feature` suffix) → glob `spec/features/**/*.feature`,
//!   match basename minus extension. First match wins (alphabetical via
//!   `glob_feature_files`).
//! * Direct path (ends in `.feature`) → resolve relative to project root.
//! * Missing file → `{success:false, error:"Feature file not found: <input>"}`.
//! * Gherkin parse failure → `{success:false, error:"Invalid Gherkin syntax: <msg>"}`.
//! * `format="text"` → file body + `\n\nWork Units:\n  ...` block OR
//!   `\n\nWork Units: None\n`.
//! * `format="json"` → 2-space pretty JSON `{feature, workUnits, ...}`.
//! * `output=<path>` → write rendered content to that path (project-root-relative).

use std::path::Path;

use gherkin::{Feature, GherkinEnv};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::types::work_unit::WorkUnitsData;

/// CLI / dispatcher arguments accepted by `show-feature`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowFeatureArgs {
    /// Feature name (e.g. `"login"`) or path ending in `.feature`.
    #[serde(default)]
    feature: Option<String>,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
    /// Optional output file path (project-root-relative).
    #[serde(default)]
    output: Option<String>,
}

/// Enriched work-unit projection rendered into both text and JSON output.
#[derive(Debug, Clone)]
struct WorkUnitEntry {
    id: String,
    title: String,
    status: String,
    level: WorkUnitLevel,
    scenarios: Vec<ScenarioRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkUnitLevel {
    Feature,
    Scenario,
}

impl WorkUnitLevel {
    fn label(self) -> &'static str {
        match self {
            WorkUnitLevel::Feature => "feature-level",
            WorkUnitLevel::Scenario => "scenario-level",
        }
    }
    fn json_str(self) -> &'static str {
        match self {
            WorkUnitLevel::Feature => "feature",
            WorkUnitLevel::Scenario => "scenario",
        }
    }
}

#[derive(Debug, Clone)]
struct ScenarioRef {
    name: String,
    line: usize,
}

/// Dispatcher entry point. Two-front-doors invariant: clap CLI bridge
/// and dispatcher both call this function.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowFeatureArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-feature",
            reason: format!("failed to parse args: {e}"),
        })?;

    let feature_input = args.feature.clone().ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "show-feature",
        reason: "missing required 'feature' argument".to_string(),
    })?;

    let format = args.format.as_deref().unwrap_or("text");

    let outcome = build_outcome(project_root, &feature_input);

    let rendered = match format {
        "json" => render_json(&outcome)?,
        _ => render_text(&outcome),
    };

    // If an output path is given AND we succeeded, write the rendered
    // content to disk under project_root (parity with TS `writeFile`).
    if outcome.success {
        if let Some(out_rel) = args.output.as_deref() {
            let out_abs = project_root.join(out_rel);
            if let Some(parent) = out_abs.parent() {
                std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
                    command: "show-feature",
                    source,
                })?;
            }
            std::fs::write(&out_abs, rendered.as_bytes()).map_err(|source| FspecCoreError::Io {
                command: "show-feature",
                source,
            })?;
        }
    }

    Ok(rendered)
}

/// Intermediate outcome state — either a successful render or a structured error.
struct Outcome {
    success: bool,
    /// On success: the original source bytes for text rendering.
    content: String,
    /// On success: the parsed Gherkin feature (for JSON rendering).
    feature: Option<Feature>,
    /// On success: the basename of the resolved file (e.g. `"auth.feature"`).
    file_basename: String,
    /// Aggregated work units (empty when no tags).
    work_units: Vec<WorkUnitEntry>,
    /// On failure: error message.
    error: Option<String>,
}

fn build_outcome(project_root: &Path, feature_input: &str) -> Outcome {
    let abs = match resolve_feature_path(project_root, feature_input) {
        Some(p) => p,
        None => {
            return Outcome {
                success: false,
                content: String::new(),
                feature: None,
                file_basename: String::new(),
                work_units: Vec::new(),
                error: Some(format!("Feature file not found: {feature_input}")),
            };
        }
    };

    let content = match std::fs::read_to_string(&abs) {
        Ok(s) => s,
        Err(_) => {
            return Outcome {
                success: false,
                content: String::new(),
                feature: None,
                file_basename: String::new(),
                work_units: Vec::new(),
                error: Some(format!("Feature file not found: {feature_input}")),
            };
        }
    };

    let feature = match Feature::parse(&content, GherkinEnv::default()) {
        Ok(f) => f,
        Err(e) => {
            return Outcome {
                success: false,
                content: String::new(),
                feature: None,
                file_basename: String::new(),
                work_units: Vec::new(),
                error: Some(format!("Invalid Gherkin syntax: {e}")),
            };
        }
    };

    let work_unit_tags = extract_work_unit_tags(&feature);
    let work_units_data = load_work_units_data(project_root);
    let work_units = enrich_work_unit_tags(work_unit_tags, work_units_data.as_ref());

    let basename = abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    Outcome {
        success: true,
        content,
        feature: Some(feature),
        file_basename: basename,
        work_units,
        error: None,
    }
}

/// Resolve a feature reference to an absolute on-disk path. Returns
/// `None` when the file does not exist.
fn resolve_feature_path(project_root: &Path, input: &str) -> Option<std::path::PathBuf> {
    if input.ends_with(".feature") {
        let p = project_root.join(input);
        if p.exists() {
            return Some(p);
        }
        return None;
    }
    // Bare-name lookup: glob spec/features/ and match basename.
    let files = glob_feature_files(project_root).ok()?;
    for rel in files {
        let basename = rel
            .rsplit('/')
            .next()
            .unwrap_or(&rel)
            .trim_end_matches(".feature");
        if basename == input {
            return Some(project_root.join(rel));
        }
    }
    None
}

/// One raw work-unit reference extracted from feature/scenario tags.
struct WorkUnitTag {
    id: String,
    level: WorkUnitLevel,
    scenarios: Vec<ScenarioRef>,
}

/// Extract work-unit tags from a parsed feature. Mirrors the TS
/// `extractWorkUnitTags` aggregation rules in `src/utils/work-unit-tags.ts`.
fn extract_work_unit_tags(feature: &Feature) -> Vec<WorkUnitTag> {
    // Insertion-ordered map to preserve first-seen order.
    let mut out: Vec<WorkUnitTag> = Vec::new();
    let mut index_of: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // Feature-level work-unit IDs.
    let feature_ids: Vec<String> = feature
        .tags
        .iter()
        .filter_map(|t| extract_work_unit_id(t))
        .collect();

    // For each feature-level WU, attach the scenarios that have NO scenario-level WU tag.
    for id in &feature_ids {
        let scenarios: Vec<ScenarioRef> = feature
            .scenarios
            .iter()
            .filter(|s| {
                let has_own = s.tags.iter().any(|t| extract_work_unit_id(t).is_some());
                !has_own
            })
            .map(|s| ScenarioRef {
                name: s.name.clone(),
                line: s.position.line,
            })
            .collect();

        index_of.insert(id.clone(), out.len());
        out.push(WorkUnitTag {
            id: id.clone(),
            level: WorkUnitLevel::Feature,
            scenarios,
        });
    }

    // Scenario-level WU tags.
    for scenario in &feature.scenarios {
        for tag in &scenario.tags {
            let Some(id) = extract_work_unit_id(tag) else {
                continue;
            };
            let entry_idx = match index_of.get(&id) {
                Some(&i) => i,
                None => {
                    let i = out.len();
                    index_of.insert(id.clone(), i);
                    out.push(WorkUnitTag {
                        id: id.clone(),
                        level: WorkUnitLevel::Scenario,
                        scenarios: Vec::new(),
                    });
                    i
                }
            };
            out[entry_idx].scenarios.push(ScenarioRef {
                name: scenario.name.clone(),
                line: scenario.position.line,
            });
            // Update level: scenario-level overrides feature-level.
            if out[entry_idx].level == WorkUnitLevel::Feature {
                out[entry_idx].level = WorkUnitLevel::Scenario;
            }
        }
    }

    out
}

/// Extract a `PREFIX-NNN` ID from a tag string. The `gherkin` crate strips
/// the leading `@`, but defensive handling accepts both forms.
fn extract_work_unit_id(tag: &str) -> Option<String> {
    let stripped = tag.strip_prefix('@').unwrap_or(tag);
    let (prefix, num) = stripped.split_once('-')?;
    if prefix.is_empty() || prefix.len() < 2 || prefix.len() > 6 {
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

/// Load `spec/work-units.json`. Returns `None` on any error (missing /
/// unreadable / malformed) — parity with the TS bare-catch path.
fn load_work_units_data(project_root: &Path) -> Option<WorkUnitsData> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Enrich raw work-unit tags with title/status from `spec/work-units.json`.
fn enrich_work_unit_tags(
    tags: Vec<WorkUnitTag>,
    data: Option<&WorkUnitsData>,
) -> Vec<WorkUnitEntry> {
    tags.into_iter()
        .map(|t| {
            let (title, status) = match data {
                Some(d) => d
                    .work_units
                    .get(&t.id)
                    .map(|wu| (wu.title.clone(), wu.status.as_str().to_string()))
                    .unwrap_or_else(|| ("Unknown".to_string(), "unknown".to_string())),
                None => ("Unknown".to_string(), "unknown".to_string()),
            };
            WorkUnitEntry {
                id: t.id,
                title,
                status,
                level: t.level,
                scenarios: t.scenarios,
            }
        })
        .collect()
}

/// Render the text output format. On success: original source + Work
/// Units block (with a trailing blank line for TS-parity: TS's
/// `output.log()` calls each append a `\n`, leaving the captured stream
/// with `\n\n` at EOF).
fn render_text(outcome: &Outcome) -> String {
    if !outcome.success {
        return outcome.error.clone().unwrap_or_default();
    }

    let mut out = outcome.content.clone();
    if outcome.work_units.is_empty() {
        out.push_str("\n\n");
        out.push_str("Work Units: None\n");
    } else {
        out.push_str("\n\n");
        out.push_str("Work Units:\n");
        for wu in &outcome.work_units {
            out.push_str(&format!("\n  {} ({}) - {}\n", wu.id, wu.level.label(), wu.title));
            for s in &wu.scenarios {
                out.push_str(&format!("    {}:{} - {}\n", outcome.file_basename, s.line, s.name));
            }
        }
    }
    // TS's final `output.log('')` (src/commands/show-feature.ts text branch)
    // appends an additional newline after the Work Units block. Add it
    // here so the captured byte stream ends with `\n\n` instead of `\n`.
    out.push('\n');
    out
}

/// Render the JSON output format. On success: 2-space pretty
/// `{feature, workUnits, ...}`. On error: `{success:false, error}`.
fn render_json(outcome: &Outcome) -> Result<String, FspecCoreError> {
    if !outcome.success {
        let v = json!({
            "success": false,
            "error": outcome.error.clone().unwrap_or_default(),
        });
        return serde_json::to_string_pretty(&v).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-feature",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    let feature = outcome
        .feature
        .as_ref()
        .expect("success branch must have parsed feature");

    let payload = json!({
        "success": true,
        "feature": feature_to_json(feature),
        "workUnits": outcome
            .work_units
            .iter()
            .map(work_unit_to_json)
            .collect::<Vec<_>>(),
    });

    serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "show-feature",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Project a parsed `gherkin::Feature` into a minimal JSON object whose
/// shape (`name`, `children[]`, `tags[]`) matches what TS callers of
/// the TS `show-feature` consume.
fn feature_to_json(feature: &Feature) -> Value {
    let mut children: Vec<Value> = Vec::new();
    if let Some(bg) = &feature.background {
        children.push(json!({
            "background": {
                "name": bg.name,
                "line": bg.position.line,
                "steps": bg.steps.iter().map(step_to_json).collect::<Vec<_>>(),
            }
        }));
    }
    for s in &feature.scenarios {
        children.push(json!({
            "scenario": scenario_to_json(s),
        }));
    }
    json!({
        "keyword": feature.keyword,
        "name": feature.name,
        "tags": feature
            .tags
            .iter()
            .map(|t| json!({"name": format!("@{t}")}))
            .collect::<Vec<_>>(),
        "line": feature.position.line,
        "children": children,
    })
}

fn scenario_to_json(s: &gherkin::Scenario) -> Value {
    json!({
        "keyword": s.keyword,
        "name": s.name,
        "line": s.position.line,
        "tags": s
            .tags
            .iter()
            .map(|t| json!({"name": format!("@{t}")}))
            .collect::<Vec<_>>(),
        "steps": s.steps.iter().map(step_to_json).collect::<Vec<_>>(),
    })
}

fn step_to_json(step: &gherkin::Step) -> Value {
    json!({
        "keyword": step.keyword,
        "text": step.value,
        "line": step.position.line,
    })
}

fn work_unit_to_json(wu: &WorkUnitEntry) -> Value {
    json!({
        "id": wu.id,
        "title": wu.title,
        "status": wu.status,
        "level": wu.level.json_str(),
        "scenarios": wu
            .scenarios
            .iter()
            .map(|s| json!({"name": s.name, "line": s.line}))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ShowFeatureArgs = serde_json::from_str("{}").unwrap();
        assert!(a.feature.is_none());
        assert!(a.format.is_none());
        assert!(a.output.is_none());
    }

    #[test]
    fn args_parse_camel_case_output_field() {
        let a: ShowFeatureArgs =
            serde_json::from_str(r#"{"feature":"login","format":"json","output":"out.txt"}"#)
                .unwrap();
        assert_eq!(a.feature.as_deref(), Some("login"));
        assert_eq!(a.format.as_deref(), Some("json"));
        assert_eq!(a.output.as_deref(), Some("out.txt"));
    }

    #[test]
    fn extract_work_unit_id_accepts_canonical_form() {
        assert_eq!(extract_work_unit_id("AUTH-001"), Some("AUTH-001".to_string()));
        assert_eq!(extract_work_unit_id("@AUTH-001"), Some("AUTH-001".to_string()));
        assert_eq!(extract_work_unit_id("RPC-304"), Some("RPC-304".to_string()));
    }

    #[test]
    fn extract_work_unit_id_rejects_non_canonical() {
        assert!(extract_work_unit_id("auth-001").is_none());
        assert!(extract_work_unit_id("AUTH001").is_none());
        assert!(extract_work_unit_id("AUTH-").is_none());
        assert!(extract_work_unit_id("@critical").is_none());
        assert!(extract_work_unit_id("A-1").is_none()); // prefix too short
    }
}
