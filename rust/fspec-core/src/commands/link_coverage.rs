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
//!   * test-only  → `--test-file` + `--test-lines` (COV-078: without
//!     `--test-lines`, the scenario's exact @step block found once).
//!   * impl-only  → `--test-file` + `--impl-file` + `--impl-lines`.
//!   * both       → all four flags.
//!
//! REFAC-020 adds indexed impl-only selection: `testMappingIndex` paired with
//! `expectedScenarioHash` targets one exact row of a uniquely named scenario,
//! re-verified inside the shared sidecar transaction. Every mode reads, checks
//! and writes while holding that transaction.
//!
//! Returns a JSON envelope `{ "success": true, "message": "<...>", "warnings"?,
//! "scenarioHash"? }`. The CLI bridge surfaces `message` (and yellow
//! `warnings`) on stdout; all mutation / validation / rendering logic lives
//! here.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use super::coverage_lineage::{
    scenario_hash, sidecar_paths, unique_scenario_index, Selector, SidecarTransaction,
};
use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

mod mapping;
pub(crate) mod stats;
pub(crate) mod step;
mod validation;

#[cfg(test)]
use mapping::parse_impl_lines;
pub(crate) use mapping::relink_test_mapping;
use mapping::removal_hint;
use mapping::{
    add_both_mappings, add_impl_mapping, add_impl_to_row, add_test_mapping, replace_test_mapping,
};
use stats::update_stats;
#[cfg(test)]
use validation::extract_work_unit_id;
use validation::{
    detect_work_unit_type, find_scenario, invalid_flag_combination, missing_coverage_error,
    skip_enforcement_error, validate_files, validate_flag_combinations,
};

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
    #[serde(default)]
    test_mapping_index: Option<Value>,
    #[serde(default)]
    expected_scenario_hash: Option<Value>,
    /// BUG-502: replace the scenario's mappings of `test_file` instead of
    /// adding one.
    #[serde(default)]
    replace: bool,
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
    let impl_file = args.impl_file.as_deref();
    let impl_lines = args.impl_lines.as_deref();

    // ---- 1. Validate flag combinations (parity with validator.ts) ----
    validate_flag_combinations(test_file, impl_file, impl_lines)?;
    let selector = Selector::parse(
        args.test_mapping_index.as_ref(),
        args.expected_scenario_hash.as_ref(),
        "link-coverage",
    )?;
    if selector.is_some() {
        validate_indexed_mode(&args)?;
    }

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
    let paths = sidecar_paths(project_root, feature_name);
    let (file_name, coverage_path, feature_path) =
        (&paths.file_name, &paths.coverage_path, &paths.feature_path);
    if !coverage_path.is_file() {
        return Err(missing_coverage_error(feature_path, file_name, scenario));
    }
    // BUG-563: a Rule block would hide its scenarios.
    crate::io::gherkin::reject_rule_blocks(feature_path, "link-coverage")?;

    // ---- 3b. COV-078: a test link without --test-lines (no implementation,
    // no indexed selection) links the scenario's exact @step block found once.
    let block_lines = match (test_file, &args.test_lines, impl_file) {
        (Some(tf), None, None) if selector.is_none() => Some(step::exact_block_lines(
            feature_path,
            scenario,
            project_root,
            tf,
        )?),
        _ => None,
    };
    let test_lines = args.test_lines.as_deref().or(block_lines.as_deref());
    if args.replace {
        validate_replace_mode(&args, test_lines)?;
    }

    // ---- 4. Load coverage file inside the sidecar transaction ----
    let _transaction = SidecarTransaction::acquire(coverage_path, "link-coverage")?;
    let raw = std::fs::read_to_string(coverage_path);
    let mut coverage: Value = match raw {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|_| missing_coverage_error(feature_path, file_name, scenario))?,
        Err(_) => {
            return Err(missing_coverage_error(feature_path, file_name, scenario));
        }
    };

    // ---- 5. Find the scenario (indexed selection requires uniqueness) ----
    let scenario_idx = match &selector {
        Some(_) => unique_scenario_index(&coverage, scenario, "link-coverage")?,
        None => find_scenario(&coverage, scenario, feature_path)?,
    };

    // ---- 6. skip-step-validation enforcement (story/bug forbidden) ----
    if test_file.is_some() && args.skip_step_validation {
        let work_unit_type = detect_work_unit_type(feature_path, project_root)?;
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
        // BUG-452: the scenario's existing ranges in this test file count
        // together with the linked range.
        let rows = coverage
            .get("scenarios")
            .and_then(Value::as_array)
            .and_then(|scenarios| scenarios.get(scenario_idx))
            .and_then(|entry| entry.get("testMappings"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                invalid(format!(
                    "the coverage entry for scenario '{scenario}' has no testMappings array"
                ))
            })?;
        let existing: Vec<String> = rows
            .iter()
            .filter(|row| row.get("file").and_then(Value::as_str) == Some(tf))
            .filter_map(|row| row.get("lines").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        // BUG-502: --replace needs a mapping to replace, and the replaced
        // ranges no longer count towards the step check.
        if args.replace && !rows.iter().any(|row| row_file_is(row, tf)) {
            return Err(no_mapping_to_replace(tf, scenario));
        }
        let existing = if args.replace { Vec::new() } else { existing };
        step::validate_step_consistency(
            feature_path,
            scenario,
            project_root,
            tf,
            test_lines,
            &existing,
            args.skip_step_validation,
        )?;
    }

    // ---- 7b. COV-079: record the implementation mapping's line text,
    // refusing a blank or non-unique first line. A file that cannot be read
    // (only possible with --skip-validation) records none.
    let impl_text = match (impl_file, impl_lines) {
        (Some(imf), Some(iml)) => match std::fs::read_to_string(project_root.join(imf)) {
            Ok(content) => Some(
                crate::commands::audit::impl_ranges::record_text(
                    imf,
                    &content,
                    &mapping::parse_impl_lines(iml),
                )
                .map_err(invalid)?,
            ),
            Err(_) => None,
        },
        _ => None,
    };
    let impl_text = impl_text.as_ref();

    // ---- 8. Perform the mutation on the scenario entry ----
    let message = {
        let scenarios = coverage
            .get_mut("scenarios")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| invalid("Scenario not found".to_string()))?;
        let entry = &mut scenarios[scenario_idx];

        if let (Some(selector), Some(tf), Some(imf), Some(iml)) =
            (&selector, test_file, impl_file, impl_lines)
        {
            // Indexed impl-only: exactly one fresh, exact-file row.
            selector.verify(entry, tf, "link-coverage")?;
            let row = entry
                .get_mut("testMappings")
                .and_then(Value::as_array_mut)
                .and_then(|rows| rows.get_mut(selector.index))
                .ok_or_else(|| invalid("Selected test mapping disappeared".to_string()))?;
            add_impl_to_row(row, imf, iml, impl_text)?
        } else if let (true, Some(tf), Some(tl)) = (args.replace, test_file, test_lines) {
            // BUG-502: replace the test file's mappings, keeping their impls.
            let replaced = replace_test_mapping(entry, tf, tl)
                .ok_or_else(|| no_mapping_to_replace(tf, scenario))?;
            match (impl_file, impl_lines) {
                (Some(imf), Some(iml)) => {
                    let row = entry
                        .get_mut("testMappings")
                        .and_then(Value::as_array_mut)
                        .and_then(|rows| rows.get_mut(replaced.index))
                        .ok_or_else(|| invalid("Replaced test mapping disappeared".to_string()))?;
                    format!(
                        "{}\n{}",
                        replaced.message,
                        add_impl_to_row(row, imf, iml, impl_text)?
                    )
                }
                _ => replaced.message,
            }
        } else if let (Some(tf), Some(tl)) = (test_file, test_lines) {
            if impl_file.is_none() {
                // Mode 1: test-only
                add_test_mapping(entry, tf, tl)
            } else if let (Some(imf), Some(iml)) = (impl_file, impl_lines) {
                // Mode 3: both
                add_both_mappings(entry, tf, tl, imf, iml, impl_text)?
            } else {
                return Err(invalid_flag_combination());
            }
        } else if let (Some(tf), Some(imf), Some(iml)) = (test_file, impl_file, impl_lines) {
            // Mode 2: impl-only (test_lines absent)
            add_impl_mapping(entry, tf, imf, iml, impl_text)?
        } else {
            return Err(invalid_flag_combination());
        }
    };

    // ---- 9. Recalculate stats (LOCAL update_stats) ----
    update_stats(&mut coverage);

    // ---- 10. Atomic write-back (2-space JSON, no trailing newline) ----
    write_json_atomic(coverage_path, &coverage)?;

    // ---- 11. Build the envelope ----
    let full_message = format!(
        "{message}{}",
        removal_hint(feature_name, scenario, test_file)
    );
    let mut envelope = json!({ "success": true, "message": full_message });
    if !warnings.is_empty() {
        envelope["warnings"] = json!(warnings.join("\n"));
    }
    if selector.is_some() {
        envelope["scenarioHash"] = json!(scenario_hash(&coverage["scenarios"][scenario_idx]));
    }
    serde_json::to_string_pretty(&envelope)
        .map_err(|e| invalid(format!("failed to serialize result: {e}")))
}

/// BUG-502: `--replace` swaps the lines of an existing test mapping.
fn validate_replace_mode(
    args: &LinkCoverageArgs,
    test_lines: Option<&str>,
) -> Result<(), FspecCoreError> {
    if args.test_file.is_none() || test_lines.is_none() {
        return Err(invalid(
            "--replace requires --test-file and --test-lines".to_string(),
        ));
    }
    if args.test_mapping_index.is_some() {
        return Err(invalid(
            "--replace cannot be combined with --test-mapping-index".to_string(),
        ));
    }
    Ok(())
}

fn row_file_is(row: &Value, test_file: &str) -> bool {
    row.get("file").and_then(Value::as_str) == Some(test_file)
}

fn no_mapping_to_replace(test_file: &str, scenario: &str) -> FspecCoreError {
    invalid(format!(
        "Scenario \"{scenario}\" has no mapping of {test_file} to replace\nSuggestion: Link it without --replace"
    ))
}

/// Indexed selection is implementation-only and never weakens validation.
fn validate_indexed_mode(args: &LinkCoverageArgs) -> Result<(), FspecCoreError> {
    if args.test_file.is_none() || args.impl_file.is_none() || args.impl_lines.is_none() {
        return Err(invalid(
            "--test-mapping-index requires --test-file, --impl-file and --impl-lines".to_string(),
        ));
    }
    if args.test_lines.is_some() {
        return Err(invalid(
            "--test-mapping-index selects an existing row and cannot be combined with --test-lines"
                .to_string(),
        ));
    }
    if args.skip_validation || args.skip_step_validation {
        return Err(invalid(
            "--test-mapping-index cannot be combined with --skip-validation or --skip-step-validation"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
