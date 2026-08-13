// Feature: spec/features/calls-imports-typeref-edges-python.feature
//
// Integration tests for Python Imports and Calls edge extraction.
// Uses real Python source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_python_extractor::extract_python;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Python import statements
// ============================================================================
#[test]
fn test_python_extract_imports_from_import_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python file with `from click.core import BaseCommand`
    let py_source = r#"
from click.core import BaseCommand
import os

def main():
    cmd = BaseCommand("test")
    return cmd
"#;
    write_test_file(project_dir, "cli/app.py", py_source);

    // @step And the target file `click/core.py` exists in the project
    let target_source = r#"
class BaseCommand:
    def __init__(self, name):
        self.name = name
"#;
    write_test_file(project_dir, "click/core.py", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Python extractor processes the source file
    let entities = extract_python(py_source, "cli/app.py", &known_files)
        .expect("Python extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to `click/core.py`
    let imports = find_edges(
        &entities,
        "Imports",
        Some("cli-app-py"),
        Some("click-core-py"),
    );
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to click/core.py. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // External `os` should NOT produce an edge
    let os_imports = find_edges(&entities, "Imports", None, Some("os"));
    assert!(
        os_imports.is_empty(),
        "External 'os' imports should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Python function calls
// ============================================================================
#[test]
fn test_python_extract_calls_from_function_calls() {
    let py_source = r#"
def main():
    config = validate_config()
    return config

def validate_config():
    return {"valid": True}
"#;
    let known_files = HashSet::new();

    // @step Given a Python file with `def main():` that calls `validate_config()`
    // @step And `validate_config` is defined in the same file
    // @step When the Python extractor processes the source file
    let entities = extract_python(py_source, "app.py", &known_files)
        .expect("Python extraction should succeed");

    // @step Then a Calls edge should be emitted from `main` to `validate_config`
    let calls = find_edges(&entities, "Calls", Some("main"), Some("validate_config"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from main to validate_config. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}
