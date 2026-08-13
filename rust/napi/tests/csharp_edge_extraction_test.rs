// Feature: spec/features/calls-imports-typeref-edges-csharp.feature
//
// Integration tests for C# Imports, Calls, and TypeRef edge extraction.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_csharp_extractor::extract_csharp;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from C# using statements
// ============================================================================
#[test]
fn test_csharp_extract_imports_from_using_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a C# file with `using MyApp.Services;`
    let cs_source = r#"
using MyApp.Services;
using System.Collections;

public class Controller {
    public void Handle() {
        var svc = new UserService();
    }
}
"#;
    write_test_file(project_dir, "Controllers/Controller.cs", cs_source);

    // @step And the target file `MyApp/Services.cs` exists in the project
    write_test_file(
        project_dir,
        "MyApp/Services.cs",
        "public class UserService {}\n",
    );

    let known_files = build_known_files(project_dir);

    // @step When the C# extractor processes the source file
    let entities = extract_csharp(cs_source, "Controllers/Controller.cs", &known_files)
        .expect("C# extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the target
    let local_imports = find_edges(&entities, "Imports", Some("Controller"), Some("MyApp"));
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge to MyApp/Services. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And system `using System.Collections` imports should NOT produce edges
    let system_imports = find_edges(&entities, "Imports", None, Some("System"));
    assert!(
        system_imports.is_empty(),
        "System imports should NOT produce edges"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from C# method calls
// ============================================================================
#[test]
fn test_csharp_extract_calls_from_method_calls() {
    let cs_source = r#"
public class Service {
    public void Process() {
        Validate();
    }

    public void Validate() {
        return;
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a C# file with method `Process()` that calls `Validate()`
    // @step And `Validate` is defined in the same file
    // @step When the C# extractor processes the source file
    let entities = extract_csharp(cs_source, "Service.cs", &known_files)
        .expect("C# extraction should succeed");

    // @step Then a Calls edge should be emitted from `Process` to `Validate`
    let calls = find_edges(&entities, "Calls", Some("Process"), Some("Validate"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from Process to Validate. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from C# type annotations
// ============================================================================
#[test]
fn test_csharp_extract_typerefs_from_type_annotations() {
    let cs_source = r#"
public class Request {
    public string Url;
}

public class Response {
    public int Status;
}

public class Handler {
    public Response Handle(Request req) {
        return new Response();
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a C# file with `public Response Handle(Request req)`
    // @step And types `Request` and `Response` are defined in the same file
    // @step When the C# extractor processes the source file
    let entities = extract_csharp(cs_source, "Handler.cs", &known_files)
        .expect("C# extraction should succeed");

    // @step Then TypeRef edges should be emitted from `Handle` to `Request` and `Response`
    let request_refs = find_edges(&entities, "TypeRef", Some("Handle"), Some("Request"));
    assert!(
        !request_refs.is_empty(),
        "Should have TypeRef from Handle to Request. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );

    let response_refs = find_edges(&entities, "TypeRef", Some("Handle"), Some("Response"));
    assert!(
        !response_refs.is_empty(),
        "Should have TypeRef from Handle to Response. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );
}
