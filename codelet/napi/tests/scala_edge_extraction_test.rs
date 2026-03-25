// Feature: spec/features/calls-imports-typeref-edges-scala.feature
//
// Integration tests for Scala Imports, Calls, and TypeRef edge extraction.
// Uses real Scala source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_scala_extractor::extract_scala;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Scala import statements
// ============================================================================
#[test]
fn test_scala_extract_imports_from_import_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Scala file with `import com.myapp.service.UserService`
    let scala_source = r#"
import com.myapp.service.UserService
import scala.collection.mutable.ListBuffer

def handle() = {
    val svc = UserService()
    process()
}

def process() = {
    return
}
"#;
    write_test_file(project_dir, "com/myapp/web/Controller.scala", scala_source);

    // @step And the target file `com/myapp/service/UserService.scala` exists in the project
    let target_source = r#"
package com.myapp.service

class UserService {
    def execute(): Unit = {}
}
"#;
    write_test_file(project_dir, "com/myapp/service/UserService.scala", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Scala extractor processes the source file
    let entities = extract_scala(scala_source, "com/myapp/web/Controller.scala", &known_files)
        .expect("Scala extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the target file
    let imports = find_edges(&entities, "Imports", Some("Controller"), Some("UserService"));
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to UserService.scala. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `import scala.collection.mutable.ListBuffer` should NOT produce an edge
    let all_imports = find_edges(&entities, "Imports", None, None);
    // Only 1 import edge (the local one), no edge for scala.collection
    assert_eq!(
        all_imports.len(),
        1,
        "Should have exactly 1 Imports edge (the local one). All Imports: {:?}",
        all_imports
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Scala function calls
// ============================================================================
#[test]
fn test_scala_extract_calls_from_function_calls() {
    let scala_source = r#"
def handle() = {
    process()
}

def process() = {
    return
}
"#;
    let known_files = HashSet::new();

    // @step Given a Scala file with function `handle()` that calls `process()`
    // @step And `process` is defined in the same file
    // @step When the Scala extractor processes the source file
    let entities = extract_scala(scala_source, "Controller.scala", &known_files)
        .expect("Scala extraction should succeed");

    // @step Then a Calls edge should be emitted from `handle` to `process`
    let calls = find_edges(&entities, "Calls", Some("handle"), Some("process"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from handle to process. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from Scala type annotations
// ============================================================================
#[test]
fn test_scala_extract_typerefs_from_type_annotations() {
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
    let known_files = HashSet::new();

    // @step Given a Scala file with `def handle(req: Request): Response`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the Scala extractor processes the source file
    let entities = extract_scala(scala_source, "Handler.scala", &known_files)
        .expect("Scala extraction should succeed");

    // @step Then TypeRef edges should be emitted from `handle` to `Request` and `Response`
    let request_refs = find_edges(&entities, "TypeRef", Some("handle"), Some("Request"));
    assert!(
        !request_refs.is_empty(),
        "Should have TypeRef from handle to Request. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );

    let response_refs = find_edges(&entities, "TypeRef", Some("handle"), Some("Response"));
    assert!(
        !response_refs.is_empty(),
        "Should have TypeRef from handle to Response. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );
}
