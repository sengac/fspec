// Feature: spec/features/calls-imports-typeref-edges-php.feature
//
// Integration tests for PHP Imports, Calls, and TypeRef edge extraction.
// Uses real PHP source code fixtures and verifies the extracted graph entities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_php_extractor::extract_php;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from PHP use statements
// ============================================================================
#[test]
fn test_php_extract_imports_from_use_statements() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a PHP file with `use Slim\Routing\RouteResolver;` namespace import
    let php_source = r#"<?php
namespace Slim;

use Slim\Routing\RouteResolver;
use Psr\Http\Message\ServerRequestInterface;

class App {
    public function addRoutingMiddleware() {
        $resolver = new RouteResolver();
        return $resolver;
    }
}
"#;
    write_test_file(project_dir, "Slim/App.php", php_source);

    // @step And the target file `Slim/Routing/RouteResolver.php` exists in the project
    let target_source = r#"<?php
namespace Slim\Routing;

class RouteResolver {
    public function resolve() {
        return true;
    }
}
"#;
    write_test_file(project_dir, "Slim/Routing/RouteResolver.php", target_source);

    let known_files = build_known_files(project_dir);

    // @step When the PHP extractor processes the source file
    let entities = extract_php(php_source, "Slim/App.php", &known_files)
        .expect("PHP extraction should succeed");

    // @step Then an Imports edge should be emitted from the source file to the target file
    let local_imports = find_edges(&entities, "Imports", Some("Slim-App-php"), Some("RouteResolver"));
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge to RouteResolver target. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And external `use Psr\Http\Message\*` imports should NOT produce edges
    let psr_imports = find_edges(&entities, "Imports", None, Some("Psr"));
    assert!(
        psr_imports.is_empty(),
        "External Psr\\Http\\Message imports should NOT produce edges, got {:?}",
        psr_imports
    );
}

// ============================================================================
// Scenario: Extract Calls edges from PHP same-file method calls
// ============================================================================
#[test]
fn test_php_extract_calls_from_same_file_methods() {
    let php_source = r#"<?php
class App {
    public function addRoutingMiddleware() {
        $resolver = $this->getRouteResolver();
        return $resolver;
    }

    public function getRouteResolver() {
        return new RouteResolver();
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a PHP file with methods `addRoutingMiddleware()` and `getRouteResolver()`
    // @step And `addRoutingMiddleware()` contains `$this->getRouteResolver()`
    // @step When the PHP extractor processes the source file
    let entities = extract_php(php_source, "Slim/App.php", &known_files)
        .expect("PHP extraction should succeed");

    // @step Then a Calls edge should be emitted from `addRoutingMiddleware` to `getRouteResolver`
    let calls = find_edges(
        &entities,
        "Calls",
        Some("addRoutingMiddleware"),
        Some("getRouteResolver"),
    );
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from addRoutingMiddleware to getRouteResolver. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from PHP type-annotated signatures
// ============================================================================
#[test]
fn test_php_extract_typerefs_from_type_annotations() {
    let php_source = r#"<?php
class AppRequest {
    public $url;
}

class AppResponse {
    public $status;
}

class Handler {
    public function handle(AppRequest $request): AppResponse {
        return new AppResponse();
    }
}
"#;
    let known_files = HashSet::new();

    // @step Given a PHP file with `public function handle(AppRequest $request): AppResponse`
    // @step And types `AppRequest` and `AppResponse` are defined in local project files
    // @step When the PHP extractor processes the source file
    let entities = extract_php(php_source, "src/Handler.php", &known_files)
        .expect("PHP extraction should succeed");

    // @step Then TypeRef edges should be emitted from `handle` to `AppRequest` and `AppResponse`
    let request_refs = find_edges(&entities, "TypeRef", Some("handle"), Some("AppRequest"));
    assert!(
        !request_refs.is_empty(),
        "Should have TypeRef edge from handle to AppRequest. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );

    let response_refs = find_edges(&entities, "TypeRef", Some("handle"), Some("AppResponse"));
    assert!(
        !response_refs.is_empty(),
        "Should have TypeRef edge from handle to AppResponse. All TypeRef: {:?}",
        find_edges(&entities, "TypeRef", None, None)
    );

    // @step And external types not in the project should NOT produce TypeRef edges
    // Built-in types like string, int, bool should not appear
    let builtin_refs = find_edges(&entities, "TypeRef", None, Some("string"));
    assert!(
        builtin_refs.is_empty(),
        "Built-in types should NOT produce TypeRef edges"
    );
}
