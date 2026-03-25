// Feature: spec/features/calls-imports-typeref-edges-java.feature
//
// Integration tests for Java Imports, Calls, and TypeRef edge extraction.
// Uses real Java source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_java_extractor::extract_java;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Java import statements
// ============================================================================
#[test]
fn test_java_extract_imports_from_import_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Java file with `import com.myapp.service.UserService;`
    let java_source = r#"
import com.myapp.service.UserService;
import java.util.List;

public class Controller {
    public void handle() {
        UserService svc = new UserService();
    }
}
"#;
    write_test_file(project_dir, "com/myapp/web/Controller.java", java_source);

    // @step And the target file `com/myapp/service/UserService.java` exists in the project
    let target_source = r#"
package com.myapp.service;

public class UserService {
    public void process() {}
}
"#;
    write_test_file(project_dir, "com/myapp/service/UserService.java", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the Java extractor processes the source file
    let entities = extract_java(java_source, "com/myapp/web/Controller.java", &known_files)
        .expect("Java extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the target file
    let imports = find_edges(&entities, "Imports", Some("Controller"), Some("UserService"));
    assert!(
        !imports.is_empty(),
        "Should have Imports edge to UserService.java. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `import java.util.List` imports should NOT produce edges
    let stdlib_imports = find_edges(&entities, "Imports", None, Some("java-util"));
    assert!(
        stdlib_imports.is_empty(),
        "Standard library imports should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Java method calls
// ============================================================================
#[test]
fn test_java_extract_calls_from_method_calls() {
    let java_source = r#"
public class Service {
    public void processRequest() {
        validate();
    }

    public void validate() {
        return;
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a Java file with method `processRequest()` that calls `validate()`
    // @step And `validate` is defined in the same file
    // @step When the Java extractor processes the source file
    let entities = extract_java(java_source, "Service.java", &known_files)
        .expect("Java extraction should succeed");

    // @step Then a Calls edge should be emitted from `processRequest` to `validate`
    let calls = find_edges(&entities, "Calls", Some("processRequest"), Some("validate"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from processRequest to validate. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from Java type annotations
// ============================================================================
#[test]
fn test_java_extract_typerefs_from_type_annotations() {
    let java_source = r#"
public class Request {
    public String url;
}

public class Response {
    public int status;
}

public class Handler {
    public Response handle(Request req) {
        return new Response();
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a Java file with `public Response handle(Request req)`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the Java extractor processes the source file
    let entities = extract_java(java_source, "Handler.java", &known_files)
        .expect("Java extraction should succeed");

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
