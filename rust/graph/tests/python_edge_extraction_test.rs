// Feature: spec/features/calls-imports-typeref-edges-python.feature
//
// Integration tests for Python Imports and Calls edge extraction.
// Every scenario indexes a temp project with the real pipeline (walk, fact
// extraction, global resolve pass); no edge is hand-written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::resolve::{Reason, RefKindCount};
use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;
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

/// The files `from` has Imports edges to, sorted.
fn imports_of(all: &[GraphEntity], from: &str) -> Vec<String> {
    let mut found: Vec<String> = edges(all, "Imports")
        .into_iter()
        .filter(|(f, _, _)| f == from)
        .map(|(_, to, _)| to)
        .collect();
    found.sort();
    found
}

/// The count of unresolved imports with `reason`.
fn unresolved_imports(indexed: &Indexed, reason: Reason) -> u64 {
    indexed
        .unresolved
        .get(&RefKindCount::Imports)
        .and_then(|reasons| reasons.get(&reason))
        .copied()
        .unwrap_or(0)
}

// ============================================================================
// Scenario: Extract Imports edges from Python import statements
// ============================================================================
#[test]
fn test_python_extract_imports_from_import_statements() -> TestResult {
    // @step Given a Python file with `from click.core import BaseCommand`
    let py_source = r#"
from click.core import BaseCommand
import os

def main():
    cmd = BaseCommand("test")
    return cmd
"#;

    // @step And the target file `click/core.py` exists in the project
    let target_source = r#"
class BaseCommand:
    def __init__(self, name):
        self.name = name
"#;

    // @step When the Python extractor processes the source file
    let (_directory, indexed) =
        index_project(&[("cli/app.py", py_source), ("click/core.py", target_source)])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to `click/core.py`
    assert_eq!(
        imports_of(all, "cli/app.py"),
        ["click/core.py"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // External `os` should NOT produce an edge: it is counted as external.
    assert_eq!(
        unresolved_imports(&indexed, Reason::External),
        1,
        "{:?}",
        indexed.unresolved
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edges from Python function calls
// ============================================================================
#[test]
fn test_python_extract_calls_from_function_calls() -> TestResult {
    let py_source = r#"
def main():
    config = validate_config()
    return config

def validate_config():
    return {"valid": True}
"#;

    // @step Given a Python file with `def main():` that calls `validate_config()`
    // @step And `validate_config` is defined in the same file
    // @step When the Python extractor processes the source file
    let (_directory, indexed) = index_project(&[("app.py", py_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `main` to `validate_config`
    let main = slug_of(all, "Function", "app.py::main")?;
    assert_eq!(
        callees_of(all, &main),
        ["app.py::validate_config"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}
