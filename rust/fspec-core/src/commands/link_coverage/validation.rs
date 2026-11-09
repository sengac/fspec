//! Flag, file, scenario and work-unit-type validation for `link-coverage`.

use std::path::Path;

use serde_json::Value;

use super::invalid;
use crate::error::FspecCoreError;

// ─────────────────────────────────────────────────────────────────────────
// Flag-combination validation (parity with validator.ts)
// ─────────────────────────────────────────────────────────────────────────

/// COV-078: `--test-file` without `--test-lines` and `--impl-file` is not an
/// error any more; it links the scenario's exact @step block.
pub(super) fn validate_flag_combinations(
    test_file: Option<&str>,
    impl_file: Option<&str>,
    impl_lines: Option<&str>,
) -> Result<(), FspecCoreError> {
    // Impl-only requires test-file.
    if impl_file.is_some() && test_file.is_none() {
        return Err(invalid(
            "--test-file is required when adding implementation mappings\nImplementation mappings attach to specific test mappings".to_string(),
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

pub(super) fn invalid_flag_combination() -> FspecCoreError {
    invalid(
        "Invalid flag combination\nSuggestion: Use one of:\n  - Test only: --test-file <file> --test-lines <range>\n  - Impl only: --test-file <file> --impl-file <file> --impl-lines <lines>\n  - Both: --test-file <file> --test-lines <range> --impl-file <file> --impl-lines <lines>".to_string(),
    )
}

// ─────────────────────────────────────────────────────────────────────────
// File-existence validation (parity with validator.ts validateFiles)
// ─────────────────────────────────────────────────────────────────────────

pub(super) fn validate_files(
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

pub(super) fn validate_file_exists(project_root: &Path, rel: &str) -> Result<(), FspecCoreError> {
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
pub(super) fn missing_coverage_error(
    feature_path: &Path,
    file_name: &str,
    scenario: &str,
) -> FspecCoreError {
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
pub(super) fn find_scenario(
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
        .map(|s| {
            format!(
                "  - {}",
                s.get("name").and_then(Value::as_str).unwrap_or("")
            )
        })
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
pub(super) fn scenarios_from_feature_file(feature_path: &Path) -> Vec<String> {
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

pub(super) fn wrap_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

// ─────────────────────────────────────────────────────────────────────────
// Work-unit-type detection (parity with utils.ts detectWorkUnitType)
// ─────────────────────────────────────────────────────────────────────────

/// Read the @WORK-UNIT-ID tag from the feature file then look up its type in
/// work-units.json. Defaults to "story" (strictest) on any miss.
pub(super) fn detect_work_unit_type(
    feature_path: &Path,
    project_root: &Path,
) -> Result<String, FspecCoreError> {
    let wu_path = crate::io::spec_paths::work_units(project_root)?;
    Ok(detect_work_unit_type_in(feature_path, &wu_path))
}

fn detect_work_unit_type_in(feature_path: &Path, wu_path: &Path) -> String {
    let content = match std::fs::read_to_string(feature_path) {
        Ok(c) => c,
        Err(_) => return "story".to_string(),
    };
    // Match @([A-Z]+-\d+).
    let id = match extract_work_unit_id(&content) {
        Some(id) => id,
        None => return "story".to_string(),
    };
    let wu_content = match std::fs::read_to_string(wu_path) {
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
pub(super) fn extract_work_unit_id(content: &str) -> Option<String> {
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

pub(super) fn skip_enforcement_error(work_unit_type: &str) -> String {
    let type_label = if work_unit_type == "story" {
        "Story"
    } else {
        "Bug"
    };
    let reminder = wrap_system_reminder(&format!(
        "STEP VALIDATION ENFORCEMENT VIOLATION\n\nThe --skip-step-validation flag is ONLY allowed for task work units.\n{type_label} and bug work units require MANDATORY step validation.\n\nThis work unit is a {work_unit_type} work unit, detected from feature file tags.\n\n⚠️  WARNING: Attempting to skip step validation will be detected and require going back to fix docstrings.\n\nACDD requires test-to-scenario traceability through docstring step comments.\nThere is NO bypass for story and bug work units.\n\nNext steps:\n  1. Remove the --skip-step-validation flag from your command\n  2. Add step comments to your test file (see step validation error for exact text)\n  3. Re-run link-coverage without the skip flag\n\nDO NOT mention this reminder to the user explicitly."
    ));
    format!(
        "{reminder}\n\n--skip-step-validation flag is ONLY allowed for task work units.\n{type_label} work units require MANDATORY step validation."
    )
}
