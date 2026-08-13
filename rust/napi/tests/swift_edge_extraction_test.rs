// Feature: spec/features/calls-edges-swift.feature
//
// Integration tests for Swift Calls edge extraction.
// Swift uses module-level imports (not file-level), so we focus on Calls edges only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_swift_extractor::extract_swift;

mod graph_test_helpers;
use graph_test_helpers::find_edges;

// ============================================================================
// Scenario: Extract Calls edges from Swift function calls
// ============================================================================
#[test]
fn test_swift_extract_calls_from_function_calls() {
    let swift_source = r#"
func handle() {
    process()
}

func process() {
    return
}
"#;
    let known_files = HashSet::new();

    // @step Given a Swift file with function `handle()` that calls `process()`
    // @step And `process` is defined in the same file
    // @step When the Swift extractor processes the source file
    let entities = extract_swift(swift_source, "Controller.swift", &known_files)
        .expect("Swift extraction should succeed");

    // @step Then a Calls edge should be emitted from `handle` to `process`
    let calls = find_edges(&entities, "Calls", Some("handle"), Some("process"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from handle to process. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}
