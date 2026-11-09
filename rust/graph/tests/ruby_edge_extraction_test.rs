// Feature: spec/features/calls-imports-edges-ruby.feature
//
// Integration tests for Ruby Imports and Calls edge extraction.
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
// Scenario: Extract Imports edges from Ruby require_relative statements
// ============================================================================
#[test]
fn test_ruby_extract_imports_from_require_relative() -> TestResult {
    // @step Given a Ruby file with `require_relative 'helpers'`
    let ruby_source = r#"
require_relative 'helpers'
require 'json'

def process(data)
  validate(data)
end
"#;

    // @step And the target file `helpers.rb` exists in the project
    let target_source = r#"
def validate(data)
  data.is_a?(Hash)
end
"#;

    // @step When the Ruby extractor processes the source file
    let (_directory, indexed) =
        index_project(&[("app.rb", ruby_source), ("helpers.rb", target_source)])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to `helpers.rb`
    assert_eq!(
        imports_of(all, "app.rb"),
        ["helpers.rb"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And external `require 'json'` imports should NOT produce edges
    assert_eq!(
        edges(all, "Imports").len(),
        1,
        "External 'json' require should NOT produce edges: {:?}",
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
// Scenario: Extract Calls edges from Ruby method calls
// ============================================================================
#[test]
fn test_ruby_extract_calls_from_method_calls() -> TestResult {
    let ruby_source = r#"
def process(data)
  validate(data)
end

def validate(data)
  data.is_a?(Hash)
end
"#;

    // @step Given a Ruby file with method `process` that calls `validate`
    // @step And `validate` is defined in the same file
    // @step When the Ruby extractor processes the source file
    let (_directory, indexed) = index_project(&[("app.rb", ruby_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `process` to `validate`
    let process = slug_of(all, "Function", "app.rb::process")?;
    assert_eq!(
        callees_of(all, &process),
        ["app.rb::validate"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}
