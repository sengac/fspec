// Feature: spec/features/source-code-metadata-storage.feature
use super::*;

// ============================================================================
// Scenario: Decorator extraction uses language-specific patterns
// ============================================================================
#[test]
fn test_decorator_extraction_language_specific() {
    // @step Given a project has functions and types with decorators or annotations

    // @step When the project is indexed with ast_index

    // @step Then Python @decorator syntax is captured
    let py_text = "@staticmethod\n@override\ndef method(): pass\n";
    let py_props = node_props("decorated.py", py_text, "Function", "method");
    let py_decs = prop(&py_props, "decorators");
    assert!(
        py_decs.contains("@staticmethod"),
        "Python @decorator should be captured, got: {py_decs}"
    );
    assert!(
        py_decs.contains("@override"),
        "Multiple Python decorators should be captured"
    );

    // @step And Rust #[attribute] syntax is captured
    let rust_text =
        "#[derive(Debug, Clone)]\n#[serde(rename_all = \"camelCase\")]\npub fn something() {}\n";
    let rust_props = node_props("src/lib.rs", rust_text, "Function", "something");
    let rust_decs = prop(&rust_props, "decorators");
    assert!(
        rust_decs.contains("#[derive(Debug, Clone)]"),
        "Rust #[attr] should be captured, got: {rust_decs}"
    );

    // @step And Java @Annotation syntax is captured
    let java_text = "class Service {\n  @Override\n  @SuppressWarnings(\"unchecked\")\n  public void method() {}\n}\n";
    let java_props = node_props("Service.java", java_text, "Function", "method");
    let java_decs = prop(&java_props, "decorators");
    assert!(
        java_decs.contains("@Override"),
        "Java @Annotation should be captured, got: {java_decs}"
    );
}

// ============================================================================
// Scenario: Function with no metadata has empty strings
// ============================================================================
#[tokio::test]
async fn test_function_with_no_metadata_has_empty_strings() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_metadata_db(temp_dir.path()).await;

    // @step Given a project has a plain function with no decorators or docstring
    // (plainHelper in the fixture has no decorators and no docstring)

    // @step When the project is indexed with ast_index
    let result = db
        .query_with_source(AST_QUERIES, "all_functions", None)
        .await
        .expect("query all_functions");
    let functions = result.as_array().expect("array");
    let plain = functions
        .iter()
        .find(|f| f.get("name").and_then(|v| v.as_str()) == Some("plainHelper"))
        .expect("find plainHelper function");

    // @step Then the function node has empty strings for decorators and docstring
    assert_eq!(
        plain.get("decorators").and_then(|v| v.as_str()),
        Some(""),
        "decorators should be empty string"
    );
    assert_eq!(
        plain.get("docstring").and_then(|v| v.as_str()),
        Some(""),
        "docstring should be empty string"
    );

    // @step And the function node still has parameters and source populated
    assert_eq!(
        plain.get("parameters").and_then(|v| v.as_str()),
        Some(""),
        "parameters should be empty string for zero-param function"
    );
    let source = plain
        .get("source")
        .and_then(|v| v.as_str())
        .expect("source present");
    assert!(
        source.contains("function plainHelper"),
        "source should still be populated"
    );
}
