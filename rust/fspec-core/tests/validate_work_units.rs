#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-work-units-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `validate-work-units`
// (RPC-325). Each scenario maps to exactly one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch returns
// FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate-work-units".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_raw_work_units(root: &Path, raw: &str) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn write_work_units(root: &Path, value: &Value) {
    write_raw_work_units(root, &serde_json::to_string_pretty(value).unwrap());
}

fn parse_data(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{}", result.data))
}

fn errors(data: &Value) -> Vec<String> {
    data["errors"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn has_error(data: &Value, needle: &str) -> bool {
    errors(data).iter().any(|e| e.contains(needle))
}

fn has_exact_error(data: &Value, exact: &str) -> bool {
    errors(data).iter().any(|e| e == exact)
}

// ---------- scenarios ----------

#[path = "validate_work_units/dispatcher_flags_a_work.rs"]
mod dispatcher_flags_a_work;
#[path = "validate_work_units/dispatcher_reports_a_clean.rs"]
mod dispatcher_reports_a_clean;
#[path = "validate_work_units/stable_index_items.rs"]
mod stable_index_items;
