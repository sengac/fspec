// Feature: spec/features/calls-imports-typeref-edges-kotlin.feature
//
// Integration tests for Kotlin Imports, Calls, and TypeRef edge extraction.
// Uses real Kotlin source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_kotlin_extractor::extract_kotlin;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Kotlin import statements
// ============================================================================
#[test]
fn test_kotlin_extract_imports_from_import_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Kotlin file with `import com.myapp.service.UserService`
    let kotlin_source = r#"
import com.myapp.service.UserService
import java.util.List

fun handle() {
    val svc = UserService()
    process()
}

fun process() {
    return
}
"#;
    write_test_file(project_dir, "com/myapp/web/Controller.kt", kotlin_source);

    // @step And the target file `com/myapp/service/UserService.kt` exists in the project
    let target_source = r#"
package com.myapp.service

class UserService {
    fun execute() {}
}
"#;
    write_test_file(project_dir, "com/myapp/service/UserService.kt", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Kotlin extractor processes the source file
    let entities = extract_kotlin(kotlin_source, "com/myapp/web/Controller.kt", &known_files)
        .expect("Kotlin extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the target file
    let imports = find_edges(&entities, "Imports", Some("Controller"), Some("UserService"));
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to UserService.kt. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `import java.util.List` should NOT produce an edge
    let java_imports = find_edges(&entities, "Imports", None, Some("java"));
    assert!(
        java_imports.is_empty(),
        "External java.util.List imports should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Kotlin function calls
// ============================================================================
#[test]
fn test_kotlin_extract_calls_from_function_calls() {
    let kotlin_source = r#"
fun handle() {
    process()
}

fun process() {
    return
}
"#;
    let known_files = HashSet::new();

    // @step Given a Kotlin file with function `handle()` that calls `process()`
    // @step And `process` is defined in the same file
    // @step When the Kotlin extractor processes the source file
    let entities = extract_kotlin(kotlin_source, "Controller.kt", &known_files)
        .expect("Kotlin extraction should succeed");

    // @step Then a Calls edge should be emitted from `handle` to `process`
    let calls = find_edges(&entities, "Calls", Some("handle"), Some("process"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from handle to process. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from Kotlin type annotations
// ============================================================================
#[test]
fn test_kotlin_extract_typerefs_from_type_annotations() {
    let kotlin_source = r#"
class Request {
    val url: String = ""
}

class Response {
    val status: Int = 0
}

fun handle(req: Request): Response {
    return Response()
}
"#;
    let known_files = HashSet::new();

    // @step Given a Kotlin file with `fun handle(req: Request): Response`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the Kotlin extractor processes the source file
    let entities = extract_kotlin(kotlin_source, "Handler.kt", &known_files)
        .expect("Kotlin extraction should succeed");

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
