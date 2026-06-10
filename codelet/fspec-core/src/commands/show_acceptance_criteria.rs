//! `show-acceptance-criteria` — Rust port of
//! `src/commands/show-acceptance-criteria.ts` (RPC-299).
//!
//! Walks `spec/features/**/*.feature`, parses each feature, filters by
//! the supplied tag list (ALL tags must be present), extracts an
//! `FeatureAC` projection per feature, then renders the result as one of
//! `text` / `markdown` / `json`. Optionally writes the rendered body to
//! disk and produces a different success message.
//!
//! ## Envelope shape
//!
//! ```json
//! {
//!   "success": true,
//!   "features": [ { "name", "tags", "description", "background",
//!                   "scenarios": [{ "name", "steps": [{keyword,text}] }] }, ... ],
//!   "totalScenarios": <usize>,
//!   "message": "...",
//!   "output": "<rendered body>"
//! }
//! ```
//!
//! On structural error (missing `spec/features` directory) the function
//! returns `Err(FspecCoreError::Io { … })` whose Display contains the
//! canonical substring `"spec/features directory not found"`, and the
//! dispatcher surfaces this as `DispatchResult { success: false, error: Some(...) }`.
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge
//! both call this function — no inline rendering anywhere else.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowArgs {
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    output: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-acceptance-criteria",
            reason: format!("failed to parse args: {e}"),
        })?;

    let tags = args.tags.clone().unwrap_or_default();
    let format = args.format.as_deref().unwrap_or("text").to_string();

    // Missing spec/features → structured Err so the dispatcher surfaces
    // `DispatchResult { success: false, error: Some("...spec/features directory not found...") }`.
    // The TS `showAcceptanceCriteria` returns `{ success:false, error:'spec/features directory not found' }`;
    // in Rust the dispatcher owns the success-flag mapping, so we escalate
    // via `FspecCoreError::Io { … }` whose Display contains the canonical
    // substring asserted by `rust_port_missing_spec_features_directory_returns_structured_error`.
    let features_dir = project_root.join("spec").join("features");
    if !features_dir.exists() {
        return Err(FspecCoreError::Io {
            command: "show-acceptance-criteria",
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "spec/features directory not found",
            ),
        });
    }

    let files = glob_feature_files(project_root).unwrap_or_default();

    if files.is_empty() {
        let envelope = json!({
            "success": true,
            "features": Vec::<Value>::new(),
            "totalScenarios": 0,
            "message": "No feature files found in spec/features/",
            "output": "",
        });
        return serialize(&envelope);
    }

    let mut features: Vec<FeatureAc> = Vec::new();
    let mut total_scenarios: usize = 0;

    for rel in &files {
        let abs = project_root.join(rel);
        let Ok(content) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(feature) = parse_feature_lenient(&content) else {
            continue;
        };

        // Feature tags (gherkin strips leading '@'; restore for compare).
        let feature_tags: Vec<String> = feature
            .tags
            .iter()
            .map(|t| format!("@{t}"))
            .collect();

        // Filter: ALL requested tags must be present.
        if !tags.is_empty() {
            let matches_all = tags.iter().all(|t| feature_tags.contains(t));
            if !matches_all {
                continue;
            }
        }

        // ── Feature description (verbatim from source) ──
        //
        // The Rust `gherkin` crate aggressively strips leading whitespace
        // from description lines (see parser.rs `_ n:not_nl()`), whereas
        // the TS `@cucumber/gherkin` parser preserves verbatim source
        // indentation. To achieve byte-for-byte parity with the TS
        // behaviour we re-extract the description from the raw source
        // between the line AFTER `Feature: ...` and BEFORE the first
        // `Background:` or `Scenario:` / `Scenario Outline:` /
        // `Rule:` keyword line, trimming trailing blank lines (TS
        // semantics: `d.trim() === ""` ⇒ None, otherwise verbatim).
        let description = extract_description_verbatim(
            &content,
            feature.position.line,
            feature
                .background
                .as_ref()
                .map(|bg| bg.position.line)
                .or_else(|| feature.scenarios.first().map(|s| s.position.line))
                .or_else(|| feature.rules.first().map(|r| r.position.line)),
        );

        // Background.
        let background = feature.background.as_ref().map(|bg| {
            let steps = bg
                .steps
                .iter()
                .map(|s| format!("{}{}", s.keyword, s.value))
                .collect::<Vec<_>>()
                .join("\n");
            // Mirrors TS conditional concatenation when description/name set.
            // TS background description is verbatim from source — re-extract
            // for parity with the @cucumber/gherkin behaviour.
            let bg_desc_verbatim = extract_description_verbatim(
                &content,
                bg.position.line,
                bg.steps.first().map(|s| s.position.line).or_else(|| {
                    feature.scenarios.first().map(|s| s.position.line)
                }),
            );
            let has_desc = bg_desc_verbatim.is_some();
            let has_name = !bg.name.is_empty();
            if has_desc || has_name {
                let desc = bg_desc_verbatim.unwrap_or_default();
                format!("{}\n{}\n{}", bg.name, desc, steps)
            } else {
                steps
            }
        });

        // Scenarios (skip Scenario Outline by checking keyword string).
        let mut scenarios: Vec<ScenarioAc> = Vec::new();
        for s in &feature.scenarios {
            let kw = s.keyword.trim();
            if kw != "Scenario" {
                continue;
            }
            let steps: Vec<StepAc> = s
                .steps
                .iter()
                .map(|st| StepAc {
                    keyword: st.keyword.trim().to_string(),
                    text: st.value.clone(),
                })
                .collect();
            scenarios.push(ScenarioAc {
                name: s.name.clone(),
                steps,
            });
        }
        total_scenarios += scenarios.len();

        features.push(FeatureAc {
            name: feature.name.clone(),
            tags: feature_tags,
            description,
            background,
            scenarios,
        });
    }

    // Compose message.
    let mut message = if features.is_empty() && !tags.is_empty() {
        format!("No features found matching tags: {}", tags.join(", "))
    } else if features.is_empty() {
        "No features found".to_string()
    } else if !tags.is_empty() {
        format!(
            "Showing acceptance criteria for {} {} from {} {} matching tags: {}",
            total_scenarios,
            pluralize("scenario", total_scenarios),
            features.len(),
            pluralize("feature", features.len()),
            tags.join(", ")
        )
    } else {
        format!(
            "Showing acceptance criteria for {} {} from {} {}",
            total_scenarios,
            pluralize("scenario", total_scenarios),
            features.len(),
            pluralize("feature", features.len())
        )
    };

    // Render output.
    let body = match format.as_str() {
        "markdown" => render_markdown(&features),
        "json" => render_json(&features),
        _ => render_text(&features),
    };

    // Write to file if requested.
    if let Some(out_rel) = args.output.as_deref() {
        let out_abs = project_root.join(out_rel);
        if let Some(parent) = out_abs.parent() {
            std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
                command: "show-acceptance-criteria",
                source,
            })?;
        }
        std::fs::write(&out_abs, body.as_bytes()).map_err(|source| FspecCoreError::Io {
            command: "show-acceptance-criteria",
            source,
        })?;
        // Replace message with "written to <basename>".
        let basename = out_rel.rsplit('/').next().unwrap_or(out_rel);
        message = format!("Acceptance criteria written to {basename}");
    }

    let envelope = json!({
        "success": true,
        "features": features_to_json(&features),
        "totalScenarios": total_scenarios,
        "message": message,
        "output": body,
    });
    serialize(&envelope)
}

// ─── Projection types ───

#[derive(Debug, Clone)]
struct FeatureAc {
    name: String,
    tags: Vec<String>,
    description: Option<String>,
    background: Option<String>,
    scenarios: Vec<ScenarioAc>,
}

#[derive(Debug, Clone)]
struct ScenarioAc {
    name: String,
    steps: Vec<StepAc>,
}

#[derive(Debug, Clone)]
struct StepAc {
    keyword: String,
    text: String,
}

// ─── Renderers ───

fn render_markdown(features: &[FeatureAc]) -> String {
    let mut md = String::new();
    for f in features {
        md.push_str(&format!("# {}\n\n", f.name));
        if !f.tags.is_empty() {
            md.push_str(&format!("**Tags:** {}\n\n", f.tags.join(" ")));
        }
        if let Some(desc) = &f.description {
            md.push_str(&format!("{desc}\n\n"));
        }
        if let Some(bg) = &f.background {
            let bq = bg.split('\n').collect::<Vec<_>>().join("\n> ");
            md.push_str(&format!("> **Background:**\n> {bq}\n\n"));
        }
        for s in &f.scenarios {
            md.push_str(&format!("## {}\n\n", s.name));
            for st in &s.steps {
                md.push_str(&format!("- **{}** {}\n", st.keyword, st.text));
            }
            md.push('\n');
        }
        if f.scenarios.is_empty() {
            md.push_str("_No scenarios defined_\n\n");
        }
        md.push_str("---\n\n");
    }
    md
}

fn render_text(features: &[FeatureAc]) -> String {
    // Non-TTY identity — no ANSI colours.
    let mut text = String::new();
    for f in features {
        text.push_str(&format!("\n{}\n", f.name));
        let bar = "\u{2500}".repeat(f.name.chars().count());
        text.push_str(&format!("{bar}\n"));
        if !f.tags.is_empty() {
            text.push_str(&format!("Tags: {}\n", f.tags.join(" ")));
        }
        if let Some(desc) = &f.description {
            text.push_str(&format!("\n{desc}\n"));
        }
        if let Some(bg) = &f.background {
            text.push_str(&format!("\nBackground:\n{bg}\n"));
        }
        for s in &f.scenarios {
            text.push_str(&format!("\n  Scenario: {}\n", s.name));
            for st in &s.steps {
                text.push_str(&format!("    {} {}\n", st.keyword, st.text));
            }
        }
        if f.scenarios.is_empty() {
            text.push_str("\n  No scenarios defined\n");
        }
        text.push('\n');
    }
    text
}

fn render_json(features: &[FeatureAc]) -> String {
    let v = features_to_json(features);
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

fn features_to_json(features: &[FeatureAc]) -> Value {
    Value::Array(
        features
            .iter()
            .map(|f| {
                // Field order mirrors TS construction in
                // `src/commands/show-acceptance-criteria.ts`:
                //   1. `name`           (set on construction)
                //   2. `tags`           (set on construction)
                //   3. `description`    (set on construction; omitted when undefined)
                //   4. `scenarios`      (set on construction; populated later)
                //   5. `background`     (assigned LAST inside the children loop)
                // `serde_json` with `preserve_order` honours declaration
                // order, so we mirror that exact insertion order here.
                let mut obj = serde_json::Map::new();
                obj.insert("name".to_string(), json!(f.name));
                obj.insert("tags".to_string(), json!(f.tags));
                if let Some(d) = &f.description {
                    obj.insert("description".to_string(), json!(d));
                }
                obj.insert(
                    "scenarios".to_string(),
                    Value::Array(
                        f.scenarios
                            .iter()
                            .map(|s| {
                                json!({
                                    "name": s.name,
                                    "steps": s.steps.iter().map(|st| json!({
                                        "keyword": st.keyword,
                                        "text": st.text,
                                    })).collect::<Vec<_>>(),
                                })
                            })
                            .collect::<Vec<_>>(),
                    ),
                );
                if let Some(bg) = &f.background {
                    obj.insert("background".to_string(), json!(bg));
                }
                Value::Object(obj)
            })
            .collect(),
    )
}

fn pluralize(word: &str, n: usize) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

fn serialize(v: &Value) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(v).map_err(|e| FspecCoreError::InvalidArgs {
        command: "show-acceptance-criteria",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Extract a description block verbatim from the raw source between the
/// directive (Feature/Background/Scenario) keyword line and the next
/// structural keyword line (Background/Scenario/Scenario Outline/Rule/
/// step) — preserving source indentation and trimming trailing blank
/// lines to match the @cucumber/gherkin JS parser behaviour.
///
/// `start_line` is the 1-based line number of the directive keyword
/// (`Feature:`, `Background:`, etc.). The description begins on the
/// NEXT line. `end_line_exclusive` is the 1-based line of the first
/// child element (Background, Scenario, or Step) — when `None`, the
/// whole rest of the file is considered.
///
/// The walk terminates at the FIRST line whose trimmed content begins
/// with `#` (comment) — comments split description blocks in Gherkin
/// (a comment ends the current description, and content after the
/// comment block is not re-attached to the same description). This
/// matches both the Rust gherkin crate and the @cucumber/gherkin JS
/// behaviour.
fn extract_description_verbatim(
    content: &str,
    start_line: usize,
    end_line_exclusive: Option<usize>,
) -> Option<String> {
    let lines: Vec<&str> = content.split('\n').collect();
    // 0-based start: skip past the directive keyword line.
    let start_idx = start_line; // 1-based directive line, +1 for next line → start_line in 0-based.
    let end_idx_exclusive = end_line_exclusive
        .map(|n| n.saturating_sub(1))
        .unwrap_or(lines.len());
    if start_idx >= end_idx_exclusive || start_idx >= lines.len() {
        return None;
    }
    let slice = &lines[start_idx..end_idx_exclusive.min(lines.len())];

    // Strip leading blank lines (TS parser `_` consumer skips leading
    // whitespace before the first description line).
    let mut start = 0usize;
    while start < slice.len() && slice[start].chars().all(|c| c.is_whitespace()) {
        start += 1;
    }
    // Walk forward and stop at the first comment line OR tag line —
    // both terminate description blocks in both Gherkin parsers.
    let mut end = start;
    while end < slice.len() {
        let trimmed = slice[end].trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with('@') {
            break;
        }
        end += 1;
    }
    // Strip trailing blank lines (parity with TS parser's `__`
    // consumer which trims trailing whitespace/newlines).
    while end > start && slice[end - 1].chars().all(|c| c.is_whitespace()) {
        end -= 1;
    }
    if start >= end {
        return None;
    }
    let joined = slice[start..end].join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: ShowArgs = serde_json::from_str("{}").unwrap();
        assert!(a.tags.is_none());
        assert!(a.format.is_none());
        assert!(a.output.is_none());
    }

    #[test]
    fn args_parse_full() {
        let a: ShowArgs = serde_json::from_str(
            r#"{"tags":["@a","@b"],"format":"markdown","output":"out.md"}"#,
        )
        .unwrap();
        assert_eq!(a.tags.as_deref().unwrap().len(), 2);
        assert_eq!(a.format.as_deref(), Some("markdown"));
        assert_eq!(a.output.as_deref(), Some("out.md"));
    }

    #[test]
    fn pluralize_one_vs_many() {
        assert_eq!(pluralize("scenario", 1), "scenario");
        assert_eq!(pluralize("scenario", 0), "scenarios");
        assert_eq!(pluralize("scenario", 2), "scenarios");
    }

    #[test]
    fn markdown_no_scenarios_marker() {
        let f = vec![FeatureAc {
            name: "X".to_string(),
            tags: vec!["@a".to_string()],
            description: None,
            background: None,
            scenarios: vec![],
        }];
        let out = render_markdown(&f);
        assert!(out.contains("_No scenarios defined_"));
    }
}
