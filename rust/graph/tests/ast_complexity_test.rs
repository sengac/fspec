// Feature: spec/features/cyclomatic-complexity-analysis.feature
//
// Cyclomatic Complexity Analysis
// Tests for ast_complexity action that queries function complexity scores
// from the AST graph, and for the complexity calculation module that
// computes cyclomatic complexity during extraction.
//
// Integration tests index a temp TypeScript project whose functions have
// known decision counts, then exercise the dispatch function on its graph.
// Unit tests run the fact engine on one source text and read the
// definition's complexity: 1 + the decision nodes the language table
// declares inside the function.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use codelet_graph::ast_complexity;
use codelet_graph::ast_pipeline::facts::{extract::extract_source, languages, Entity};
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{entities, index, project, slug_of};

/// `n` if statements on `x`: `n` decision nodes.
fn ifs(n: usize) -> String {
    (0..n)
        .map(|i| format!("    if (x === {i}) {{\n        return {i};\n    }}\n"))
        .collect()
}

/// Helper: index a project with functions of varying complexity.
///
/// Functions:
///   - `src/handler.ts::complex_handler` — complexity 12 (highest)
///   - `src/handler.ts::process_data`    — complexity 8
///   - `src/handler.ts::validate_input`  — complexity 6
///   - `src/utils.ts::simple_getter`     — complexity 1 (lowest)
///   - `src/utils.ts::medium_logic`      — complexity 4
async fn setup_complexity_project() -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let handler = format!(
        "export async function complex_handler(x: number, y: number, z: number) {{\n{}    return -1;\n}}\n\n\
         export function process_data(x: number, y: number) {{\n{}    return -1;\n}}\n\n\
         export function validate_input(x: number) {{\n{}    return -1;\n}}\n",
        ifs(11),
        ifs(7),
        ifs(5)
    );
    let utils = format!(
        "export function simple_getter() {{\n    return 42;\n}}\n\n\
         function medium_logic(x: number, y: number) {{\n{}    return -1;\n}}\n",
        ifs(3)
    );
    let (directory, root) = project(&[("src/handler.ts", &handler), ("src/utils.ts", &utils)])?;
    index(&root).await?;
    Ok((directory, root))
}

/// The complexity the fact engine gives the function `name` of `source`,
/// a file at `rel`.
fn complexity_of(rel: &str, source: &str, name: &str) -> u32 {
    let spec = languages::for_path(rel).expect("declared language");
    let facts = extract_source(spec, rel, source);
    let found: Vec<u32> = facts
        .definitions
        .iter()
        .filter(|d| d.entity == Entity::Function && d.name == name)
        .map(|d| d.complexity)
        .collect();
    assert_eq!(found.len(), 1, "one function {name} in {rel}: {found:?}");
    found[0]
}

#[path = "ast_complexity_test/find_top_n_most.rs"]
mod find_top_n_most;
#[path = "ast_complexity_test/nonexistent_function_returns_error.rs"]
mod nonexistent_function_returns_error;

// ============================================================================
// Unit tests: Complexity calculator across languages
// ============================================================================
