//! `show-foundation` — Rust port of `src/commands/show-foundation.ts` (RPC-305).
//!
//! Resolves either the full foundation, a single named section, or a dotted
//! JSON path against `spec/foundation.json` (or `spec/foundation.json.draft`
//! when `draft=true`) and renders the result as plain text or pretty JSON.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## TS-parity rules
//!
//! * `draft=true` → read `spec/foundation.json.draft` directly; missing →
//!   `No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.`
//! * `draft=false` (default) → load-or-init `spec/foundation.json` via
//!   [`ensure_foundation_file`] (auto-creates canonical defaults).
//! * `section` resolution: first look up the literal name in `FIELD_MAP`
//!   (e.g. `projectName` → `project.name`), then split on `.` and walk the
//!   JSON object. Undefined at any step → `Field '<section>' not found`.
//! * `format="json"` → `JSON.stringify(displayData, null, 2)`.
//! * `format="text"` → if a section is supplied AND the value is a string,
//!   emit the raw string; otherwise pretty JSON. If no section, render the
//!   full foundation as a multi-section human-readable summary.
//! * `output=<path>` → write rendered content to that path (project-root
//!   relative) verbatim — the formatter does NOT append a newline.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_foundation_file;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowFoundationArgs {
    #[serde(default)]
    section: Option<String>,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
    /// Project-root-relative file path to write the rendered content to.
    #[serde(default)]
    output: Option<String>,
    /// When true, read `spec/foundation.json.draft` instead of
    /// `spec/foundation.json`.
    #[serde(default)]
    draft: bool,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowFoundationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    let format = args.format.as_deref().unwrap_or("text");

    // Load source — either draft or final foundation.
    let foundation = if args.draft {
        let draft_path = project_root.join("spec").join("foundation.json.draft");
        if !draft_path.exists() {
            return Err(FspecCoreError::InvalidArgs {
                command: "show-foundation",
                reason: "No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.".to_string(),
            });
        }
        let raw = std::fs::read_to_string(&draft_path).map_err(|source| FspecCoreError::Io {
            command: "show-foundation",
            source,
        })?;
        serde_json::from_str::<Value>(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "foundation.json.draft".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?
    } else {
        ensure_foundation_file(project_root)?
    };

    // Resolve section (if supplied) via FIELD_MAP → dotted path walk.
    let display_data: Value = match args.section.as_deref() {
        Some(section) => {
            let field_path = resolve_field_path(section);
            match get_nested_property(&foundation, &field_path) {
                Some(v) => v,
                None => {
                    return Err(FspecCoreError::InvalidArgs {
                        command: "show-foundation",
                        reason: format!("Field '{section}' not found"),
                    });
                }
            }
        }
        None => foundation.clone(),
    };

    // Format selection.
    let rendered = if format == "json" {
        serde_json::to_string_pretty(&display_data).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-foundation",
            reason: format!("failed to serialize result: {e}"),
        })?
    } else {
        // text format
        if args.section.is_some() {
            // For specific field, display as plain text.
            match &display_data {
                Value::String(s) => s.clone(),
                other => serde_json::to_string_pretty(other).map_err(|e| {
                    FspecCoreError::InvalidArgs {
                        command: "show-foundation",
                        reason: format!("failed to serialize result: {e}"),
                    }
                })?,
            }
        } else {
            // For entire foundation, render as human-readable multi-section text.
            format_foundation_as_text(&foundation)
        }
    };

    // Write to file if requested.
    if let Some(out_rel) = args.output.as_deref() {
        let out_abs = project_root.join(out_rel);
        if let Some(parent) = out_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
                command: "show-foundation",
                source,
            })?;
        }
        std::fs::write(&out_abs, rendered.as_bytes()).map_err(|source| FspecCoreError::Io {
            command: "show-foundation",
            source,
        })?;
    }

    Ok(rendered)
}

// ─────────────────────────────────────────────────────────────────────────
// FIELD_MAP — convenience aliases for nested paths
// ─────────────────────────────────────────────────────────────────────────

/// Mirrors the TS `FIELD_MAP` at `src/commands/show-foundation.ts:30-42`.
/// Returns the dotted path for the given section alias, or the section
/// itself if no alias exists.
fn resolve_field_path(section: &str) -> String {
    match section {
        "projectName" => "project.name".to_string(),
        "projectVision" => "project.vision".to_string(),
        "projectType" => "project.projectType".to_string(),
        "problemTitle" => "problemSpace.primaryProblem.title".to_string(),
        "problemDescription" => "problemSpace.primaryProblem.description".to_string(),
        "problemImpact" => "problemSpace.primaryProblem.impact".to_string(),
        "solutionOverview" => "solutionSpace.overview".to_string(),
        // Legacy aliases.
        "projectOverview" => "solutionSpace.overview".to_string(),
        "problemDefinition" => "problemSpace.primaryProblem.description".to_string(),
        other => other.to_string(),
    }
}

/// Walk a dotted JSON path. Returns `Some(Value::clone)` when present at every
/// step, `None` otherwise.
fn get_nested_property(root: &Value, path: &str) -> Option<Value> {
    let mut cur = root;
    for part in path.split('.') {
        match cur {
            Value::Object(map) => match map.get(part) {
                Some(v) => cur = v,
                None => return None,
            },
            _ => return None,
        }
    }
    Some(cur.clone())
}

// ─────────────────────────────────────────────────────────────────────────
// Text rendering of full foundation
// ─────────────────────────────────────────────────────────────────────────

/// Mirrors TS `formatFoundationAsText` at lines 150-213.
fn format_foundation_as_text(foundation: &Value) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("=== PROJECT ===".to_string());
    if let Some(project) = foundation.get("project").and_then(Value::as_object) {
        lines.push(format!(
            "Name: {}",
            project.get("name").and_then(Value::as_str).unwrap_or("N/A")
        ));
        lines.push(format!(
            "Vision: {}",
            project
                .get("vision")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
        ));
        lines.push(format!(
            "Type: {}",
            project
                .get("projectType")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
        ));
        if let Some(repo) = project.get("repository").and_then(Value::as_str) {
            lines.push(format!("Repository: {repo}"));
        }
        if let Some(lic) = project.get("license").and_then(Value::as_str) {
            lines.push(format!("License: {lic}"));
        }
    }
    lines.push(String::new());

    if let Some(problem) = foundation
        .get("problemSpace")
        .and_then(|v| v.get("primaryProblem"))
        .and_then(Value::as_object)
    {
        lines.push("=== PROBLEM SPACE ===".to_string());
        lines.push(format!(
            "Title: {}",
            problem
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
        ));
        lines.push(format!(
            "Description: {}",
            problem
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
        ));
        lines.push(format!(
            "Impact: {}",
            problem
                .get("impact")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
        ));
        lines.push(String::new());
    }

    if let Some(solution) = foundation.get("solutionSpace").and_then(Value::as_object) {
        lines.push("=== SOLUTION SPACE ===".to_string());
        lines.push(
            solution
                .get("overview")
                .and_then(Value::as_str)
                .unwrap_or("N/A")
                .to_string(),
        );
        lines.push(String::new());

        if let Some(caps) = solution.get("capabilities").and_then(Value::as_array) {
            if !caps.is_empty() {
                lines.push("Capabilities:".to_string());
                for cap in caps {
                    let name = cap.get("name").and_then(Value::as_str).unwrap_or("");
                    let desc = cap.get("description").and_then(Value::as_str).unwrap_or("");
                    lines.push(format!("- {name}: {desc}"));
                }
                lines.push(String::new());
            }
        }
    }

    if let Some(personas) = foundation.get("personas").and_then(Value::as_array) {
        if !personas.is_empty() {
            lines.push("=== PERSONAS ===".to_string());
            for persona in personas {
                let name = persona.get("name").and_then(Value::as_str).unwrap_or("");
                let desc = persona
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                lines.push(format!("- {name}: {desc}"));
            }
            lines.push(String::new());
        }
    }

    if let Some(diagrams) = foundation
        .get("architectureDiagrams")
        .and_then(Value::as_array)
    {
        if !diagrams.is_empty() {
            lines.push("=== ARCHITECTURE DIAGRAMS ===".to_string());
            for d in diagrams {
                let title = d.get("title").and_then(Value::as_str).unwrap_or("");
                lines.push(format!("- {title}"));
            }
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_field_path_canonical_aliases() {
        assert_eq!(resolve_field_path("projectName"), "project.name");
        assert_eq!(resolve_field_path("projectVision"), "project.vision");
        assert_eq!(resolve_field_path("projectType"), "project.projectType");
        assert_eq!(
            resolve_field_path("problemTitle"),
            "problemSpace.primaryProblem.title"
        );
        assert_eq!(
            resolve_field_path("solutionOverview"),
            "solutionSpace.overview"
        );
    }

    #[test]
    fn resolve_field_path_legacy_aliases() {
        assert_eq!(
            resolve_field_path("projectOverview"),
            "solutionSpace.overview"
        );
        assert_eq!(
            resolve_field_path("problemDefinition"),
            "problemSpace.primaryProblem.description"
        );
    }

    #[test]
    fn resolve_field_path_unmapped_passthrough() {
        assert_eq!(resolve_field_path("project.name"), "project.name");
        assert_eq!(resolve_field_path("nonexistent"), "nonexistent");
    }

    #[test]
    fn get_nested_property_returns_value_at_path() {
        let root = json!({"project": {"name": "fspec"}});
        let v = get_nested_property(&root, "project.name").unwrap();
        assert_eq!(v.as_str(), Some("fspec"));
    }

    #[test]
    fn get_nested_property_returns_none_for_missing_step() {
        let root = json!({"project": {"name": "fspec"}});
        assert!(get_nested_property(&root, "project.unknown").is_none());
        assert!(get_nested_property(&root, "nonexistent").is_none());
    }

    #[test]
    fn format_foundation_as_text_includes_project_section() {
        let f = json!({
            "project": {"name": "fspec", "vision": "V", "projectType": "cli-tool"}
        });
        let out = format_foundation_as_text(&f);
        assert!(out.lines().any(|l| l == "=== PROJECT ==="));
        assert!(out.lines().any(|l| l == "Name: fspec"));
        assert!(out.lines().any(|l| l == "Vision: V"));
        assert!(out.lines().any(|l| l == "Type: cli-tool"));
    }

    #[test]
    fn format_foundation_as_text_includes_problem_solution_personas() {
        let f = json!({
            "project": {"name": "fspec"},
            "problemSpace": {"primaryProblem": {"title": "P", "description": "D", "impact": "I"}},
            "solutionSpace": {"overview": "O", "capabilities": [{"name": "C", "description": "DC"}]},
            "personas": [{"name": "User", "description": "d"}]
        });
        let out = format_foundation_as_text(&f);
        assert!(out.lines().any(|l| l == "=== PROBLEM SPACE ==="));
        assert!(out.lines().any(|l| l == "=== SOLUTION SPACE ==="));
        assert!(out.lines().any(|l| l == "=== PERSONAS ==="));
    }
}
