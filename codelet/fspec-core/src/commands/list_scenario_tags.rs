//! `list-scenario-tags` — Rust port of `src/commands/list-scenario-tags.ts` (RPC-249).
//!
//! Reads one `.feature` file (path supplied via `args.file`), locates a
//! top-level Scenario by exact-name match (`args.scenario`), and returns
//! the set of `@tag` lines that immediately precede that Scenario. With
//! `args.showCategories=true` the result is enriched with category labels
//! resolved from the project tag registry (`spec/tags.json`).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## TS-parity error envelope
//!
//! ALL recoverable errors are surfaced inside the `{success, tags,
//! message?, error?, categorizedTags?}` payload (parity with the TS
//! `ListScenarioTagsResult` shape). The outer `Result<String,
//! FspecCoreError>` envelope is only used for arg-parse failures —
//! matching the canonical `list_feature_tags.rs` (RPC-244) pattern.
//!
//! Behaviour parity with TypeScript (`src/commands/list-scenario-tags.ts`):
//!
//! * ENOENT on the feature file → `{success:false, tags:[],
//!   error:"File not found: <path>"}`.
//! * Other I/O errors → `{success:false, tags:[],
//!   error:"Failed to read <path>: <io-error>"}` (TS `throw`s here —
//!   we route through the inner payload for symmetry).
//! * Gherkin parser rejection → `{success:false, tags:[],
//!   error:"Invalid Gherkin syntax: Parser errors:\n(line:col):
//!   expected: ..., got '<token>'"}` (parity with TS
//!   `Gherkin.Parser.parse` throw message).
//! * Parser succeeds but no Feature element → `{success:false,
//!   tags:[], error:"File does not contain a valid Feature"}`.
//! * Scenario name not found among top-level `Scenario:` children →
//!   `{success:false, tags:[], error:"Scenario '<name>' not found
//!   in <path>"}`. Background, Examples, Scenario Outline, and
//!   Rule-nested Scenarios are NOT searched.
//! * Scenario found with zero tags → `{success:true, tags:[],
//!   message:"No tags found on this scenario"}`.
//! * Scenario found with tags → `{success:true, tags:["@t1","@t2"]}`
//!   with the leading `@` preserved verbatim from the source.
//! * `showCategories=true` with valid registry → `{success:true,
//!   tags, categorizedTags:[{tag, category}, ...]}` (unknown tags →
//!   `"Unknown"`).
//! * `showCategories=true` with missing/invalid registry → silent
//!   degrade to `{success:true, tags}` (no `categorizedTags` field).

use std::path::Path;

use crate::io::gherkin::parse_feature_lenient;
use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;

/// CLI arguments accepted by `list-scenario-tags`. Mirrors the TS
/// `ListScenarioTagsOptions` interface at
/// `src/commands/list-scenario-tags.ts:9-12`, plus the dispatcher-only
/// `format` selector also exposed by every other ported listing command.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListScenarioTagsArgs {
    /// Feature file path RELATIVE to `project_root`.
    #[serde(default)]
    file: Option<String>,
    /// Scenario name to match (exact, case-sensitive).
    #[serde(default)]
    scenario: Option<String>,
    /// When true, enrich the result with category labels from the registry.
    #[serde(default)]
    show_categories: bool,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// One entry in the `categorizedTags` array.
#[derive(Debug, Serialize)]
struct CategorizedTag {
    tag: String,
    category: String,
}

/// Canonical TS-parity response shape — matches the TS
/// `ListScenarioTagsResult` interface verbatim. `success/tags` always
/// present; the optional fields are omitted via
/// `skip_serializing_if = "Option::is_none"` so the JSON shape is
/// identical to the TS `JSON.stringify` output.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListScenarioTagsResult {
    success: bool,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    categorized_tags: Option<Vec<CategorizedTag>>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListScenarioTagsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-scenario-tags",
            reason: format!("failed to parse args: {e}"),
        })?;

    let file = args.file.clone().ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "list-scenario-tags",
        reason: "missing required 'file' argument".to_string(),
    })?;
    let scenario_name = args.scenario.clone().ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "list-scenario-tags",
        reason: "missing required 'scenario' argument".to_string(),
    })?;

    let result = load_scenario_tags(project_root, &file, &scenario_name, args.show_categories);

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-scenario-tags",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text.
        _ => Ok(render_text(&scenario_name, &result)),
    }
}

/// Top-level orchestration of the read/parse/match/categorize pipeline.
/// Errors are surfaced as `{success:false, error:"..."}` inside the
/// returned [`ListScenarioTagsResult`] — never escalated as a
/// `FspecCoreError`. This matches the TS shape exactly.
fn load_scenario_tags(
    project_root: &Path,
    file_rel: &str,
    scenario_name: &str,
    show_categories: bool,
) -> ListScenarioTagsResult {
    let abs = project_root.join(file_rel);

    let content = match std::fs::read_to_string(&abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return err_payload(format!("File not found: {file_rel}"));
        }
        Err(e) => {
            return err_payload(format!("Failed to read {file_rel}: {e}"));
        }
    };

    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(parse_err) => {
            return err_payload(format!(
                "Invalid Gherkin syntax: Parser errors:\n{parse_err}"
            ));
        }
    };

    // Parser yields a Feature struct, but if the source did not include
    // a `Feature:` keyword the parser would have rejected it above.
    // Defensive: when keyword is empty treat as missing-Feature parity.
    if feature.keyword.is_empty() && feature.name.is_empty() && feature.scenarios.is_empty() {
        return err_payload("File does not contain a valid Feature".to_string());
    }

    // Only top-level `Scenario:` children are searched — TS filter
    // `child.scenario.keyword === 'Scenario'`. The `gherkin` crate
    // stores Background separately, Scenario Outline under the same
    // `Scenario` struct but with `keyword` set to "Scenario Outline",
    // and Rule-nested scenarios under `feature.rules[*].scenarios`.
    let target = feature
        .scenarios
        .iter()
        .find(|s| s.keyword.trim() == "Scenario" && s.name == scenario_name);

    let target = match target {
        Some(s) => s,
        None => {
            return err_payload(format!(
                "Scenario '{scenario_name}' not found in {file_rel}"
            ));
        }
    };

    let tags: Vec<String> = target.tags.iter().map(|t| format!("@{t}")).collect();

    if tags.is_empty() {
        return ListScenarioTagsResult {
            success: true,
            tags: Vec::new(),
            message: Some("No tags found on this scenario".to_string()),
            error: None,
            categorized_tags: None,
        };
    }

    if show_categories {
        if let Some(cats) = try_categorize(project_root, &tags) {
            return ListScenarioTagsResult {
                success: true,
                tags,
                message: None,
                error: None,
                categorized_tags: Some(cats),
            };
        }
        // Silent degradation — parity with TS bare catch at
        // `src/commands/list-scenario-tags.ts:121-127`.
    }

    ListScenarioTagsResult {
        success: true,
        tags,
        message: None,
        error: None,
        categorized_tags: None,
    }
}

fn err_payload(error: String) -> ListScenarioTagsResult {
    ListScenarioTagsResult {
        success: false,
        tags: Vec::new(),
        message: None,
        error: Some(error),
        categorized_tags: None,
    }
}

/// Build the `categorizedTags` projection from `spec/tags.json`.
/// Returns `None` if the file is missing, unreadable, or malformed —
/// the caller silently degrades. Unknown tags map to `"Unknown"`.
fn try_categorize(project_root: &Path, tags: &[String]) -> Option<Vec<CategorizedTag>> {
    let path = project_root.join("spec").join("tags.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let parsed: TagsRegistry = serde_json::from_str(&raw).ok()?;

    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for category in &parsed.categories {
        for tag in &category.tags {
            map.entry(tag.name.clone())
                .or_insert_with(|| category.name.clone());
        }
    }

    Some(
        tags.iter()
            .map(|t| CategorizedTag {
                tag: t.clone(),
                category: map.get(t).cloned().unwrap_or_else(|| "Unknown".to_string()),
            })
            .collect(),
    )
}

/// Deserialised projection of the project tag registry.
#[derive(Debug, Deserialize)]
struct TagsRegistry {
    #[serde(default)]
    categories: Vec<TagsCategory>,
}

#[derive(Debug, Deserialize)]
struct TagsCategory {
    name: String,
    #[serde(default)]
    tags: Vec<TagEntry>,
}

#[derive(Debug, Deserialize)]
struct TagEntry {
    name: String,
}

/// Render the human-readable text output from a JSON payload (as
/// produced by `run` with `format=json`). Public so the CLI bridge
/// can re-use the canonical TS rendering without duplicating the
/// `"Tags on scenario '<name>':"` literal in the shell-facing crate.
///
/// `scenario_name` is the value the user supplied at the command-line
/// (parity with TS `output.log(`Tags on scenario '${scenarioName}':`)`).
///
/// The `value` argument MUST be a parsed `serde_json::Value` of the
/// payload returned by `run(..., format=json)`. Missing/extra fields
/// are tolerated; the rendering degrades gracefully.
pub fn render_text_from_json(scenario_name: &str, value: &serde_json::Value) -> String {
    if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
        return err.to_string();
    }
    if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        return msg.to_string();
    }

    let mut out = format!("Tags on scenario '{scenario_name}':\n\n");

    if let Some(cats) = value.get("categorizedTags").and_then(|v| v.as_array()) {
        out.push_str(&format!("{:<20} {}\n", "Tag", "Category"));
        out.push_str(&format!("{}\n", "─".repeat(50)));
        for entry in cats {
            let tag = entry.get("tag").and_then(|v| v.as_str()).unwrap_or("");
            let cat = entry.get("category").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!("{tag:<20} {cat}\n"));
        }
    } else if let Some(tags) = value.get("tags").and_then(|v| v.as_array()) {
        for t in tags {
            if let Some(s) = t.as_str() {
                out.push_str(&format!("  {s}\n"));
            }
        }
    }
    out
}

/// Render the text output format documented in the TS CLI wrapper
/// (`src/commands/list-scenario-tags.ts:148-180`).
///
/// On error/message path the canonical reason is returned verbatim so
/// the CLI bridge can inspect it; on the categorized-tags path the
/// table uses ASCII '-' x 50 separator (intentional divergence from
/// TS Unicode '─', kept for shell-pipeline / monospace safety).
fn render_text(scenario_name: &str, result: &ListScenarioTagsResult) -> String {
    if let Some(err) = &result.error {
        return err.clone();
    }
    if let Some(msg) = &result.message {
        return msg.clone();
    }

    let mut out = format!("Tags on scenario '{scenario_name}':\n\n");

    if let Some(categorized) = &result.categorized_tags {
        out.push_str(&format!("{:<20} {}\n", "Tag", "Category"));
        out.push_str(&format!("{}\n", "─".repeat(50)));
        for entry in categorized {
            out.push_str(&format!("{:<20} {}\n", entry.tag, entry.category));
        }
    } else {
        for tag in &result.tags {
            out.push_str(&format!("  {tag}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ListScenarioTagsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.file.is_none());
        assert!(a.scenario.is_none());
        assert!(!a.show_categories);
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case_show_categories() {
        let a: ListScenarioTagsArgs = serde_json::from_str(
            r#"{"file":"f.feature","scenario":"S","showCategories":true,"format":"json"}"#,
        )
        .unwrap();
        assert_eq!(a.file.as_deref(), Some("f.feature"));
        assert_eq!(a.scenario.as_deref(), Some("S"));
        assert!(a.show_categories);
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn render_text_with_message_returns_message_verbatim() {
        let r = ListScenarioTagsResult {
            success: true,
            tags: vec![],
            message: Some("No tags found on this scenario".to_string()),
            error: None,
            categorized_tags: None,
        };
        assert_eq!(render_text("Some", &r), "No tags found on this scenario");
    }

    #[test]
    fn render_text_with_error_returns_error_verbatim() {
        let r = err_payload("File not found: x.feature".to_string());
        assert_eq!(render_text("Any", &r), "File not found: x.feature");
    }

    #[test]
    fn render_text_plain_lists_each_tag_indented() {
        let r = ListScenarioTagsResult {
            success: true,
            tags: vec!["@smoke".to_string(), "@critical".to_string()],
            message: None,
            error: None,
            categorized_tags: None,
        };
        let out = render_text("Login", &r);
        assert!(out.starts_with("Tags on scenario 'Login':\n"));
        assert!(out.lines().any(|l| l == "  @smoke"));
        assert!(out.lines().any(|l| l == "  @critical"));
    }
}
