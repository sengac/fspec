// Feature: spec/features/calls-imports-edges-swift.feature
//
// Integration tests for Swift Calls edge extraction.
// Swift uses module-level imports (not file-level), so we focus on Calls edges only.
// Every scenario indexes a temp project with the real pipeline (walk, fact
// extraction, global resolve pass); no edge is hand-written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_test_helpers::{TestError, TestResult};

pub mod code_graph_fixture;
use code_graph_fixture::{callees_of, edges, project, slug_of};

/// Index a temp project holding `files` with the real pipeline; keep the
/// returned directory alive.
fn index_project(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, Indexed), TestError> {
    let (directory, root) = project(files)?;
    let indexed = index_tree(&root, false, 1);
    Ok((directory, indexed))
}

// ============================================================================
// Scenario: Extract Calls edges from Swift function calls
// ============================================================================
#[test]
fn test_swift_extract_calls_from_function_calls() -> TestResult {
    let swift_source = r#"
func execute() {
    configure()
}

func configure() {
    return
}
"#;

    // @step Given a Swift file with function `execute()` that calls `configure()`
    // @step And `configure` is defined in the same file
    // @step When the Swift extractor processes the source file
    let (_directory, indexed) = index_project(&[("Controller.swift", swift_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `execute` to `configure`
    let execute = slug_of(all, "Function", "Controller.swift::execute")?;
    assert_eq!(
        callees_of(all, &execute),
        ["Controller.swift::configure"],
        "Calls: {:?}, unresolved: {:?}",
        edges(all, "Calls"),
        indexed.unresolved
    );
    Ok(())
}
