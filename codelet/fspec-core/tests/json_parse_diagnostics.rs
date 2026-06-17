#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/use-native-serde-json-errors-with-good-diagnostics-for-malformed-json-remove-v8-style-emulation.feature
//
// RPC-334: malformed canonical state files surface a caret-pointed diagnostic
// (rendered by codelet-fspec-json-error) instead of a bare serde message or
// the fabricated `Unexpected token in JSON:` V8-emulation prefix. Each #[test]
// maps to exactly one Gherkin scenario with @step comments mirroring it.

use std::fs;
use std::path::Path;

use codelet_fspec_core::io::json_error::parse_json_reason;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::Value;
use tempfile::TempDir;

/// A multi-line work-units.json whose line 4 has an unquoted key `status:`
/// (serde reports `key must be a string at line 4 column 37`).
const MALFORMED_WORK_UNITS: &str =
    "{\n  \"version\": \"0.7.1\",\n  \"workUnits\": {\n    \"AUTH-001\": { \"id\": \"AUTH-001\", status: \"done\" }\n  }\n}";

fn seed(project_root: &Path, file: &str, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join(file), raw).expect("write state file");
}

fn req(command: &str, project_root: &Path) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: "{}".to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn req_args(command: &str, project_root: &Path, args_json: &str) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shared funnel surfaces a caret-pointed diagnostic naming the file
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn scenario_shared_funnel_surfaces_caret_pointed_diagnostic_naming_the_file() {
    // @step Given a work-units.json whose line 4 contains an unquoted key "status:" at column 37
    let tmp = TempDir::new().expect("tempdir");
    seed(tmp.path(), "work-units.json", MALFORMED_WORK_UNITS);

    // @step When the file is read through the shared read_or_init_json funnel
    // (list-work-units loads work-units.json via ensure_work_units_file →
    //  read_or_init_json, the shared funnel)
    let result = dispatch_command(req("list-work-units", tmp.path()));
    assert!(!result.success, "expected failure, got {result:?}");
    let msg = result.error.expect("error message expected");

    // @step Then the error message contains "Failed to parse work-units.json"
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "missing file framing: {msg}"
    );
    // @step And the error message contains "the file may be corrupted or contain invalid JSON."
    assert!(
        msg.contains("The file may be corrupted or contain invalid JSON."),
        "missing corruption guidance: {msg}"
    );
    // @step And the error message contains the offending source line
    assert!(msg.contains("status:"), "missing offending source line: {msg}");
    // @step And the error message contains a caret line with "key must be a string at line 4 column 37"
    assert!(msg.contains('^'), "missing caret: {msg}");
    assert!(
        msg.contains("key must be a string at line 4 column 37"),
        "missing serde position: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: auto-advance keeps its outer wrapper but drops the fabricated V8 prefix
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn scenario_auto_advance_keeps_wrapper_but_drops_fabricated_v8_prefix() {
    // @step Given a corrupt work-units.json
    let tmp = TempDir::new().expect("tempdir");
    seed(tmp.path(), "work-units.json", MALFORMED_WORK_UNITS);

    // @step When I run the auto-advance command against it
    let result = dispatch_command(req("auto-advance", tmp.path()));
    assert!(!result.success, "expected failure, got {result:?}");
    let msg = result.error.expect("error message expected");

    // @step Then the error message contains "Failed to auto-advance:"
    assert!(
        msg.contains("Failed to auto-advance:"),
        "missing command wrapper: {msg}"
    );
    // @step And the error message contains the serde caret snippet
    assert!(msg.contains('^'), "missing caret snippet: {msg}");
    assert!(
        msg.contains("key must be a string at line 4 column 37"),
        "missing serde position: {msg}"
    );
    // @step And the error message does not contain "Unexpected token in JSON:"
    assert!(
        !msg.contains("Unexpected token in JSON:"),
        "must not emit fabricated V8 prefix: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Single-line input places the caret under the exact error column
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn scenario_single_line_input_places_caret_under_exact_error_column() {
    // @step Given a single-line JSON input "{ bad"
    let input = "{ bad";
    let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();

    // @step When the input is rendered by the shared diagnostic helper
    let snippet = parse_json_reason(input, &err);

    // @step Then the snippet contains " 1 | { bad"
    assert!(snippet.contains(" 1 | { bad"), "missing numbered line: {snippet}");
    // @step And the snippet contains a caret under column 3
    assert!(snippet.contains('^'), "missing caret: {snippet}");
    // @step And the snippet contains "key must be a string at line 1 column 3"
    assert!(
        snippet.contains("key must be a string at line 1 column 3"),
        "missing serde position: {snippet}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Lenient parse sites stay silent on malformed input
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn scenario_lenient_parse_sites_stay_silent_on_malformed_input() {
    // @step Given a malformed fspec-hooks.json
    let tmp = TempDir::new().expect("tempdir");
    seed(tmp.path(), "fspec-hooks.json", "{ not json");

    // @step When I run the list-hooks command against it
    let result = dispatch_command(req_args("list-hooks", tmp.path(), r#"{"format":"json"}"#));

    // @step Then an empty hook list is returned
    assert!(
        result.success,
        "lenient parse must swallow the error; got {result:?}"
    );
    let data: Value =
        serde_json::from_str(&result.data).expect("dispatch data should be valid JSON");
    assert_eq!(
        data["events"].as_array().map(Vec::len),
        Some(0),
        "expected empty events array on malformed input, got {}",
        result.data
    );

    // @step And no parse error is surfaced
    assert!(
        result.error.is_none(),
        "lenient parse site must not surface a parse error: {result:?}"
    );
    assert!(
        !result.data.contains("Failed to parse"),
        "lenient parse site must not surface file framing: {}",
        result.data
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A serde error with no location falls back to the bare message
// ─────────────────────────────────────────────────────────────────────────
#[test]
fn scenario_serde_error_with_no_location_falls_back_to_bare_message() {
    // @step Given a serde_json error that carries no line or column
    // (an empty document — serde reports "EOF while parsing a value" whose
    //  rendered window is empty, so no caret can be drawn)
    let input = "";
    let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();

    // @step When the input is rendered by the shared diagnostic helper
    let snippet = parse_json_reason(input, &err);

    // @step Then the rendered output is the bare serde message
    assert!(
        snippet.contains("EOF while parsing"),
        "expected bare serde message: {snippet}"
    );
    // @step And no caret line is produced
    assert!(!snippet.contains('^'), "must not draw a caret: {snippet}");
}
