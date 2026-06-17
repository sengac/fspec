//! `compare-implementations` — Rust port of
//! `src/commands/compare-implementations.ts` (RPC-207).
//!
//! Queries work units carrying a tag, optionally aggregates deduplicated
//! test/impl file paths from `.feature.coverage` sidecars, and returns the
//! JSON envelope `{workUnits, comparison, namingConventionDifferences,
//! coverage}`.
//!
//! ## Parity notes
//! - The TS implementation reuses `queryWorkUnits({tag})` which reads
//!   `spec/work-units.json` DIRECTLY (no auto-create) — a missing file
//!   surfaces an IO error that the dispatcher reports as a failure. We mirror
//!   that by reading the file ourselves and escalating ENOENT.
//! - `workUnits` is mapped to `{tags}` ONLY (TS maps `wu => ({ tags })`).
//! - `comparison.type` is the constant `"side-by-side"`.
//! - `namingConventionDifferences` is always empty (TS leaves it as a TODO).
//! - When `showCoverage` is true the coverage array contains exactly ONE
//!   entry whose `testFiles` / `implementationFiles` are the deduplicated,
//!   first-encounter-order paths read across all `.feature.coverage` files.
//!   When `showCoverage` is false the coverage array is empty.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::io_error::format_io_error;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CompareImplementationsArgs {
    tag: Option<String>,
    show_coverage: bool,
    #[allow(dead_code)]
    json: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CompareImplementationsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "compare-implementations",
            reason: format!("failed to parse args: {e}"),
        })?;

    let tag = args.tag.unwrap_or_default();

    // ---- Query work units (parity with queryWorkUnits({tag})) ----
    // queryWorkUnits reads spec/work-units.json directly; ENOENT surfaces as
    // an error so the dispatcher / CLI both report a failure. TS wraps any
    // failure with the literal prefix `Failed to query work units:` (RPC-207
    // parity fix — the earlier port used the generic InvalidArgs prefix).
    let work_units_path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&work_units_path).map_err(|e| {
        FspecCoreError::Message(format!(
            "Failed to query work units: {}",
            format_io_error(&e, &work_units_path.display().to_string())
        ))
    })?;
    let data: Value = serde_json::from_str(&raw).map_err(|e| {
        FspecCoreError::Message(format!(
            "Failed to query work units: {}",
            crate::io::json_error::parse_json_reason(&raw, &e)
        ))
    })?;

    // Filter by tag and project down to `{tags}` (TS: wu => ({ tags })).
    let mut work_units: Vec<Value> = Vec::new();
    if let Some(map) = data.get("workUnits").and_then(Value::as_object) {
        for wu in map.values() {
            let tags: Vec<Value> = wu
                .get("tags")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let matches = tags.iter().any(|t| t.as_str() == Some(tag.as_str()));
            if matches {
                work_units.push(json!({ "tags": tags }));
            }
        }
    }

    // ---- Coverage aggregation ----
    let coverage = if args.show_coverage {
        let (test_files, impl_files) = collect_coverage(project_root)?;
        vec![json!({
            "testFiles": test_files,
            "implementationFiles": impl_files,
        })]
    } else {
        Vec::new()
    };

    let result = json!({
        "workUnits": work_units,
        "comparison": { "type": "side-by-side" },
        "namingConventionDifferences": Vec::<Value>::new(),
        "coverage": coverage,
    });

    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "compare-implementations",
        reason: format!("failed to serialise response: {e}"),
    })
}

/// Read every `spec/features/*.feature.coverage` file and collect the
/// deduplicated test-file and implementation-file paths in first-encounter
/// order (parity with `Array.from(new Set(...))`).
///
/// TS schema: `scenarios[].testMappings[].file` (test files) and
/// `scenarios[].testMappings[].implMappings[].file` (impl files).
fn collect_coverage(project_root: &Path) -> Result<(Vec<String>, Vec<String>), FspecCoreError> {
    let features_dir = project_root.join("spec").join("features");
    if !features_dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let entries = std::fs::read_dir(&features_dir).map_err(|source| FspecCoreError::Io {
        command: "compare-implementations",
        source,
    })?;

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

    // first-encounter-order dedup
    let mut test_files: Vec<String> = Vec::new();
    let mut impl_files: Vec<String> = Vec::new();

    for path in paths {
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(scenarios) = v.get("scenarios").and_then(Value::as_array) else {
            continue;
        };
        for scenario in scenarios {
            let Some(mappings) = scenario.get("testMappings").and_then(Value::as_array) else {
                continue;
            };
            for m in mappings {
                if let Some(tf) = m
                    .get("file")
                    .and_then(Value::as_str)
                    .or_else(|| m.get("filePath").and_then(Value::as_str))
                {
                    push_unique(&mut test_files, tf);
                }
                if let Some(impl_mappings) = m.get("implMappings").and_then(Value::as_array) {
                    for im in impl_mappings {
                        if let Some(ifp) = im.get("file").and_then(Value::as_str) {
                            push_unique(&mut impl_files, ifp);
                        }
                    }
                }
            }
        }
    }

    Ok((test_files, impl_files))
}

fn push_unique(vec: &mut Vec<String>, value: &str) {
    if !vec.iter().any(|v| v == value) {
        vec.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: CompareImplementationsArgs =
            serde_json::from_str(r#"{"tag":"@cli","showCoverage":true,"json":true}"#).unwrap();
        assert_eq!(a.tag.as_deref(), Some("@cli"));
        assert!(a.show_coverage);
        assert!(a.json);
    }

    #[test]
    fn args_default_empty() {
        let a: CompareImplementationsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.tag.is_none());
        assert!(!a.show_coverage);
    }

    #[test]
    fn push_unique_dedups() {
        let mut v: Vec<String> = Vec::new();
        push_unique(&mut v, "a");
        push_unique(&mut v, "a");
        push_unique(&mut v, "b");
        assert_eq!(v, vec!["a".to_string(), "b".to_string()]);
    }
}
