//! Shared helper that enumerates `spec/features/*.feature.coverage` files
//! and extracts test-file path strings from their `scenarios[].testMappings`
//! arrays.
//!
//! Introduced for RPC-307 `show-test-patterns`, but designed to be
//! reused by any future coverage-aware command (e.g. `show-coverage`,
//! `audit-coverage`).
//!
//! ### Schema tolerance
//!
//! The canonical `*.feature.coverage` files written by `link-coverage`
//! use `testMappings[i].file` and `testMappings[i].lines` (see
//! `spec/features/show-feature-cli-subcommand.feature.coverage`).
//! Some legacy / test-only fixtures use `filePath`/`testLines` as the
//! field names. To stay robust against both conventions this helper
//! reads either field name.

use std::path::Path;

use serde_json::Value;

use crate::error::FspecCoreError;

/// Single test-file reference extracted from a coverage file.
#[derive(Debug, Clone)]
pub struct CoverageTestRef {
    pub file_path: String,
    #[allow(dead_code)]
    pub lines: String,
}

/// Read every `spec/features/*.feature.coverage` file under
/// `project_root` and return all test-file references in encounter
/// order. Missing `spec/features/` directory → `Ok(Vec::new())`
/// (parity with `glob([…])` returning an empty list).
///
/// Coverage files that fail to parse as JSON are silently skipped
/// (parity with TS `try { JSON.parse } catch { continue }`).
pub fn read_all_coverage_files(project_root: &Path) -> Result<Vec<CoverageTestRef>, FspecCoreError> {
    let features_dir = project_root.join("spec").join("features");
    if !features_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&features_dir).map_err(|source| FspecCoreError::Io {
        command: "read_all_coverage_files",
        source,
    })?;

    // Stable order: sort by file name so output is deterministic.
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

    let mut refs: Vec<CoverageTestRef> = Vec::new();
    for path in paths {
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(scenarios) = v.get("scenarios").and_then(|s| s.as_array()) else {
            continue;
        };
        for scenario in scenarios {
            let Some(mappings) = scenario.get("testMappings").and_then(|m| m.as_array()) else {
                continue;
            };
            for m in mappings {
                let file_path = m
                    .get("file")
                    .and_then(|v| v.as_str())
                    .or_else(|| m.get("filePath").and_then(|v| v.as_str()));
                let lines = m
                    .get("lines")
                    .and_then(|v| v.as_str())
                    .or_else(|| m.get("testLines").and_then(|v| v.as_str()))
                    .unwrap_or("");
                if let Some(fp) = file_path {
                    refs.push(CoverageTestRef {
                        file_path: fp.to_string(),
                        lines: lines.to_string(),
                    });
                }
            }
        }
    }
    Ok(refs)
}
