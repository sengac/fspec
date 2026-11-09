// Feature: spec/features/decorator-annotation-search.feature
//
// Decorator and Annotation Search
// Tests that decorators/annotations are extracted for all supported languages
// during ast_index, including the Scala (AtSign) and PHP (HashBracket) gaps
// fixed in this work unit.
//
// Integration tests index one real source file per language with the fact
// pipeline and read the Function node's decorators property.
// Cross-language decorator filter tests are covered by ast_search_filter_test.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::graph_entities::GraphEntity;

pub mod graph_test_helpers;
use graph_test_helpers::index_fixture;

/// Index a project holding `rel` with `source` and return the decorators
/// property of its one Function node named `name`.
fn decorators_of(rel: &str, source: &str, name: &str) -> String {
    let (_dir, indexed) = index_fixture(&[(rel, source)]);
    let mut found: Vec<String> = indexed
        .entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type,
                properties,
                ..
            } if node_type == "Function"
                && properties.get("name").and_then(|v| v.as_str()) == Some(name) =>
            {
                Some(
                    properties
                        .get("decorators")
                        .and_then(|v| v.as_str())
                        .expect("decorators property")
                        .to_string(),
                )
            }
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "one Function {name} in {rel}: {found:?}");
    found.remove(0)
}

// ============================================================================
// Scenario: Python decorators extracted during indexing
// ============================================================================
#[test]
fn test_python_decorators_extracted() {
    // @step Given a Python file with functions decorated with @staticmethod and @override
    let text = "@staticmethod\n@override\ndef process(name, age):\n    pass\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("handlers.py", text, "process");

    // @step Then the function node's decorators property contains "@staticmethod, @override"
    assert!(
        decorators.contains("@staticmethod"),
        "Should extract @staticmethod — got: {}",
        decorators
    );
    assert!(
        decorators.contains("@override"),
        "Should extract @override — got: {}",
        decorators
    );
}

// ============================================================================
// Scenario: Scala annotations extracted as AtSign style
// ============================================================================
#[test]
fn test_scala_annotations_extracted_as_atsign() {
    // @step Given a Scala file with a function annotated with @tailrec
    let text = "object Math {\n  @tailrec\n  def factorial(n: Int, acc: Int = 1): Int = {\n    if (n <= 1) acc else factorial(n - 1, n * acc)\n  }\n}\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("Math.scala", text, "factorial");

    // @step Then the function node's decorators property contains "@tailrec"
    assert!(
        decorators.contains("@tailrec"),
        "Scala @tailrec should be extracted as AtSign style — got: '{}'",
        decorators
    );
}

// ============================================================================
// Scenario: PHP 8 attributes extracted as HashBracket style
// ============================================================================
#[test]
fn test_php_attributes_extracted_as_hashbracket() {
    // @step Given a PHP file with a function attributed with #[Route('/api')]
    let text = "<?php\nclass ApiController {\n  #[Route('/api')]\n  public function index(): Response {\n    return new Response();\n  }\n}\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("ApiController.php", text, "index");

    // @step Then the function node's decorators property contains "#[Route('/api')]"
    assert!(
        decorators.contains("#[Route('/api')]"),
        "PHP #[Route('/api')] should be extracted as HashBracket style — got: '{}'",
        decorators
    );
}

// ============================================================================
// Scenario: Rust attributes extracted as HashBracket style
// ============================================================================
#[test]
fn test_rust_attributes_extracted() {
    // @step Given a Rust file with a function attributed with #[test]
    let text = "#[test]\nfn test_something() {\n    assert!(true);\n}\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("src/lib.rs", text, "test_something");

    // @step Then the function node's decorators property contains "#[test]"
    assert!(
        decorators.contains("#[test]"),
        "Rust #[test] should be extracted — got: '{}'",
        decorators
    );
}

// ============================================================================
// Scenario: C# attributes extracted as SquareBracket style
// ============================================================================
#[test]
fn test_csharp_attributes_extracted() {
    // @step Given a C# file with a function attributed with [HttpGet]
    let text = "public class HomeController {\n  [HttpGet]\n  public IActionResult Index() {\n    return View();\n  }\n}\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("HomeController.cs", text, "Index");

    // @step Then the function node's decorators property contains "[HttpGet]"
    assert!(
        decorators.contains("[HttpGet]"),
        "C# [HttpGet] should be extracted — got: '{}'",
        decorators
    );
}

// ============================================================================
// Scenario: Languages without decorators produce empty string
// ============================================================================
#[test]
fn test_no_decorator_languages_produce_empty() {
    // @step Given a Go file with functions that have no decorator syntax
    let text = "package main\n\nfunc handleRequest(w http.ResponseWriter, r *http.Request) {\n    w.Write([]byte(\"ok\"))\n}\n";

    // @step When the file is indexed via ast_index
    let decorators = decorators_of("main.go", text, "handleRequest");

    // @step Then the function node's decorators property is an empty string
    assert!(
        decorators.is_empty(),
        "Go should produce empty decorators — got: '{}'",
        decorators
    );

    // Also check C and C++
    let c_decorators = decorators_of("foo.c", "void foo() {}\n", "foo");
    assert!(c_decorators.is_empty(), "C should produce empty decorators");

    let cpp_decorators = decorators_of("bar.cpp", "void bar() {}\n", "bar");
    assert!(
        cpp_decorators.is_empty(),
        "C++ should produce empty decorators"
    );

    let ruby_decorators = decorators_of("greet.rb", "def greet\n  puts 'hi'\nend\n", "greet");
    assert!(
        ruby_decorators.is_empty(),
        "Ruby should produce empty decorators"
    );
}
