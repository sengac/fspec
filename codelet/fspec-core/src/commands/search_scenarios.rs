//! `search-scenarios` — Rust port of `src/commands/search-scenarios.ts`
//! (RPC-297, part of QRY-002).
//!
//! Read-only command: walks `spec/features/*.feature`, parses each with the
//! lenient gherkin front-end, and matches the supplied `query` (literal
//! substring, case-insensitive — or a case-insensitive regex when
//! `regex=true`) against the feature name, feature description, feature file
//! path, the (optional) work-unit title, and finally each scenario name.
//! A feature-level match yields ALL of that feature's scenarios; otherwise
//! individual scenario-name matches are returned.
//!
//! Returns a JSON envelope (field order mirrors the TS result object):
//!
//! ```json
//! {
//!   "searchedFiles": <usize>,
//!   "scenarios": [ { "name", "scenarioName", "featureFilePath", "workUnitId" }, ... ],
//!   "format": "table" | "json",
//!   "searchMode": "literal" | "regex",
//!   "message"?: "✓ Found N scenarios matching \"<query>\""
//! }
//! ```
//!
//! The `message` field is emitted ONLY in the default (`format == "table"`)
//! path — mirroring `show_test_patterns`: the CLI bridge surfaces it as the
//! green summary line, while the `--json` envelope stays byte-equivalent to
//! the TS `JSON.stringify({ searchedFiles, scenarios, format, searchMode })`.
//!
//! Two-front-doors invariant: the LLM dispatcher AND the standalone clap CLI
//! both call this single `run` function — the bridge holds zero search logic.
//!
//! ## Parity notes
//!
//! * Missing `spec/features/` directory → `searchedFiles: 0`, empty
//!   `scenarios` (parity with TS `glob([...])` returning `[]`, NOT an error).
//! * Invalid regex (when `regex=true`) → `FspecCoreError::InvalidArgs` whose
//!   reason begins `Invalid regex pattern:` (contains the substring `regex`).
//! * `workUnitId` falls back to `"unknown"` when the feature carries no
//!   `@[A-Z]+-\d+` work-unit tag.
//! * BUG-059 parity: work-unit titles from `spec/work-units.json` participate
//!   in the feature-level match (tolerates a missing/invalid file).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

/// CLI / dispatcher arguments accepted by `search-scenarios`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SearchScenariosArgs {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    regex: Option<bool>,
    #[serde(default)]
    json: Option<bool>,
}

/// Dispatcher / CLI entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: SearchScenariosArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "search-scenarios",
            reason: format!("failed to parse args: {e}"),
        })?;

    let query = args
        .query
        .as_deref()
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "search-scenarios",
            reason: "missing required 'query' argument".to_string(),
        })?;

    let use_regex = args.regex.unwrap_or(false);
    let json_out = args.json.unwrap_or(false);

    // Compile the regex up-front so an invalid pattern surfaces regardless of
    // how many feature files are present (parity with TS `searchScenarios`
    // validating the pattern before iterating).
    let pattern = if use_regex {
        match regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
        {
            Ok(re) => Some(re),
            Err(e) => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "search-scenarios",
                    reason: format!("Invalid regex pattern: \"{query}\". {e}"),
                });
            }
        }
    } else {
        None
    };

    // Collect feature files (tolerate a missing directory → empty list).
    let files = glob_feature_files(project_root).unwrap_or_default();

    // Optional BUG-059 work-unit title map (id → title). Missing/invalid file
    // is silently ignored.
    let work_unit_titles = load_work_unit_titles(project_root);

    let mut searched_files: usize = 0;
    let mut scenarios: Vec<Value> = Vec::new();

    for rel in &files {
        let abs = project_root.join(rel);
        let Ok(content) = std::fs::read_to_string(&abs) else {
            // Unreadable file → skipped, not counted (parity with the TS
            // try/continue around readFile + parse).
            continue;
        };
        let Ok(feature) = parse_feature_lenient(&content) else {
            continue;
        };
        searched_files += 1;

        let work_unit_id = feature
            .tags
            .iter()
            .find_map(|t| extract_work_unit_id(t))
            .map(ToString::to_string);

        let feature_name = feature.name.clone();
        let feature_description = feature.description.clone().unwrap_or_default();
        let work_unit_title = work_unit_id
            .as_deref()
            .and_then(|id| work_unit_titles.get(id))
            .cloned()
            .unwrap_or_default();

        let feature_matches = matches_query(&feature_name, query, pattern.as_ref())
            || matches_query(&feature_description, query, pattern.as_ref())
            || matches_query(rel, query, pattern.as_ref())
            || matches_query(&work_unit_title, query, pattern.as_ref());

        let id_field = work_unit_id.as_deref().unwrap_or("unknown");

        if feature_matches {
            for s in &feature.scenarios {
                scenarios.push(scenario_entry(&s.name, rel, id_field));
            }
        } else {
            for s in &feature.scenarios {
                if matches_query(&s.name, query, pattern.as_ref()) {
                    scenarios.push(scenario_entry(&s.name, rel, id_field));
                }
            }
        }
    }

    let search_mode = if use_regex { "regex" } else { "literal" };
    let format = if json_out { "json" } else { "table" };

    let mut envelope = serde_json::Map::new();
    envelope.insert("searchedFiles".to_string(), json!(searched_files));
    let scenario_count = scenarios.len();
    envelope.insert("scenarios".to_string(), Value::Array(scenarios));
    envelope.insert("format".to_string(), json!(format));
    envelope.insert("searchMode".to_string(), json!(search_mode));
    if format == "table" {
        // Mirrors the TS green summary line: `✓ Found N scenarios matching "<query>"`.
        envelope.insert(
            "message".to_string(),
            json!(format!(
                "\u{2713} Found {scenario_count} scenarios matching \"{query}\""
            )),
        );
    }

    serde_json::to_string_pretty(&Value::Object(envelope)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "search-scenarios",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Build a single scenario result object. Field order mirrors the TS literal:
/// `{ name, scenarioName, featureFilePath, workUnitId }`. `name` duplicates
/// `scenarioName` for backward compatibility with legacy callers.
fn scenario_entry(name: &str, feature_file_path: &str, work_unit_id: &str) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("name".to_string(), json!(name));
    obj.insert("scenarioName".to_string(), json!(name));
    obj.insert("featureFilePath".to_string(), json!(feature_file_path));
    obj.insert("workUnitId".to_string(), json!(work_unit_id));
    Value::Object(obj)
}

/// `true` when `text` matches `query`: regex (`pattern`) when supplied, else a
/// case-insensitive substring test (parity with TS `matchesQuery`).
fn matches_query(text: &str, query: &str, pattern: Option<&regex::Regex>) -> bool {
    match pattern {
        Some(re) => re.is_match(text),
        None => text.to_lowercase().contains(&query.to_lowercase()),
    }
}

/// Extract a work-unit id from a feature tag (the gherkin crate strips the
/// leading `@`). Matches the TS regex `^@[A-Z]+-\d+$` against the `@`-prefixed
/// form, i.e. the stored tag must be `[A-Z]+-\d+`.
fn extract_work_unit_id(tag: &str) -> Option<&str> {
    let (prefix, num) = tag.split_once('-')?;
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(tag)
}

/// Load `spec/work-units.json` and return an `id → title` map. Returns an empty
/// map when the file is missing or cannot be parsed (parity with the TS
/// try/catch that continues without work-unit-title search).
fn load_work_unit_titles(project_root: &Path) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let path = project_root.join("spec").join("work-units.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return map;
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return map;
    };
    if let Some(units) = v.get("workUnits").and_then(Value::as_object) {
        for (id, wu) in units {
            if let Some(title) = wu.get("title").and_then(Value::as_str) {
                map.insert(id.clone(), title.to_string());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: SearchScenariosArgs = serde_json::from_str("{}").unwrap();
        assert!(a.query.is_none());
        assert!(a.regex.is_none());
        assert!(a.json.is_none());
    }

    #[test]
    fn args_parse_full() {
        let a: SearchScenariosArgs =
            serde_json::from_str(r#"{"query":"x","regex":true,"json":true}"#).unwrap();
        assert_eq!(a.query.as_deref(), Some("x"));
        assert_eq!(a.regex, Some(true));
        assert_eq!(a.json, Some(true));
    }

    #[test]
    fn extract_work_unit_id_accepts_canonical() {
        assert_eq!(extract_work_unit_id("AUTH-001"), Some("AUTH-001"));
        assert_eq!(extract_work_unit_id("VAL-12"), Some("VAL-12"));
    }

    #[test]
    fn extract_work_unit_id_rejects_non_canonical() {
        assert_eq!(extract_work_unit_id("wip"), None);
        assert_eq!(extract_work_unit_id("auth-001"), None);
        assert_eq!(extract_work_unit_id("AUTH-"), None);
        assert_eq!(extract_work_unit_id("AUTH001"), None);
    }

    #[test]
    fn matches_query_literal_is_case_insensitive() {
        assert!(matches_query("Login With Credentials", "login", None));
        assert!(!matches_query("Logout", "login", None));
    }
}
