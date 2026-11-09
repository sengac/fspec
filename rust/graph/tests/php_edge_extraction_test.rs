// Feature: spec/features/calls-imports-typeref-edges-php.feature
//
// Integration tests for PHP Imports, Calls, and TypeRef edge extraction.
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
// Scenario: Extract Imports edges from PHP use statements
// ============================================================================
#[test]
fn test_php_extract_imports_from_use_statements() -> TestResult {
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

    // @step And the target file `Slim/Routing/RouteResolver.php` exists in the project
    // `use Slim\Routing\RouteResolver;` names the class RouteResolver that
    // a file of the tree declares in namespace Slim\Routing.
    let target_source = r#"<?php
namespace Slim\Routing;

class RouteResolver {
    public function resolve() {
        return true;
    }
}
"#;

    // @step When the PHP extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("Slim/App.php", php_source),
        ("Slim/Routing/RouteResolver.php", target_source),
    ])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted from the source file to the target file
    assert_eq!(
        imports_of(all, "Slim/App.php"),
        ["Slim/Routing/RouteResolver.php"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And external `use Psr\Http\Message\*` imports should NOT produce edges
    assert_eq!(
        edges(all, "Imports").len(),
        1,
        "External Psr\\Http\\Message imports should NOT produce edges: {:?}",
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
// Scenario: Extract Calls edges from PHP same-file method calls
// ============================================================================
#[test]
fn test_php_extract_calls_from_same_file_methods() -> TestResult {
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

    // @step Given a PHP file with methods `addRoutingMiddleware()` and `getRouteResolver()`
    // @step And `addRoutingMiddleware()` contains `$this->getRouteResolver()`
    // @step When the PHP extractor processes the source file
    let (_directory, indexed) = index_project(&[("Slim/App.php", php_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `addRoutingMiddleware` to `getRouteResolver`
    let caller = slug_of(all, "Function", "Slim/App.php::App::addRoutingMiddleware")?;
    assert_eq!(
        callees_of(all, &caller),
        ["Slim/App.php::App::getRouteResolver"],
        "Calls: {:?}, unresolved: {:?}",
        edges(all, "Calls"),
        indexed.unresolved
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from PHP type-annotated signatures
// ============================================================================
#[test]
fn test_php_extract_typerefs_from_type_annotations() -> TestResult {
    // Files declaring the same namespace see each other's names.
    let handler_source = r#"<?php
namespace App;

class Handler {
    public function handle(AppRequest $request): AppResponse {
        return new AppResponse();
    }

    public function stamp(string $format): \DateTimeImmutable {
        return new \DateTimeImmutable($format);
    }
}
"#;
    let request_source = "<?php\nnamespace App;\n\nclass AppRequest {\n    public $url;\n}\n";
    let response_source = "<?php\nnamespace App;\n\nclass AppResponse {\n    public $status;\n}\n";

    // @step Given a PHP file with `public function handle(AppRequest $request): AppResponse`
    // @step And types `AppRequest` and `AppResponse` are defined in local project files
    // @step When the PHP extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("src/Handler.php", handler_source),
        ("src/AppRequest.php", request_source),
        ("src/AppResponse.php", response_source),
    ])?;
    let all = &indexed.entities;

    // @step Then TypeRef edges should be emitted from `handle` to `AppRequest` and `AppResponse`
    let handle = slug_of(all, "Function", "src/Handler.php::Handler::handle")?;
    assert_eq!(
        type_refs_of(all, &handle),
        [
            "src/AppRequest.php::AppRequest",
            "src/AppResponse.php::AppResponse"
        ],
        "TypeRef: {:?}",
        edges(all, "TypeRef")
    );

    // @step And external types not in the project should NOT produce TypeRef edges
    // Built-in types like string, int, bool should not appear
    let stamp = slug_of(all, "Function", "src/Handler.php::Handler::stamp")?;
    assert!(
        type_refs_of(all, &stamp).is_empty(),
        "Built-in and external types should NOT produce TypeRef edges: {:?}",
        edges(all, "TypeRef")
    );
    assert_eq!(
        edges(all, "TypeRef").len(),
        2,
        "TypeRef: {:?}",
        edges(all, "TypeRef")
    );
    Ok(())
}
