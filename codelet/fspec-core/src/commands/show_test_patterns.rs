//! `show-test-patterns` — Rust port of `src/commands/show-test-patterns.ts`
//! (RPC-307).
//!
//! Loads `spec/work-units.json`, filters work units by the supplied
//! `tag`, optionally reads every `spec/features/*.feature.coverage` file
//! to extract a deduplicated `testFiles` array, and returns a structured
//! envelope:
//!
//! ```json
//! {
//!   "workUnits": [{ "tags": ["@cli"] }, ...],
//!   "testFiles": ["test/a.ts", "test/b.ts"],
//!   "patterns": [],
//!   "format": "table" | "json"
//! }
//! ```
//!
//! Notes:
//! * `tag` is required — missing surfaces an `InvalidArgs` error.
//! * Missing `spec/work-units.json` surfaces an `Io` error (parity with
//!   TS `queryWorkUnits` which `throws` on readFile failure).
//! * Missing `spec/features/` directory → empty `testFiles` (TS uses
//!   `glob` which returns `[]` when the dir is missing).
//! * Coverage files that fail to parse as JSON are skipped silently.
//! * Until the shared `crate::io::coverage_glob` helper is wired, this
//!   module owns an inlined private equivalent (`read_test_refs`).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitsData;

/// CLI / dispatcher arguments accepted by `show-test-patterns`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowArgs {
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    include_coverage: Option<bool>,
    #[serde(default)]
    json: Option<bool>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-test-patterns",
            reason: format!("failed to parse args: {e}"),
        })?;

    let tag = args
        .tag
        .as_deref()
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "show-test-patterns",
            reason: "missing required 'tag' argument".to_string(),
        })?;

    // Read work-units.json directly — TS queryWorkUnits throws if missing.
    let wu_path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&wu_path).map_err(|source| FspecCoreError::Io {
        command: "show-test-patterns",
        source,
    })?;
    let data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: e.to_string(),
        })?;

    // Filter by tag (tags live in WorkUnit::extra under key "tags").
    let mut filtered: Vec<Value> = Vec::new();
    for wu in data.work_units.values() {
        let tags = wu
            .extra
            .get("tags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_tag = tags.iter().any(|t| t.as_str() == Some(tag));
        if has_tag {
            filtered.push(json!({ "tags": tags }));
        }
    }

    // Coverage: if includeCoverage requested, read coverage files and
    // deduplicate test-file paths preserving first-encounter order.
    let mut test_files: Vec<String> = Vec::new();
    if args.include_coverage.unwrap_or(false) {
        let refs = read_test_refs(project_root);
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for fp in refs {
            if seen.insert(fp.clone()) {
                test_files.push(fp);
            }
        }
    }

    let format = if args.json.unwrap_or(false) {
        "json"
    } else {
        "table"
    };

    // The CLI bridge needs the human summary line for the default
    // (non-`--json`) rendering, but the TS `--json` envelope
    // (src/commands/show-test-patterns.ts:65-70) is exactly
    // `{ workUnits, testFiles, patterns, format }` — no `message` field.
    // We therefore include `message` ONLY when `format == "table"`, so
    // the bridge can render it without duplicating any business logic
    // AND the `--json` envelope stays byte-equivalent to the TS one.
    let mut envelope = serde_json::Map::new();
    envelope.insert("workUnits".to_string(), Value::Array(filtered.clone()));
    envelope.insert("testFiles".to_string(), json!(test_files));
    envelope.insert("patterns".to_string(), json!(Vec::<Value>::new()));
    envelope.insert("format".to_string(), Value::String(format.to_string()));
    if format == "table" {
        let message = format!(
            "\u{2713} Analyzed testing patterns for {} work units tagged with {}",
            filtered.len(),
            tag
        );
        envelope.insert("message".to_string(), Value::String(message));
    }
    let envelope = Value::Object(envelope);

    serde_json::to_string_pretty(&envelope).map_err(|e| FspecCoreError::InvalidArgs {
        command: "show-test-patterns",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Walk `spec/features/*.feature.coverage` and return every test-file
/// path string in encounter order. Tolerates both `{file, lines}` and
/// `{filePath, testLines}` field name conventions. Returns an empty
/// vec when the directory is missing OR when no coverage files exist.
fn read_test_refs(project_root: &Path) -> Vec<String> {
    let features_dir = project_root.join("spec").join("features");
    let Ok(entries) = std::fs::read_dir(&features_dir) else {
        return Vec::new();
    };

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        let is_cov = p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".feature.coverage"))
            .unwrap_or(false);
        if is_cov {
            paths.push(p);
        }
    }
    paths.sort();

    let mut out: Vec<String> = Vec::new();
    for path in paths {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let Some(scenarios) = v.get("scenarios").and_then(|s| s.as_array()) else {
            continue;
        };
        for scenario in scenarios {
            let Some(mappings) = scenario.get("testMappings").and_then(|m| m.as_array()) else {
                continue;
            };
            for m in mappings {
                let fp = m
                    .get("file")
                    .and_then(|v| v.as_str())
                    .or_else(|| m.get("filePath").and_then(|v| v.as_str()));
                if let Some(s) = fp {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: ShowArgs = serde_json::from_str("{}").unwrap();
        assert!(a.tag.is_none());
        assert!(a.include_coverage.is_none());
        assert!(a.json.is_none());
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ShowArgs =
            serde_json::from_str(r#"{"tag":"@cli","includeCoverage":true,"json":true}"#).unwrap();
        assert_eq!(a.tag.as_deref(), Some("@cli"));
        assert_eq!(a.include_coverage, Some(true));
        assert_eq!(a.json, Some(true));
    }
}
