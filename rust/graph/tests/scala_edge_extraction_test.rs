// Feature: spec/features/calls-imports-typeref-edges-scala.feature
//
// Integration tests for Scala Imports, Calls, and TypeRef edge extraction.
// Every scenario indexes a temp project with the real pipeline (walk, fact
// extraction, global resolve pass); no edge is hand-written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::resolve::{Reason, RefKindCount};
use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;
use codelet_test_helpers::{TestError, TestResult};

pub mod code_graph_fixture;
use code_graph_fixture::{callees_of, edges, nodes, project, slug_of};

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

/// The qualifiedNames the TypeRef edges from `function` lead to, sorted.
fn type_refs_of(all: &[GraphEntity], function: &str) -> Vec<String> {
    let names: std::collections::BTreeMap<String, String> = nodes(all, "Type")
        .into_iter()
        .filter_map(|(slug, p)| Some((slug, p.get("qualifiedName")?.as_str()?.to_string())))
        .collect();
    let mut found: Vec<String> = edges(all, "TypeRef")
        .into_iter()
        .filter(|(from, _, _)| from == function)
        .filter_map(|(_, to, _)| names.get(&to).cloned())
        .collect();
    found.sort();
    found.dedup();
    found
}
// ============================================================================
// Scenario: Extract Imports edges from Scala import statements
// ============================================================================
#[test]
fn test_scala_extract_imports_from_import_statements() -> TestResult {
    // @step Given a Scala file with `import com.myapp.UserService`
    let scala_source = r#"
package com.myapp.web

import com.myapp.UserService
import scala.collection.mutable

def handle() = {
    val svc = UserService()
    process()
}

def process() = {
    return
}
"#;

    // @step And the target file `com/myapp/UserService.scala` exists in the project
    let target_source = r#"
package com.myapp

class UserService {
    def execute(): Unit = {}
}
"#;

    // @step When the Scala extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("com/myapp/web/Controller.scala", scala_source),
        ("com/myapp/UserService.scala", target_source),
    ])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to the target
    assert_eq!(
        imports_of(all, "com/myapp/web/Controller.scala"),
        ["com/myapp/UserService.scala"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And external `import scala.collection.mutable` imports should NOT produce edges
    // Only 1 import edge (the local one), no edge for scala.collection
    assert_eq!(
        edges(all, "Imports").len(),
        1,
        "Should have exactly 1 Imports edge (the local one). All Imports: {:?}",
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
// Scenario: Extract Calls edges from Scala function calls
// ============================================================================
#[test]
fn test_scala_extract_calls_from_function_calls() -> TestResult {
    let scala_source = r#"
def process() = {
    validate()
}

def validate() = {
    return
}
"#;

    // @step Given a Scala file with function `process()` that calls `validate()`
    // @step And `validate` is defined in the same file
    // @step When the Scala extractor processes the source file
    let (_directory, indexed) = index_project(&[("Controller.scala", scala_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `process` to `validate`
    let process = slug_of(all, "Function", "Controller.scala::process")?;
    assert_eq!(
        callees_of(all, &process),
        ["Controller.scala::validate"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from Scala type annotations
// ============================================================================
#[test]
fn test_scala_extract_typerefs_from_type_annotations() -> TestResult {
    let scala_source = r#"
class Request {
    val url: String = ""
}

class Response {
    val status: Int = 0
}

def handle(req: Request): Response = {
    new Response()
}
"#;

    // @step Given a Scala file with `def handle(req: Request): Response`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the Scala extractor processes the source file
    let (_directory, indexed) = index_project(&[("Handler.scala", scala_source)])?;
    let all = &indexed.entities;

    // @step Then TypeRef edges should be emitted from `handle` to `Request` and `Response`
    let handle = slug_of(all, "Function", "Handler.scala::handle")?;
    assert_eq!(
        type_refs_of(all, &handle),
        ["Handler.scala::Request", "Handler.scala::Response"],
        "TypeRef: {:?}",
        edges(all, "TypeRef")
    );
    Ok(())
}
