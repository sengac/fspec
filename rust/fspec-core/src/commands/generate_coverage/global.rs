//! Global (all-feature) sidecar create/update for `generate-coverage`.

use super::*;

/// Process a single feature file, mutating / creating its sidecar as needed.
/// With `apply == false` only the resulting status is computed (no writes),
/// so callers can take the sidecar transaction only when a write is needed.
pub(super) fn process_feature(
    feature_path: &Path,
    coverage_path: &Path,
    apply: bool,
) -> Result<Status, FspecCoreError> {
    let feature_content = std::fs::read_to_string(feature_path)
        .map_err(|e| invalid(format!("Failed to read feature file: {e}")))?;

    // BUG-563: a Rule block would hide its scenarios.
    crate::io::gherkin::reject_rule_blocks(feature_path, "generate-coverage")?;
    // Parse the feature to extract scenario names (in file order).
    let feature = parse_feature_lenient(&feature_content)
        .map_err(|e| invalid(format!("Failed to parse feature file: {e}")))?;
    let scenario_names: Vec<String> = feature.scenarios.iter().map(|s| s.name.clone()).collect();

    match std::fs::read_to_string(coverage_path) {
        Ok(existing) => {
            // Sidecar exists — validate JSON.
            match serde_json::from_str::<Value>(&existing) {
                Ok(coverage) => {
                    // Valid JSON: update if scenario set drifted.
                    let (rewritten, change) =
                        update_coverage(coverage_path, &coverage, &scenario_names, apply)?;
                    if rewritten {
                        Ok(Status::Updated(change))
                    } else {
                        Ok(Status::Skipped {
                            blocked: change.blocked,
                        })
                    }
                }
                Err(_) => {
                    // Invalid JSON: recreate from scratch.
                    if apply {
                        write_fresh_coverage(coverage_path, &scenario_names)?;
                    }
                    Ok(Status::Recreated)
                }
            }
        }
        Err(_) => {
            // No sidecar: create a fresh one.
            if apply {
                write_fresh_coverage(coverage_path, &scenario_names)?;
            }
            Ok(Status::Created)
        }
    }
}

/// Write a brand-new coverage sidecar with one empty scenario mapping per
/// scenario and zeroed stats — mirrors TS `writeCoverageFile`.
pub(super) fn write_fresh_coverage(
    coverage_path: &Path,
    scenario_names: &[String],
) -> Result<(), FspecCoreError> {
    let scenarios: Vec<Value> = scenario_names
        .iter()
        .map(|name| json!({ "name": name, "testMappings": [] }))
        .collect();
    let mut coverage = json!({ "scenarios": scenarios });
    // BUG-608: the one stats definition.
    crate::commands::link_coverage::stats::update_stats(&mut coverage);
    write_json_atomic(coverage_path, &coverage)
}

/// Update an existing (valid) sidecar in place on a `serde_json::Value`,
/// preserving existing test mappings and unknown fields. Returns whether the
/// file needs rewriting (scenario set drifted) plus the change.
///
/// BUG-301: unknown top-level fields are preserved, and a stale scenario that
/// still carries test mappings is kept and reported as blocked rather than
/// deleted (resolve it with selected mode, `--feature-name`).
pub(super) fn update_coverage(
    coverage_path: &Path,
    existing: &Value,
    feature_scenario_names: &[String],
    apply: bool,
) -> Result<(bool, Change), FspecCoreError> {
    let existing_scenarios = existing
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let name_of = |s: &Value| s.get("name").and_then(Value::as_str).map(String::from);
    let has_mappings = |s: &Value| {
        s.get("testMappings")
            .and_then(Value::as_array)
            .is_some_and(|mappings| !mappings.is_empty())
    };
    let existing_names: Vec<String> = existing_scenarios.iter().filter_map(name_of).collect();

    let mut change = Change::default();
    // Kept scenarios: still in the feature file, or stale but mapped. Order:
    // existing order first (parity with TS `[...keptScenarios, ...newScenarios]`).
    let mut kept: Vec<Value> = Vec::new();
    for scenario in &existing_scenarios {
        let name = name_of(scenario).unwrap_or_default();
        if feature_scenario_names.contains(&name) {
            kept.push(scenario.clone());
        } else if has_mappings(scenario) {
            change.blocked.push(name);
            kept.push(scenario.clone());
        } else {
            change.removed.push(name);
        }
    }

    // New scenarios: feature scenarios not already present in the sidecar.
    for name in feature_scenario_names {
        if !existing_names.contains(name) {
            change.added.push(name.clone());
            kept.push(json!({ "name": name, "testMappings": [] }));
        }
    }

    if change.added.is_empty() && change.removed.is_empty() {
        return Ok((false, change));
    }

    let mut output = existing.as_object().cloned().unwrap_or_default();
    output.insert("scenarios".to_string(), Value::Array(kept));
    // BUG-608: recompute every stats field with the one stats definition,
    // keeping any other stats members.
    let mut output = Value::Object(output);
    crate::commands::link_coverage::stats::update_stats(&mut output);

    // TS writes `JSON.stringify(updatedCoverage, null, 2)` (no trailing
    // newline) via plain writeFile — use the non-newline atomic writer.
    if apply {
        write_json_atomic(coverage_path, &output)?;
    }
    Ok((true, change))
}
