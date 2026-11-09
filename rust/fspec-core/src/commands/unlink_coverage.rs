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

//!
//! ## REFAC-020 indexed selection
//!
//! `testMappingIndex` paired with `expectedScenarioHash` removes exactly one
//! row (or one implementation mapping of that row with `implFile`) of a
//! uniquely named scenario; it cannot be combined with `all`. All modes read,
//! check and write inside the shared sidecar transaction.

use std::path::Path;

use serde_json::{json, Value};

use super::coverage_lineage::{
    scenario_hash, sidecar_paths, unique_scenario_index, Selector, SidecarTransaction,
};
use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

use serde::Deserialize;

mod mapping;

use mapping::{carrying_row, first_row, remove_impl_from_row, removed_row_message, test_mappings};
// BUG-608: the one stats definition (link-coverage's).
use crate::commands::link_coverage::stats::update_stats;

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
    #[serde(default)]
    test_mapping_index: Option<Value>,
    #[serde(default)]
    expected_scenario_hash: Option<Value>,
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
    let selector = Selector::parse(
        args.test_mapping_index.as_ref(),
        args.expected_scenario_hash.as_ref(),
        "unlink-coverage",
    )?;

    // Validate flag combinations (parity with the TS guards).
    if selector.is_some() && (all || test_file.is_none()) {
        return Err(invalid(
            "--test-mapping-index requires --test-file and cannot be combined with --all"
                .to_string(),
        ));
    }
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
    let paths = sidecar_paths(project_root, feature_name);
    let (file_name, coverage_path) = (&paths.file_name, &paths.coverage_path);
    let not_found = || {
        invalid(format!(
            "Coverage file not found: {file_name}.coverage\nSuggestion: Run fspec show-coverage to see available features"
        ))
    };
    if !coverage_path.is_file() {
        return Err(not_found());
    }

    // TS reads the file and treats any failure (missing OR unreadable OR bad
    // JSON) as "Coverage file not found". The read happens inside the shared
    // sidecar transaction.
    let _transaction = SidecarTransaction::acquire(coverage_path, "unlink-coverage")?;
    let raw = std::fs::read_to_string(coverage_path).map_err(|_| not_found())?;
    let mut coverage: Value = serde_json::from_str(&raw).map_err(|_| not_found())?;

    // Locate the scenario by name within `coverage.scenarios`.
    let scenario_idx = match &selector {
        Some(_) => unique_scenario_index(&coverage, scenario, "unlink-coverage")?,
        None => legacy_scenario_index(&coverage, scenario)?,
    };

    // Mutate the scenario entry in place on the live `coverage` Value.
    let scenarios_arr = coverage
        .get_mut("scenarios")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("Scenario not found".to_string()))?;
    let scenario_entry = &mut scenarios_arr[scenario_idx];

    let message = if let (Some(selector), Some(tf)) = (&selector, test_file) {
        // Indexed: exactly one fresh, exact-file row (or one of its impls).
        selector.verify(scenario_entry, tf, "unlink-coverage")?;
        let rows = test_mappings(scenario_entry, "findIndex")?;
        match impl_file {
            Some(imf) => remove_impl_from_row(&mut rows[selector.index], imf, scenario)?,
            None => {
                rows.remove(selector.index);
                removed_row_message(tf, scenario)
            }
        }
    } else if all {
        // Mode 1: remove all mappings.
        scenario_entry
            .as_object_mut()
            .map(|o| o.insert("testMappings".to_string(), Value::Array(vec![])));
        format!("\u{2713} Removed all coverage mappings for scenario \"{scenario}\"")
    } else if let (Some(tf), Some(imf)) = (test_file, impl_file) {
        // Mode 2: remove only the implementation mapping.
        let rows = test_mappings(scenario_entry, "find")?;
        let tm_pos = first_row(rows, tf)?;
        // BUG-502: the test file's mapping that carries the implementation
        // file; when none does, the first mapping reports it as not found.
        let tm_pos = carrying_row(rows, tf, imf).unwrap_or(tm_pos);
        remove_impl_from_row(&mut rows[tm_pos], imf, scenario)?
    } else if let Some(tf) = test_file {
        // Mode 3: remove the entire test mapping (and its impl mappings).
        let rows = test_mappings(scenario_entry, "findIndex")?;
        let tm_pos = first_row(rows, tf)?;
        rows.remove(tm_pos);
        removed_row_message(tf, scenario)
    } else {
        // Unreachable: the flag-combination guards above reject this case.
        return Err(invalid(
            "Must specify either --all or --test-file".to_string(),
        ));
    };

    // BUG-608: recompute stats with the one definition every writer uses.
    update_stats(&mut coverage);

    // Write back with `JSON.stringify(data, null, 2)` byte parity (no trailing
    // newline). `preserve_order` keeps the original top-level key ordering.
    write_json_atomic(coverage_path, &coverage)?;

    let mut envelope = json!({ "success": true, "message": message });
    if selector.is_some() {
        envelope["scenarioHash"] = json!(scenario_hash(&coverage["scenarios"][scenario_idx]));
    }
    serde_json::to_string_pretty(&envelope)
        .map_err(|e| invalid(format!("failed to serialize result: {e}")))
}

/// Legacy first-match scenario lookup with the available-scenarios listing.
fn legacy_scenario_index(coverage: &Value, scenario: &str) -> Result<usize, FspecCoreError> {
    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(i) = scenarios
        .iter()
        .position(|s| s.get("name").and_then(Value::as_str) == Some(scenario))
    {
        return Ok(i);
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
    Err(invalid(format!(
        "Scenario not found: \"{scenario}\"\nAvailable scenarios:\n{available}"
    )))
}

#[cfg(test)]
mod tests;
