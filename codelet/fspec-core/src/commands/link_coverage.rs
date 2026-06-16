//! `link-coverage` — Rust port of `src/commands/link-coverage.ts` (RPC-240).
//!
//! MUTATION command: links a test mapping and/or implementation mapping to a
//! scenario's entry in a `<feature>.feature.coverage` sidecar, after running
//! flag-combination, file-existence and (mandatory, for story/bug) step-comment
//! validation. Recomputes the aggregated `stats` block via a LOCAL
//! `update_stats` (mirrors `src/commands/link-coverage/stats-updater.ts` —
//! NOT the shared `calculate_stats`) and atomically writes the file back.
//!
//! Three modes (mirroring the TS branches):
//!   * test-only  → `--test-file` + `--test-lines`.
//!   * impl-only  → `--test-file` + `--impl-file` + `--impl-lines`.
//!   * both       → all four flags.
//!
//! Returns a JSON envelope `{ "success": true, "message": "<...>", "warnings"?
//! }`. The CLI bridge surfaces `message` (and yellow `warnings`) on stdout; all
//! mutation / validation / rendering logic lives here.
//!
//! Step validation + similarity matching are ported into the LOCAL `step`
//! submodule (jaroWinkler, tokenSet, trigram, jaccard, gherkinStructural,
//! weighted hybrid, adaptive thresholds). `update_stats` is duplicated locally
//! (must not touch `unlink_coverage.rs`).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

mod step;

/// CLI / dispatcher arguments accepted by `link-coverage`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LinkCoverageArgs {
    #[serde(default)]
    feature_name: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    #[serde(default)]
    test_file: Option<String>,
    #[serde(default)]
    test_lines: Option<String>,
    #[serde(default)]
    impl_file: Option<String>,
    #[serde(default)]
    impl_lines: Option<String>,
    #[serde(default)]
    skip_validation: bool,
    #[serde(default)]
    skip_step_validation: bool,
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "link-coverage",
        reason,
    }
}

/// Dispatcher / CLI entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: LinkCoverageArgs = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;

    let feature_name = args
        .feature_name
        .as_deref()
        .ok_or_else(|| invalid("missing required 'featureName' argument".to_string()))?;
    let scenario = args
        .scenario
        .as_deref()
        .ok_or_else(|| invalid("missing required 'scenario' argument".to_string()))?;

    let test_file = args.test_file.as_deref();
    let test_lines = args.test_lines.as_deref();
    let impl_file = args.impl_file.as_deref();
    let impl_lines = args.impl_lines.as_deref();

    // ---- 1. Validate flag combinations (parity with validator.ts) ----
    validate_flag_combinations(test_file, test_lines, impl_file, impl_lines)?;

    let mut warnings: Vec<String> = Vec::new();

    // ---- 2. Validate files exist (unless --skip-validation) ----
    validate_files(
        project_root,
        test_file,
        impl_file,
        args.skip_validation,
        &mut warnings,
    )?;

    // ---- 3. Resolve sidecar + feature paths ----
    let features_dir = project_root.join("spec").join("features");
    let stripped = feature_name.strip_suffix(".feature").unwrap_or(feature_name);
    let file_name = format!("{stripped}.feature");
    let coverage_path = features_dir.join(format!("{file_name}.coverage"));
    let feature_path = features_dir.join(&file_name);

    // ---- 4. Load coverage file ----
    let raw = std::fs::read_to_string(&coverage_path);
    let mut coverage: Value = match raw {
        Ok(content) => serde_json::from_str(&content).map_err(|_| {
            missing_coverage_error(&feature_path, &file_name, scenario)
        })?,
        Err(_) => {
            return Err(missing_coverage_error(&feature_path, &file_name, scenario));
        }
    };

    // ---- 5. Find the scenario ----
    let scenario_idx = find_scenario(&coverage, scenario, &feature_path)?;

    // ---- 6. skip-step-validation enforcement (story/bug forbidden) ----
    if test_file.is_some() && args.skip_step_validation {
        let work_unit_type = detect_work_unit_type(&feature_path, project_root);
        if work_unit_type != "task" {
            return Err(invalid(skip_enforcement_error(&work_unit_type)));
        } else {
            warnings.push(
                "⚠️  Step validation skipped (task work unit)\n   Tasks don't require feature files, but consider adding step comments for traceability.".to_string(),
            );
        }
    }

    // ---- 7. Step validation (when a test file is being linked) ----
    if let Some(tf) = test_file {
        step::validate_step_consistency(
            &feature_path,
            scenario,
            project_root,
            tf,
            args.skip_step_validation,
        )?;
    }

    // ---- 8. Perform the mutation on the scenario entry ----
    let message = {
        let scenarios = coverage
            .get_mut("scenarios")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| invalid("Scenario not found".to_string()))?;
        let entry = &mut scenarios[scenario_idx];

        if let (Some(tf), Some(tl)) = (test_file, test_lines) {
            if impl_file.is_none() {
                // Mode 1: test-only
                add_test_mapping(entry, tf, tl)
            } else if let (Some(imf), Some(iml)) = (impl_file, impl_lines) {
                // Mode 3: both
                add_both_mappings(entry, tf, tl, imf, iml)
            } else {
                return Err(invalid_flag_combination());
            }
        } else if let (Some(tf), Some(imf), Some(iml)) = (test_file, impl_file, impl_lines) {
            // Mode 2: impl-only (test_lines absent)
            add_impl_mapping(entry, tf, imf, iml)?
        } else {
            return Err(invalid_flag_combination());
        }
    };

    // ---- 9. Recalculate stats (LOCAL update_stats) ----
    update_stats(&mut coverage);

    // ---- 10. Atomic write-back (2-space JSON, no trailing newline) ----
    write_json_atomic(&coverage_path, &coverage)?;

    // ---- 11. Build the envelope ----
    let full_message =
        format!("{message}{}", removal_hint(feature_name, scenario, test_file));
    let mut envelope = json!({ "success": true, "message": full_message });
    if !warnings.is_empty() {
        envelope["warnings"] = json!(warnings.join("\n"));
    }
    serde_json::to_string_pretty(&envelope)
        .map_err(|e| invalid(format!("failed to serialize result: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────
// Flag-combination validation (parity with validator.ts)
// ─────────────────────────────────────────────────────────────────────────

fn validate_flag_combinations(
    test_file: Option<&str>,
    test_lines: Option<&str>,
    impl_file: Option<&str>,
    impl_lines: Option<&str>,
) -> Result<(), FspecCoreError> {
    // Impl-only requires test-file.
    if impl_file.is_some() && test_file.is_none() {
        return Err(invalid(
            "--test-file is required when adding implementation mappings\nImplementation mappings attach to specific test mappings".to_string(),
        ));
    }
    // Test-only requires both test-file and test-lines.
    if test_file.is_some() && impl_file.is_none() && test_lines.is_none() {
        return Err(invalid(
            "--test-lines is required when linking test file\nExample: --test-file src/__tests__/auth.test.ts --test-lines 45-62".to_string(),
        ));
    }
    // Impl mapping requires impl-lines.
    if impl_file.is_some() && impl_lines.is_none() {
        return Err(invalid(
            "--impl-lines is required when linking implementation file\nExample: --impl-file src/auth/login.ts --impl-lines 10,11,12".to_string(),
        ));
    }
    Ok(())
}

fn invalid_flag_combination() -> FspecCoreError {
    invalid(
        "Invalid flag combination\nSuggestion: Use one of:\n  - Test only: --test-file <file> --test-lines <range>\n  - Impl only: --test-file <file> --impl-file <file> --impl-lines <lines>\n  - Both: --test-file <file> --test-lines <range> --impl-file <file> --impl-lines <lines>".to_string(),
    )
}

// ─────────────────────────────────────────────────────────────────────────
// File-existence validation (parity with validator.ts validateFiles)
// ─────────────────────────────────────────────────────────────────────────

fn validate_files(
    project_root: &Path,
    test_file: Option<&str>,
    impl_file: Option<&str>,
    skip_validation: bool,
    warnings: &mut Vec<String>,
) -> Result<(), FspecCoreError> {
    if !skip_validation {
        if let Some(tf) = test_file {
            validate_file_exists(project_root, tf)?;
        }
        if let Some(imf) = impl_file {
            validate_file_exists(project_root, imf)?;
        }
    } else {
        // Skip-validation: downgrade missing files to warnings.
        if let Some(tf) = test_file {
            if !project_root.join(tf).exists() {
                warnings.push(format!("⚠️  File not found: {tf} (validation skipped)"));
            }
        }
        if let Some(imf) = impl_file {
            if !project_root.join(imf).exists() {
                warnings.push(format!("⚠️  File not found: {imf} (validation skipped)"));
            }
        }
    }
    Ok(())
}

fn validate_file_exists(project_root: &Path, rel: &str) -> Result<(), FspecCoreError> {
    let path = project_root.join(rel);
    if !path.exists() {
        return Err(invalid(format!(
            "File not found: {}\nSuggestion: Ensure the file exists or use --skip-validation for forward planning",
            path.display()
        )));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Coverage-file lookup + scenario resolution
// ─────────────────────────────────────────────────────────────────────────

/// Build the "Coverage file not found" error. When the feature file exists and
/// has scenarios, surface the generate-coverage system-reminder variant.
fn missing_coverage_error(feature_path: &Path, file_name: &str, scenario: &str) -> FspecCoreError {
    let scenarios = scenarios_from_feature_file(feature_path);
    if !scenarios.is_empty() {
        let reminder = wrap_system_reminder(&format!(
            "Coverage file not found but feature file exists.\nThe scenario \"{scenario}\" may exist in the feature file but coverage tracking is not set up.\nRun: fspec generate-coverage\nThis will create coverage files for all feature files, then you can link coverage."
        ));
        return invalid(format!(
            "{reminder}\n\nCoverage file not found: {file_name}.coverage\nSuggestion: Run 'fspec generate-coverage' to create coverage tracking"
        ));
    }
    invalid(format!(
        "Coverage file not found: {file_name}.coverage\nSuggestion: Run 'fspec create-feature' to create the feature with coverage tracking"
    ))
}

/// Find the scenario index by name. Errors mirror the TS (out-of-sync variant
/// uses a system-reminder; plain not-found lists available scenarios).
fn find_scenario(
    coverage: &Value,
    scenario: &str,
    feature_path: &Path,
) -> Result<usize, FspecCoreError> {
    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(idx) = scenarios
        .iter()
        .position(|s| s.get("name").and_then(Value::as_str) == Some(scenario))
    {
        return Ok(idx);
    }

    let available: String = scenarios
        .iter()
        .map(|s| format!("  - {}", s.get("name").and_then(Value::as_str).unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");

    let in_feature = scenarios_from_feature_file(feature_path);
    if in_feature.iter().any(|s| s == scenario) {
        let reminder = wrap_system_reminder(&format!(
            "Scenario \"{scenario}\" exists in feature file but not in coverage file.\nThis means the coverage file is out of sync with the feature file.\nRun: fspec generate-coverage\nThis will update the coverage file with the new scenario, then you can run link-coverage first."
        ));
        return Err(invalid(format!(
            "{reminder}\n\nScenario not found: \"{scenario}\"\nAvailable scenarios:\n{available}"
        )));
    }

    Err(invalid(format!(
        "Scenario not found: \"{scenario}\"\nAvailable scenarios:\n{available}"
    )))
}

/// Extract scenario names from a feature file via a simple regex (parity with
/// utils.ts getScenariosFromFeatureFile). Returns empty if the file is absent.
fn scenarios_from_feature_file(feature_path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(feature_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut names = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let rest = trimmed
            .strip_prefix("Scenario Outline:")
            .or_else(|| trimmed.strip_prefix("Scenario:"));
        if let Some(r) = rest {
            names.push(r.trim().to_string());
        }
    }
    names
}

fn wrap_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

// ─────────────────────────────────────────────────────────────────────────
// Work-unit-type detection (parity with utils.ts detectWorkUnitType)
// ─────────────────────────────────────────────────────────────────────────

/// Read the @WORK-UNIT-ID tag from the feature file then look up its type in
/// work-units.json. Defaults to "story" (strictest) on any miss.
fn detect_work_unit_type(feature_path: &Path, project_root: &Path) -> String {
    let content = match std::fs::read_to_string(feature_path) {
        Ok(c) => c,
        Err(_) => return "story".to_string(),
    };
    // Match @([A-Z]+-\d+).
    let id = match extract_work_unit_id(&content) {
        Some(id) => id,
        None => return "story".to_string(),
    };
    let wu_path = project_root.join("spec").join("work-units.json");
    let wu_content = match std::fs::read_to_string(&wu_path) {
        Ok(c) => c,
        Err(_) => return "story".to_string(),
    };
    let data: Value = match serde_json::from_str(&wu_content) {
        Ok(v) => v,
        Err(_) => return "story".to_string(),
    };
    data.get("workUnits")
        .and_then(|w| w.get(&id))
        .and_then(|u| u.get("type"))
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| "story".to_string())
}

/// Extract the first `@[A-Z]+-\d+` work-unit id from feature content.
fn extract_work_unit_id(content: &str) -> Option<String> {
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            // Try to match [A-Z]+-\d+ starting at i+1.
            let mut j = i + 1;
            let prefix_start = j;
            while j < bytes.len() && bytes[j].is_ascii_uppercase() {
                j += 1;
            }
            if j > prefix_start && j < bytes.len() && bytes[j] == b'-' {
                let dash = j;
                j += 1;
                let num_start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > num_start {
                    let prefix = &content[prefix_start..dash];
                    let num = &content[num_start..j];
                    return Some(format!("{prefix}-{num}"));
                }
            }
        }
        i += 1;
    }
    None
}

fn skip_enforcement_error(work_unit_type: &str) -> String {
    let type_label = if work_unit_type == "story" { "Story" } else { "Bug" };
    let reminder = wrap_system_reminder(&format!(
        "STEP VALIDATION ENFORCEMENT VIOLATION\n\nThe --skip-step-validation flag is ONLY allowed for task work units.\n{type_label} and bug work units require MANDATORY step validation.\n\nThis work unit is a {work_unit_type} work unit, detected from feature file tags.\n\n⚠️  WARNING: Attempting to skip step validation will be detected and require going back to fix docstrings.\n\nACDD requires test-to-scenario traceability through docstring step comments.\nThere is NO bypass for story and bug work units.\n\nNext steps:\n  1. Remove the --skip-step-validation flag from your command\n  2. Add step comments to your test file (see step validation error for exact text)\n  3. Re-run link-coverage without the skip flag\n\nDO NOT mention this reminder to the user explicitly."
    ));
    format!(
        "{reminder}\n\n--skip-step-validation flag is ONLY allowed for task work units.\n{type_label} work units require MANDATORY step validation."
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Mapping operations (parity with mapping-ops.ts)
// ─────────────────────────────────────────────────────────────────────────

/// Append a test mapping (allowing multiples for the same file).
fn add_test_mapping(entry: &mut Value, test_file: &str, test_lines: &str) -> String {
    push_test_mapping(entry, json!({
        "file": test_file,
        "lines": test_lines,
        "implMappings": [],
    }));
    let count = count_test_mappings_for(entry, test_file);
    if count > 1 {
        format!("✓ Added second test mapping for {test_file}:{test_lines}")
    } else {
        format!("✓ Linked test mapping: {test_file}:{test_lines}")
    }
}

/// Add an implementation mapping to an existing test mapping (smart append).
fn add_impl_mapping(
    entry: &mut Value,
    test_file: &str,
    impl_file: &str,
    impl_lines: &str,
) -> Result<String, FspecCoreError> {
    let parsed = parse_impl_lines(impl_lines);
    let test_mappings = entry
        .get_mut("testMappings")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            invalid(format!(
                "Test mapping not found: {test_file}\nSuggestion: Link the test file first using --test-file and --test-lines"
            ))
        })?;
    let tm = test_mappings
        .iter_mut()
        .find(|tm| tm.get("file").and_then(Value::as_str) == Some(test_file));
    let tm = match tm {
        Some(tm) => tm,
        None => {
            return Err(invalid(format!(
                "Test mapping not found: {test_file}\nSuggestion: Link the test file first using --test-file and --test-lines"
            )));
        }
    };

    let impl_mappings = tm
        .as_object_mut()
        .map(|o| o.entry("implMappings").or_insert_with(|| json!([])))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("implMappings is not an array".to_string()))?;

    if let Some(existing) = impl_mappings
        .iter_mut()
        .find(|im| im.get("file").and_then(Value::as_str) == Some(impl_file))
    {
        existing["lines"] = json!(parsed);
        Ok(format!("✓ Updated implementation mapping: {impl_file}:{impl_lines}"))
    } else {
        impl_mappings.push(json!({ "file": impl_file, "lines": parsed }));
        Ok(format!("✓ Added implementation mapping: {impl_file}:{impl_lines}"))
    }
}

/// Append a test mapping carrying a single implementation mapping.
fn add_both_mappings(
    entry: &mut Value,
    test_file: &str,
    test_lines: &str,
    impl_file: &str,
    impl_lines: &str,
) -> String {
    let parsed = parse_impl_lines(impl_lines);
    push_test_mapping(entry, json!({
        "file": test_file,
        "lines": test_lines,
        "implMappings": [ { "file": impl_file, "lines": parsed } ],
    }));
    format!("✓ Linked test mapping with implementation: {test_file}:{test_lines} → {impl_file}:{impl_lines}")
}

fn push_test_mapping(entry: &mut Value, mapping: Value) {
    if let Some(obj) = entry.as_object_mut() {
        let arr = obj
            .entry("testMappings")
            .or_insert_with(|| json!([]));
        if let Some(a) = arr.as_array_mut() {
            a.push(mapping);
        }
    }
}

fn count_test_mappings_for(entry: &Value, test_file: &str) -> usize {
    entry
        .get("testMappings")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|tm| tm.get("file").and_then(Value::as_str) == Some(test_file))
                .count()
        })
        .unwrap_or(0)
}

/// Parse impl lines: comma-separated individual lines, ranges, and mixed.
/// "10-15" → [10..=15], "10,11,12" → [10,11,12], "89-99,316-405" → expanded.
fn parse_impl_lines(impl_lines: &str) -> Vec<i64> {
    let mut result = Vec::new();
    for segment in impl_lines.split(',') {
        let segment = segment.trim();
        if let Some((s, e)) = segment.split_once('-') {
            if let (Ok(start), Ok(end)) = (s.trim().parse::<i64>(), e.trim().parse::<i64>()) {
                let mut i = start;
                while i <= end {
                    result.push(i);
                    i += 1;
                }
            }
        } else if let Ok(num) = segment.parse::<i64>() {
            result.push(num);
        }
    }
    result
}

/// The "To remove this mapping" hint appended to the success message. The TS
/// uses `chalk.gray`, but chalk auto-disables ANSI when stdout is not a TTY
/// (the dispatcher and piped-CLI paths), so we emit plain text for parity with
/// the common non-TTY case.
fn removal_hint(feature_name: &str, scenario: &str, test_file: Option<&str>) -> String {
    let tf = test_file
        .map(|t| format!(" --test-file {t}"))
        .unwrap_or_default();
    format!(
        "\n\nTo remove this mapping:\n  fspec unlink-coverage {feature_name} --scenario \"{scenario}\"{tf}"
    )
}

// ─────────────────────────────────────────────────────────────────────────
// LOCAL update_stats (parity with stats-updater.ts — NOT shared calculate_stats)
// ─────────────────────────────────────────────────────────────────────────

/// Recompute the aggregated `stats` block in place. `totalLinesCovered` =
/// test line ranges (`"N-M"` → M-N+1) + impl array lengths. Initializes a
/// missing `stats` object (BUG-091 fix in TS).
fn update_stats(coverage: &mut Value) {
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
    let total_scenarios_from_arr = scenarios.len() as i64;

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
                let parts: Vec<&str> = lines.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(s), Ok(e)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                        total_test_lines += e - s + 1;
                    }
                }
            }
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

    let top = match coverage.as_object_mut() {
        Some(o) => o,
        None => return,
    };

    // Initialize stats if missing.
    if !top.get("stats").map(Value::is_object).unwrap_or(false) {
        top.insert(
            "stats".to_string(),
            json!({
                "totalScenarios": total_scenarios_from_arr,
                "coveredScenarios": 0,
                "coveragePercent": 0,
                "testFiles": [],
                "implFiles": [],
                "totalLinesCovered": 0,
            }),
        );
    }

    let Some(stats) = top.get_mut("stats").and_then(Value::as_object_mut) else {
        // `stats` was just inserted as an object above; this branch is
        // unreachable in practice but avoids a panic-prone `expect`.
        return;
    };

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
}

/// `implMapping.lines.length` parity: array length for `number[]`, char count
/// for the `"N-M"` string form, 0 otherwise.
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
    fn args_parse_full() {
        let a: LinkCoverageArgs = serde_json::from_str(
            r#"{"featureName":"user-login","scenario":"Login","testFile":"t.ts","testLines":"1-5","implFile":"i.ts","implLines":"10-12","skipValidation":true,"skipStepValidation":false}"#,
        )
        .unwrap();
        assert_eq!(a.feature_name.as_deref(), Some("user-login"));
        assert_eq!(a.scenario.as_deref(), Some("Login"));
        assert_eq!(a.test_lines.as_deref(), Some("1-5"));
        assert!(a.skip_validation);
        assert!(!a.skip_step_validation);
    }

    #[test]
    fn parse_impl_lines_ranges_and_lists() {
        assert_eq!(parse_impl_lines("10-12"), vec![10, 11, 12]);
        assert_eq!(parse_impl_lines("10,11,12"), vec![10, 11, 12]);
        assert_eq!(parse_impl_lines("89-90,316-317"), vec![89, 90, 316, 317]);
    }

    #[test]
    fn extract_work_unit_id_finds_tag() {
        assert_eq!(extract_work_unit_id("@AUTH-001\nFeature: X"), Some("AUTH-001".to_string()));
        assert_eq!(extract_work_unit_id("@wip\nFeature: X"), None);
    }

    #[test]
    fn update_stats_initializes_missing_stats() {
        let mut cov = json!({
            "scenarios": [ { "name": "Login", "testMappings": [
                { "file": "t.ts", "lines": "1-10", "implMappings": [ { "file": "i.ts", "lines": [1,2,3] } ] }
            ] } ]
        });
        update_stats(&mut cov);
        let stats = cov.get("stats").unwrap();
        assert_eq!(stats.get("coveredScenarios").unwrap(), 1);
        assert_eq!(stats.get("coveragePercent").unwrap(), 100);
        assert_eq!(stats.get("totalLinesCovered").unwrap(), 13);
    }

    #[test]
    fn add_test_mapping_appends_and_labels() {
        let mut entry = json!({ "name": "Login", "testMappings": [] });
        let msg = add_test_mapping(&mut entry, "t.ts", "45-62");
        assert!(msg.contains("Linked test mapping"));
        let msg2 = add_test_mapping(&mut entry, "t.ts", "70-80");
        assert!(msg2.contains("Added second test mapping"));
    }
}
