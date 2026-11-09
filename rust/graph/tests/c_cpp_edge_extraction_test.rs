// Feature: spec/features/calls-imports-edges-c-cpp.feature
//
// Integration tests for C and C++ Imports and Calls edge extraction.
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
// Scenario: Extract Imports edges from C include directives
// ============================================================================
#[test]
fn test_c_extract_imports_from_include_directives() -> TestResult {
    // @step Given a C file with `#include "jv.h"`
    let c_source = r#"
#include "jv.h"
#include <stdio.h>

int main() {
    jv_parse("test");
    return 0;
}
"#;

    // @step And the target file `jv.h` exists in the project
    let header = "void jv_parse(const char *input);\n";

    // @step When the C extractor processes the source file
    let (_directory, indexed) = index_project(&[("main.c", c_source), ("jv.h", header)])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to `jv.h`
    assert_eq!(
        imports_of(all, "main.c"),
        ["jv.h"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And system includes like `#include <stdio.h>` should NOT produce edges
    assert_eq!(
        edges(all, "Imports").len(),
        1,
        "Imports: {:?}",
        edges(all, "Imports")
    );
    assert_eq!(
        unresolved_imports(&indexed, Reason::External),
        1,
        "{:?}",
        indexed.unresolved
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edges from C function calls
// ============================================================================
#[test]
fn test_c_extract_calls_from_function_calls() -> TestResult {
    let c_source = r#"
void jv_parse(const char *input) {
    return;
}

int main() {
    jv_parse("test");
    return 0;
}
"#;

    // @step Given a C file with function `main()` that calls `jv_parse()`
    // @step And `jv_parse` is defined in the same file
    // @step When the C extractor processes the source file
    let (_directory, indexed) = index_project(&[("main.c", c_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `main` to `jv_parse`
    let main = slug_of(all, "Function", "main.c::main")?;
    assert_eq!(
        callees_of(all, &main),
        ["main.c::jv_parse"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Imports edges from C++ include directives
// ============================================================================
#[test]
fn test_cpp_extract_imports_from_include_directives() -> TestResult {
    // @step Given a C++ file with `#include "utils.h"`
    let cpp_source = r#"
#include "utils.h"
#include <iostream>

int main() {
    helper();
    return 0;
}
"#;

    // @step And the target file `utils.h` exists in the project
    let header = "void helper();\n";

    // @step When the C++ extractor processes the source file
    let (_directory, indexed) = index_project(&[("main.cpp", cpp_source), ("utils.h", header)])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to `utils.h`
    assert_eq!(
        imports_of(all, "main.cpp"),
        ["utils.h"],
        "Imports: {:?}",
        edges(all, "Imports")
    );
    Ok(())
}
