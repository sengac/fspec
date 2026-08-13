// Feature: spec/features/calls-imports-edges-c-cpp.feature
//
// Integration tests for C and C++ Imports and Calls edge extraction.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_c_extractor::extract_c;
use codelet_napi::graph::ast_pipeline::ast_cpp_extractor::extract_cpp;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from C include directives
// ============================================================================
#[test]
fn test_c_extract_imports_from_include_directives() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a C file with `#include "jv.h"`
    let c_source = r#"
#include "jv.h"
#include <stdio.h>

int main() {
    jv_parse("test");
    return 0;
}
"#;
    write_test_file(project_dir, "main.c", c_source);

    // @step And the target file `jv.h` exists in the project
    write_test_file(project_dir, "jv.h", "void jv_parse(const char *input);\n");

    let known_files = build_known_files(project_dir);

    // @step When the C extractor processes the source file
    let entities =
        extract_c(c_source, "main.c", &known_files).expect("C extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to `jv.h`
    let local_imports = find_edges(&entities, "Imports", Some("main-c"), Some("jv-h"));
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge to jv.h. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And system includes like `#include <stdio.h>` should NOT produce edges
    let system_imports = find_edges(&entities, "Imports", None, Some("stdio"));
    assert!(
        system_imports.is_empty(),
        "System includes should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from C function calls
// ============================================================================
#[test]
fn test_c_extract_calls_from_function_calls() {
    let c_source = r#"
void jv_parse(const char *input) {
    return;
}

int main() {
    jv_parse("test");
    return 0;
}
"#;
    let known_files = HashSet::new();

    // @step Given a C file with function `main()` that calls `jv_parse()`
    // @step And `jv_parse` is defined in the same file
    // @step When the C extractor processes the source file
    let entities =
        extract_c(c_source, "main.c", &known_files).expect("C extraction should succeed");

    // @step Then a Calls edge should be emitted from `main` to `jv_parse`
    let calls = find_edges(&entities, "Calls", Some("main"), Some("jv_parse"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from main to jv_parse. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract Imports edges from C++ include directives
// ============================================================================
#[test]
fn test_cpp_extract_imports_from_include_directives() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a C++ file with `#include "utils.h"`
    let cpp_source = r#"
#include "utils.h"
#include <iostream>

int main() {
    helper();
    return 0;
}
"#;
    write_test_file(project_dir, "main.cpp", cpp_source);

    // @step And the target file `utils.h` exists in the project
    write_test_file(project_dir, "utils.h", "void helper();\n");

    let known_files = build_known_files(project_dir);

    // @step When the C++ extractor processes the source file
    let entities =
        extract_cpp(cpp_source, "main.cpp", &known_files).expect("C++ extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to `utils.h`
    let local_imports = find_edges(&entities, "Imports", Some("main-cpp"), Some("utils-h"));
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge to utils.h. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );
}
