//! `get-scenarios` — Rust port of `getScenarios` in
//! `src/commands/get-scenarios.ts` (RPC-237).
//!
//! Read-only command: walks `spec/features/**/*.feature`, extracts every
//! `Scenario` (NOT `Scenario Outline`), optionally filters by an AND-set of
//! tags evaluated against the UNION of feature-level and scenario-level tags,
//! and returns a JSON envelope:
//!
//! ```json
//! {
//!   "success": true,
//!   "scenarios": [ { "feature", "name", "line", "tags"? }, ... ],
//!   "totalCount": <usize>,
//!   "message": "...",
//!   "warnings"?: [ "..." ]
//! }
//! ```
//!
//! The envelope mirrors the `show-acceptance-criteria` (RPC-299) contract: the
//! dispatcher surfaces structured data to the LLM, while the standalone CLI
//! bridge renders the format-specific body (for `--format json` it prints ONLY
//! the `scenarios` array).
//!
//! ## Tag matching
//!
//! Mirrors the TS `allTags = [...new Set([...featureTags, ...scenarioTags])]`
//! union followed by `tags.every(tag => allTags.includes(tag))` AND-logic
//! (`src/commands/get-scenarios.ts:104-121`). Tags are compared WITH the
//! leading `@` (the gherkin crate strips it; we restore it via `@{t}`).
//!
//! ## Errors
//!
//! A missing `spec/features` directory surfaces as `FspecCoreError::Io` whose
//! Display contains the canonical substring `"spec/features directory not
//! found"` — the dispatcher maps this to `{ success: false, error: Some(...) }`
//! (parity with the TS early-return `{ success:false, error:'spec/features
//! directory not found' }`).
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge both
//! call this single function — no inline scenario extraction elsewhere.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GetScenariosArgs {
    #[serde(default)]
    tags: Option<Vec<String>>,
    // Accepted from the bridge for arg-shape parity, but never read by core:
    // `run` always returns the full JSON envelope and the CLI bridge performs
    // all format-specific rendering (see architecture note [1] / RPC-237).
    // Round-trip tests below exercise this field; silence the lib `dead_code` lint.
    #[serde(default)]
    #[allow(dead_code)]
    format: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GetScenariosArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "get-scenarios",
            reason: format!("failed to parse args: {e}"),
        })?;

    let tags = args.tags.unwrap_or_default();

    // Missing spec/features → structured Err carrying the canonical substring
    // so the dispatcher surfaces `{ success:false, error:Some("...") }`.
    let features_dir = project_root.join("spec").join("features");
    if !features_dir.exists() {
        return Err(FspecCoreError::Io {
            command: "get-scenarios",
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
            "scenarios": Vec::<Value>::new(),
            "totalCount": 0,
            "message": "No feature files found in spec/features/",
        });
        return serialize(&envelope);
    }

    let mut scenarios: Vec<Value> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for rel in &files {
        let abs = project_root.join(rel);
        let content = match std::fs::read_to_string(&abs) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!("Error reading {rel}: {e}"));
                continue;
            }
        };
        let Ok(feature) = parse_feature_lenient(&content) else {
            warnings.push(format!("Skipping invalid file: {rel}"));
            continue;
        };

        // Feature tags (gherkin strips leading '@'; restore for compare).
        let feature_tags: Vec<String> = feature.tags.iter().map(|t| format!("@{t}")).collect();

        for s in &feature.scenarios {
            // Skip Scenario Outline — only plain `Scenario` keyword.
            if s.keyword.trim() != "Scenario" {
                continue;
            }

            let scenario_tags: Vec<String> = s.tags.iter().map(|t| format!("@{t}")).collect();

            // AND-logic against the feature ∪ scenario tag union.
            if !tags.is_empty() {
                let matches_all = tags
                    .iter()
                    .all(|t| feature_tags.contains(t) || scenario_tags.contains(t));
                if !matches_all {
                    continue;
                }
            }

            // Field order mirrors the TS object literal: feature, name, line,
            // tags? (omitted when the scenario has no own tags).
            let mut obj = serde_json::Map::new();
            obj.insert("feature".to_string(), json!(rel));
            obj.insert("name".to_string(), json!(s.name));
            obj.insert("line".to_string(), json!(s.position.line));
            if !scenario_tags.is_empty() {
                obj.insert("tags".to_string(), json!(scenario_tags));
            }
            scenarios.push(Value::Object(obj));
        }
    }

    let total_count = scenarios.len();
    let message = compose_message(total_count, &tags);

    let mut envelope = serde_json::Map::new();
    envelope.insert("success".to_string(), json!(true));
    envelope.insert("scenarios".to_string(), Value::Array(scenarios));
    envelope.insert("totalCount".to_string(), json!(total_count));
    envelope.insert("message".to_string(), json!(message));
    if !warnings.is_empty() {
        envelope.insert("warnings".to_string(), json!(warnings));
    }
    serialize(&Value::Object(envelope))
}

/// Compose the human-readable summary message — parity with the TS branch at
/// `src/commands/get-scenarios.ts:138-147`.
fn compose_message(total_count: usize, tags: &[String]) -> String {
    if total_count == 0 && !tags.is_empty() {
        format!("No scenarios found matching tags: {}", tags.join(", "))
    } else if total_count == 0 {
        "No scenarios found".to_string()
    } else if !tags.is_empty() {
        format!(
            "Found {total_count} {} matching tags: {}",
            pluralize(total_count),
            tags.join(", ")
        )
    } else {
        format!("Found {total_count} {}", pluralize(total_count))
    }
}

fn pluralize(n: usize) -> &'static str {
    if n == 1 {
        "scenario"
    } else {
        "scenarios"
    }
}

fn serialize(v: &Value) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(v).map_err(|e| FspecCoreError::InvalidArgs {
        command: "get-scenarios",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: GetScenariosArgs = serde_json::from_str("{}").unwrap();
        assert!(a.tags.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_full() {
        let a: GetScenariosArgs =
            serde_json::from_str(r#"{"tags":["@a","@b"],"format":"json"}"#).unwrap();
        assert_eq!(a.tags.as_deref().unwrap().len(), 2);
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn message_no_tags_zero() {
        assert_eq!(compose_message(0, &[]), "No scenarios found");
    }

    #[test]
    fn message_with_tags_zero() {
        assert_eq!(
            compose_message(0, &["@x".to_string()]),
            "No scenarios found matching tags: @x"
        );
    }

    #[test]
    fn message_singular_vs_plural() {
        assert_eq!(compose_message(1, &[]), "Found 1 scenario");
        assert_eq!(compose_message(3, &[]), "Found 3 scenarios");
        assert_eq!(
            compose_message(2, &["@a".to_string(), "@b".to_string()]),
            "Found 2 scenarios matching tags: @a, @b"
        );
    }
}
