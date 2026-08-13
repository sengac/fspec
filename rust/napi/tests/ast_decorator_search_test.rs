// Feature: spec/features/decorator-annotation-search.feature
//
// Decorator and Annotation Search
// Tests that decorators/annotations are extracted for all supported languages
// during ast_index, including the Scala (AtSign) and PHP (HashBracket) gaps
// fixed in this work unit.
//
// Integration tests verify decorator extraction via the shared metadata module.
// Cross-language decorator filter tests are covered by ast_search_filter_test.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::metadata;

// ============================================================================
// Scenario: Python decorators extracted during indexing
// ============================================================================
#[test]
fn test_python_decorators_extracted() {
    // @step Given a Python file with functions decorated with @staticmethod and @override
    let text = "@staticmethod\n@override\ndef process(name, age):\n    pass";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "python");

    // @step Then the function node's decorators property contains "@staticmethod, @override"
    assert!(
        meta.decorators.contains("@staticmethod"),
        "Should extract @staticmethod — got: {}",
        meta.decorators
    );
    assert!(
        meta.decorators.contains("@override"),
        "Should extract @override — got: {}",
        meta.decorators
    );
}

// ============================================================================
// Scenario: Scala annotations extracted as AtSign style
// ============================================================================
#[test]
fn test_scala_annotations_extracted_as_atsign() {
    // @step Given a Scala file with a function annotated with @tailrec
    let text = "@tailrec\ndef factorial(n: Int, acc: Int = 1): Int = {\n  if (n <= 1) acc else factorial(n - 1, n * acc)\n}";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "scala");

    // @step Then the function node's decorators property contains "@tailrec"
    assert!(
        meta.decorators.contains("@tailrec"),
        "Scala @tailrec should be extracted as AtSign style — got: '{}'",
        meta.decorators
    );
}

// ============================================================================
// Scenario: PHP 8 attributes extracted as HashBracket style
// ============================================================================
#[test]
fn test_php_attributes_extracted_as_hashbracket() {
    // @step Given a PHP file with a function attributed with #[Route('/api')]
    let text = "#[Route('/api')]\npublic function index(): Response {\n  return new Response();\n}";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "php");

    // @step Then the function node's decorators property contains "#[Route('/api')]"
    assert!(
        meta.decorators.contains("#[Route('/api')]"),
        "PHP #[Route('/api')] should be extracted as HashBracket style — got: '{}'",
        meta.decorators
    );
}

// ============================================================================
// Scenario: Rust attributes extracted as HashBracket style
// ============================================================================
#[test]
fn test_rust_attributes_extracted() {
    // @step Given a Rust file with a function attributed with #[test]
    let text = "#[test]\nfn test_something() {\n    assert!(true);\n}";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "rust");

    // @step Then the function node's decorators property contains "#[test]"
    assert!(
        meta.decorators.contains("#[test]"),
        "Rust #[test] should be extracted — got: '{}'",
        meta.decorators
    );
}

// ============================================================================
// Scenario: C# attributes extracted as SquareBracket style
// ============================================================================
#[test]
fn test_csharp_attributes_extracted() {
    // @step Given a C# file with a function attributed with [HttpGet]
    let text = "[HttpGet]\npublic IActionResult Index() {\n  return View();\n}";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "csharp");

    // @step Then the function node's decorators property contains "[HttpGet]"
    assert!(
        meta.decorators.contains("[HttpGet]"),
        "C# [HttpGet] should be extracted — got: '{}'",
        meta.decorators
    );
}

// ============================================================================
// Scenario: Languages without decorators produce empty string
// ============================================================================
#[test]
fn test_no_decorator_languages_produce_empty() {
    // @step Given a Go file with functions that have no decorator syntax
    let text = "func handleRequest(w http.ResponseWriter, r *http.Request) {\n    w.Write([]byte(\"ok\"))\n}";

    // @step When the file is indexed via ast_index
    let meta = metadata::extract_function_meta(text, "go");

    // @step Then the function node's decorators property is an empty string
    assert!(
        meta.decorators.is_empty(),
        "Go should produce empty decorators — got: '{}'",
        meta.decorators
    );

    // Also check C and C++
    let c_meta = metadata::extract_function_meta("void foo() {}", "c");
    assert!(
        c_meta.decorators.is_empty(),
        "C should produce empty decorators"
    );

    let cpp_meta = metadata::extract_function_meta("void bar() {}", "cpp");
    assert!(
        cpp_meta.decorators.is_empty(),
        "C++ should produce empty decorators"
    );

    let ruby_meta = metadata::extract_function_meta("def greet\n  puts 'hi'\nend", "ruby");
    assert!(
        ruby_meta.decorators.is_empty(),
        "Ruby should produce empty decorators"
    );
}
