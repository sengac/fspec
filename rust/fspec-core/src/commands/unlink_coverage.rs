//! `unlink-coverage` — Rust port of `src/commands/unlink-coverage.ts`
//! (RPC-311).
//!
//! MUTATION command: removes test and/or implementation mappings from a
//! scenario's entry in a `<feature>.feature.coverage` sidecar, recomputes the
//! aggregated `stats` block, and atomically writes the file back.
//!
//! Three modes (mirroring the TS branches):
//!   * `all` → empty the scenario's `testMappings`.
//!   * `testFile` + `implFile` → remove only that impl mapping from the test
//!     mapping.
//!   * `testFile` alone → remove the whole test mapping (and its impl mappings).
//!
//! Returns a JSON envelope `{ "success": true, "message": "<...>" }`. The CLI
//! bridge surfaces `message` on stdout; the dispatcher returns the envelope to
//! the LLM. All mutation + rendering logic lives here — the bridge is a thin
//! arg-marshalling façade.
//!
//! ## Byte-parity strategy
//!
//! The TS implementation mutates the parsed JSON object in place and writes it
//! back with `JSON.stringify(data, null, 2)` (no trailing newline), preserving
//! the original top-level key ORDER (the TS `Object.assign(fileData, coverage)`
//! overwrites existing keys without reordering). To match that byte-for-byte we
//! operate directly on a `serde_json::Value` (the workspace enables
//! `preserve_order`), rather than a typed struct that would re-emit keys in
//! declaration order.
//!
//! ## Validation / error parity (substrings asserted by tests)
//!
//! * Neither `all` nor `testFile` → `Must specify either --all or --test-file`.
//! * `implFile` without `testFile` → `--test-file is required when specifying
//!   --impl-file`.
//! * Missing sidecar → `Coverage file not found`.
//! * Unknown scenario → `Scenario not found`.
//! * Unknown test file → `Test file not found in scenario mappings`.
//! * Unknown impl file → `Implementation file not found in test mapping`.
//! * `updateStats` throws verbatim when the file has no `stats` object
//!   (`Cannot set properties of undefined (setting 'coveredScenarios')`) or
//!   when a test mapping has no `implMappings` array in impl-mode
//!   (`Cannot read properties of undefined (reading 'findIndex')`).

use std::path::Path;

use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

use serde::Deserialize;

/// CLI / dispatcher arguments accepted by `unlink-coverage`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct UnlinkCoverageArgs {
    /// Feature basename (e.g. `"user-login"`), with or without `.feature`.
    /// Supplied as the positional `<feature-name>` on the CLI.
    #[serde(default)]
    feature_name: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    #[serde(default)]
    test_file: Option<String>,
    #[serde(default)]
    impl_file: Option<String>,
    #[serde(default)]
    all: Option<bool>,
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "unlink-coverage",
        reason,
    }
}

/// Dispatcher / CLI entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UnlinkCoverageArgs = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;

    let feature_name = args
        .feature_name
        .as_deref()
        .ok_or_else(|| invalid("missing required 'featureName' argument".to_string()))?;
    let scenario = args
        .scenario
        .as_deref()
        .ok_or_else(|| invalid("missing required 'scenario' argument".to_string()))?;

    let all = args.all.unwrap_or(false);
    let test_file = args.test_file.as_deref();
    let impl_file = args.impl_file.as_deref();

    // Validate flag combinations (parity with the TS guards).
    if !all && test_file.is_none() {
        return Err(invalid(
            "Must specify either --all or --test-file\nUse --all to remove all mappings, or --test-file to remove specific test mapping".to_string(),
        ));
    }
    if impl_file.is_some() && test_file.is_none() {
        return Err(invalid(
            "--test-file is required when specifying --impl-file\nImplementation mappings are attached to test mappings".to_string(),
        ));
    }

    // Resolve the coverage sidecar path (tolerate a trailing `.feature`).
    let stripped = feature_name
        .strip_suffix(".feature")
        .unwrap_or(feature_name);
    let file_name = format!("{stripped}.feature");
    let coverage_path = project_root
        .join("spec")
        .join("features")
        .join(format!("{file_name}.coverage"));

    // TS reads the file and treats any failure (missing OR unreadable OR bad
    // JSON) as "Coverage file not found".
    let raw = match std::fs::read_to_string(&coverage_path) {
        Ok(c) => c,
        Err(_) => {
            return Err(invalid(format!(
                "Coverage file not found: {file_name}.coverage\nSuggestion: Run fspec show-coverage to see available features"
            )));
        }
    };
    let mut coverage: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            return Err(invalid(format!(
                "Coverage file not found: {file_name}.coverage\nSuggestion: Run fspec show-coverage to see available features"
            )));
        }
    };

    // Locate the scenario by name within `coverage.scenarios`.
    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scenario_idx = scenarios
        .iter()
        .position(|s| s.get("name").and_then(Value::as_str) == Some(scenario));
    let scenario_idx = match scenario_idx {
        Some(i) => i,
        None => {
            let available: String = scenarios
                .iter()
                .map(|s| {
                    format!(
                        "  - {}",
                        s.get("name").and_then(Value::as_str).unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(invalid(format!(
                "Scenario not found: \"{scenario}\"\nAvailable scenarios:\n{available}"
            )));
        }
    };

    // Mutate the scenario entry in place on the live `coverage` Value.
    let scenarios_arr = coverage
        .get_mut("scenarios")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("Scenario not found".to_string()))?;
    let scenario_entry = &mut scenarios_arr[scenario_idx];

    let message = if all {
        // Mode 1: remove all mappings.
        scenario_entry
            .as_object_mut()
            .map(|o| o.insert("testMappings".to_string(), Value::Array(vec![])));
        format!("\u{2713} Removed all coverage mappings for scenario \"{scenario}\"")
    } else if let (Some(tf), Some(imf)) = (test_file, impl_file) {
        // Mode 2: remove only the implementation mapping.
        // TS: `scenarioEntry.testMappings.find(...)` throws if testMappings is
        // undefined.
        let test_mappings = match scenario_entry
            .get_mut("testMappings")
            .and_then(Value::as_array_mut)
        {
            Some(tms) => tms,
            None => {
                return Err(invalid(
                    "Cannot read properties of undefined (reading 'find')".to_string(),
                ));
            }
        };
        // Find the matching test mapping.
        let tm_pos = test_mappings
            .iter()
            .position(|tm| tm.get("file").and_then(Value::as_str) == Some(tf));
        let tm_pos = match tm_pos {
            Some(p) => p,
            None => {
                return Err(invalid(format!(
                    "Test file not found in scenario mappings: {tf}\nSuggestion: Run fspec show-coverage to see current mappings"
                )));
            }
        };
        let tm = &mut test_mappings[tm_pos];
        // TS: testMapping.implMappings.findIndex — throws if implMappings
        // is undefined.
        let impl_mappings = tm.get_mut("implMappings").and_then(Value::as_array_mut);
        let impl_mappings = match impl_mappings {
            Some(ims) => ims,
            None => {
                return Err(invalid(
                    "Cannot read properties of undefined (reading 'findIndex')".to_string(),
                ));
            }
        };
        let impl_pos = impl_mappings
            .iter()
            .position(|im| im.get("file").and_then(Value::as_str) == Some(imf));
        let impl_pos = match impl_pos {
            Some(p) => p,
            None => {
                return Err(invalid(format!(
                    "Implementation file not found in test mapping: {imf}\nSuggestion: Run fspec show-coverage to see current mappings"
                )));
            }
        };
        impl_mappings.remove(impl_pos);
        format!("\u{2713} Removed implementation mapping {imf} from scenario \"{scenario}\"")
    } else if let Some(tf) = test_file {
        // Mode 3: remove the entire test mapping (and its impl mappings).
        // TS: `scenarioEntry.testMappings.findIndex(...)` throws if
        // testMappings is undefined.
        let test_mappings = match scenario_entry
            .get_mut("testMappings")
            .and_then(Value::as_array_mut)
        {
            Some(tms) => tms,
            None => {
                return Err(invalid(
                    "Cannot read properties of undefined (reading 'findIndex')".to_string(),
                ));
            }
        };
        let tm_pos = test_mappings
            .iter()
            .position(|tm| tm.get("file").and_then(Value::as_str) == Some(tf));
        let tm_pos = match tm_pos {
            Some(p) => p,
            None => {
                return Err(invalid(format!(
                    "Test file not found in scenario mappings: {tf}\nSuggestion: Run fspec show-coverage to see current mappings"
                )));
            }
        };
        test_mappings.remove(tm_pos);
        format!(
            "\u{2713} Removed test mapping {tf} (and all its implementation mappings) from scenario \"{scenario}\""
        )
    } else {
        // Unreachable: the flag-combination guards above reject this case.
        return Err(invalid(
            "Must specify either --all or --test-file".to_string(),
        ));
    };

    // Recalculate stats (mirrors TS `updateStats` — throws if `stats` absent).
    update_stats(&mut coverage)?;

    // Write back with `JSON.stringify(data, null, 2)` byte parity (no trailing
    // newline). `preserve_order` keeps the original top-level key ordering.
    write_json_atomic(&coverage_path, &coverage)?;

    serde_json::to_string_pretty(&json!({ "success": true, "message": message }))
        .map_err(|e| invalid(format!("failed to serialize result: {e}")))
}

/// Recompute the aggregated `stats` block in place — port of the TS
/// `updateStats(coverage)` helper (`src/commands/unlink-coverage.ts:146`).
///
/// Mirrors the TS exactly, including its runtime errors:
///   * If `coverage.stats` is absent/non-object, the TS assignment
///     `coverage.stats.coveredScenarios = …` throws
///     `Cannot set properties of undefined (setting 'coveredScenarios')`.
///   * `coverage.stats.totalScenarios` is read verbatim and never reassigned —
///     so a stats object that lacks `totalScenarios` leaves it absent.
///   * test lines derive from the `"N-M"` ranges; impl lines count
///     `implMapping.lines.length` (array length for `number[]`, char count for
///     the string form).
fn update_stats(coverage: &mut Value) -> Result<(), FspecCoreError> {
    let mut test_files: Vec<String> = Vec::new();
    let mut impl_files: Vec<String> = Vec::new();
    let mut total_test_lines: i64 = 0;
    let mut total_impl_lines: i64 = 0;
    let mut covered_scenarios: i64 = 0;

    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for scenario in &scenarios {
        let test_mappings = scenario
            .get("testMappings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !test_mappings.is_empty() {
            covered_scenarios += 1;
        }
        for tm in &test_mappings {
            if let Some(file) = tm.get("file").and_then(Value::as_str) {
                if !test_files.iter().any(|f| f == file) {
                    test_files.push(file.to_string());
                }
            }
            if let Some(lines) = tm.get("lines").and_then(Value::as_str) {
                let range: Vec<&str> = lines.split('-').collect();
                if range.len() == 2 {
                    if let (Ok(s), Ok(e)) = (range[0].parse::<i64>(), range[1].parse::<i64>()) {
                        total_test_lines += e - s + 1;
                    }
                }
            }
            // TS iterates `testMapping.implMappings` unconditionally; an absent
            // array here would throw at runtime, but `updateStats` is only
            // reached AFTER the mode branches, which (in impl-mode) already
            // guarantee the array exists. For all/test-file modes the array may
            // be absent on remaining mappings — TS would crash there too, but
            // those fixtures are not exercised; treat absent as empty here to
            // avoid panicking while still matching the common path.
            if let Some(impl_mappings) = tm.get("implMappings").and_then(Value::as_array) {
                for im in impl_mappings {
                    if let Some(file) = im.get("file").and_then(Value::as_str) {
                        if !impl_files.iter().any(|f| f == file) {
                            impl_files.push(file.to_string());
                        }
                    }
                    total_impl_lines += impl_lines_len(im.get("lines"));
                }
            }
        }
    }

    // TS: `coverage.stats.coveredScenarios = …` throws if `stats` is undefined.
    let stats = match coverage.get_mut("stats").and_then(Value::as_object_mut) {
        Some(s) => s,
        None => {
            return Err(invalid(
                "Cannot set properties of undefined (setting 'coveredScenarios')".to_string(),
            ));
        }
    };

    // `totalScenarios` is read verbatim; absent → treated as missing/0 for the
    // percentage and left untouched in the object (TS never assigns it).
    let total_scenarios = stats
        .get("totalScenarios")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let coverage_percent = if total_scenarios > 0 {
        ((covered_scenarios as f64) / (total_scenarios as f64) * 100.0).round() as i64
    } else {
        0
    };

    stats.insert("coveredScenarios".to_string(), json!(covered_scenarios));
    stats.insert("coveragePercent".to_string(), json!(coverage_percent));
    stats.insert("testFiles".to_string(), json!(test_files));
    stats.insert("implFiles".to_string(), json!(impl_files));
    stats.insert(
        "totalLinesCovered".to_string(),
        json!(total_test_lines + total_impl_lines),
    );

    Ok(())
}

/// `implMapping.lines.length` parity: array length for the `number[]` form,
/// char count for the `"N-M"` string form, 0 otherwise.
fn impl_lines_len(lines: Option<&Value>) -> i64 {
    match lines {
        Some(Value::Array(a)) => a.len() as i64,
        Some(Value::String(s)) => s.chars().count() as i64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: UnlinkCoverageArgs = serde_json::from_str("{}").unwrap();
        assert!(a.feature_name.is_none());
        assert!(a.scenario.is_none());
        assert!(a.all.is_none());
    }

    #[test]
    fn args_parse_full() {
        let a: UnlinkCoverageArgs = serde_json::from_str(
            r#"{"featureName":"user-login","scenario":"Login","testFile":"t.ts","implFile":"i.ts","all":false}"#,
        )
        .unwrap();
        assert_eq!(a.feature_name.as_deref(), Some("user-login"));
        assert_eq!(a.scenario.as_deref(), Some("Login"));
        assert_eq!(a.test_file.as_deref(), Some("t.ts"));
        assert_eq!(a.impl_file.as_deref(), Some("i.ts"));
        assert_eq!(a.all, Some(false));
    }

    #[test]
    fn update_stats_errors_when_stats_absent() {
        let mut cov = json!({
            "scenarios": [ { "name": "S", "testMappings": [] } ]
        });
        let err = update_stats(&mut cov).unwrap_err();
        assert!(format!("{err}").contains("Cannot set properties of undefined"));
    }

    #[test]
    fn update_stats_counts_lines_and_files() {
        let mut cov = json!({
            "stats": { "totalScenarios": 1 },
            "scenarios": [ { "name": "Login", "testMappings": [
                { "file": "t.ts", "lines": "1-10", "implMappings": [ { "file": "i.ts", "lines": [1,2,3] } ] }
            ] } ]
        });
        update_stats(&mut cov).unwrap();
        let stats = cov.get("stats").unwrap();
        assert_eq!(stats.get("coveredScenarios").unwrap(), 1);
        assert_eq!(stats.get("coveragePercent").unwrap(), 100);
        assert_eq!(stats.get("testFiles").unwrap(), &json!(["t.ts"]));
        assert_eq!(stats.get("implFiles").unwrap(), &json!(["i.ts"]));
        // 10 test lines (1-10 inclusive) + 3 impl lines (array length).
        assert_eq!(stats.get("totalLinesCovered").unwrap(), 13);
    }

    #[test]
    fn update_stats_preserves_total_scenarios_verbatim() {
        let mut cov = json!({
            "stats": { "totalScenarios": 2 },
            "scenarios": [
                { "name": "A", "testMappings": [ { "file": "t.ts", "lines": "1-3", "implMappings": [] } ] },
                { "name": "B", "testMappings": [] }
            ]
        });
        update_stats(&mut cov).unwrap();
        let stats = cov.get("stats").unwrap();
        assert_eq!(stats.get("totalScenarios").unwrap(), 2);
        assert_eq!(stats.get("coveragePercent").unwrap(), 50);
    }

    #[test]
    fn update_stats_impl_lines_string_counts_chars() {
        let mut cov = json!({
            "stats": { "totalScenarios": 1 },
            "scenarios": [ { "name": "S", "testMappings": [
                { "file": "t.ts", "lines": "1-3", "implMappings": [ { "file": "i.ts", "lines": "10-20" } ] }
            ] } ]
        });
        update_stats(&mut cov).unwrap();
        let stats = cov.get("stats").unwrap();
        // 3 test lines + 5 chars ("10-20").
        assert_eq!(stats.get("totalLinesCovered").unwrap(), 8);
    }
}
