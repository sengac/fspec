// Feature: spec/features/source-code-metadata-storage.feature
//
// Source Code and Metadata Storage in Graph Nodes
// Tests that Function and Type nodes store source code, docstrings,
// parameters, decorators, and language as graph properties after indexing.
//
// Integration tests populate an AST graph with nodes that have the new
// metadata fields, then verify ast_search returns them.
// Unit tests verify the metadata extraction helpers (extract_source,
// extract_docstring, extract_decorators, extract_parameters).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::metadata;
use codelet_napi::graph::database::GraphDatabase;

/// The AST code schema (must include new metadata fields).
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Helper: create a graph with Functions and Types that have metadata fields.
async fn setup_metadata_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-metadata.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-handler-ts","path":"src/handler.ts","language":"typescript","lineCount":200,"isTest":false}}
{"type":"File","data":{"slug":"src-service-py","path":"src/service.py","language":"python","lineCount":100,"isTest":false}}
{"type":"Function","data":{"slug":"src-handler-ts::dispatch","name":"dispatch","qualifiedName":"src-handler-ts::dispatch","isAsync":true,"isPublic":true,"paramCount":2,"lineStart":10,"lineEnd":35,"cyclomaticComplexity":5,"parameters":"action, options","source":"async function dispatch(action: string, options: Options) {\n  if (action === 'create') {\n    return create(options);\n  }\n  return update(options);\n}","docstring":"/** Dispatch an action to the appropriate handler. */","decorators":"@Injectable","language":"typescript"}}
{"type":"Function","data":{"slug":"src-service-py::process","name":"process","qualifiedName":"src-service-py::process","isAsync":false,"isPublic":true,"paramCount":2,"lineStart":5,"lineEnd":20,"cyclomaticComplexity":3,"parameters":"name, age","source":"def process(name: str, age: int):\n    if age < 0:\n        raise ValueError('negative age')\n    return {'name': name, 'age': age}","docstring":"Process a user record.","decorators":"@staticmethod","language":"python"}}
{"type":"Function","data":{"slug":"src-handler-ts::plainHelper","name":"plainHelper","qualifiedName":"src-handler-ts::plainHelper","isAsync":false,"isPublic":false,"paramCount":0,"lineStart":40,"lineEnd":42,"cyclomaticComplexity":1,"parameters":"","source":"function plainHelper() {\n  return 42;\n}","docstring":"","decorators":"","language":"typescript"}}
{"type":"Type","data":{"slug":"src-service-py::UserService","name":"UserService","typeKind":"class","isPublic":true,"lineStart":25,"lineEnd":89,"source":"class UserService:\n    \"\"\"Service for managing user operations.\"\"\"\n    pass","docstring":"Service for managing user operations.","decorators":"@dataclass","language":"python"}}
{"type":"Type","data":{"slug":"src-handler-ts::Options","name":"Options","typeKind":"interface","isPublic":true,"lineStart":1,"lineEnd":5,"source":"interface Options {\n  verbose: boolean;\n  timeout: number;\n}","docstring":"/** Configuration options. */","decorators":"","language":"typescript"}}
{"edge":"Contains","from":"src-handler-ts","to":"src-handler-ts::dispatch","data":{}}
{"edge":"Contains","from":"src-handler-ts","to":"src-handler-ts::plainHelper","data":{}}
{"edge":"Contains","from":"src-service-py","to":"src-service-py::process","data":{}}
{"edge":"ContainsType","from":"src-service-py","to":"src-service-py::UserService","data":{}}
{"edge":"ContainsType","from":"src-handler-ts","to":"src-handler-ts::Options","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

/// Bundled AST query source for named queries.
const AST_QUERIES: &str = include_str!("../../graph/schemas/ast-queries.gq");

// ============================================================================
// Scenario: Function nodes include metadata after indexing
// ============================================================================
#[tokio::test]
async fn test_function_nodes_include_metadata_after_indexing() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_metadata_db(temp_dir.path()).await;

    // @step Given a TypeScript project has been indexed with ast_index
    // (DB populated with TS functions in setup)

    // @step When I search for a function using ast_search
    let result = db
        .query_with_source(AST_QUERIES, "all_functions", None)
        .await
        .expect("query all_functions");
    let functions = result.as_array().expect("array");
    let dispatch = functions
        .iter()
        .find(|f| f.get("name").and_then(|v| v.as_str()) == Some("dispatch"))
        .expect("find dispatch function");

    // @step Then the result includes parameters as comma-separated names
    assert_eq!(
        dispatch.get("parameters").and_then(|v| v.as_str()),
        Some("action, options"),
        "parameters should be comma-separated names"
    );

    // @step And the result includes the function source code
    let source = dispatch
        .get("source")
        .and_then(|v| v.as_str())
        .expect("source present");
    assert!(
        source.contains("async function dispatch"),
        "source should contain the function signature"
    );

    // @step And the result includes the extracted JSDoc docstring
    assert_eq!(
        dispatch.get("docstring").and_then(|v| v.as_str()),
        Some("/** Dispatch an action to the appropriate handler. */"),
        "docstring should be the JSDoc comment"
    );

    // @step And the result includes decorators as comma-separated list
    assert_eq!(
        dispatch.get("decorators").and_then(|v| v.as_str()),
        Some("@Injectable"),
        "decorators should be comma-separated"
    );

    // @step And the result includes the language identifier "typescript"
    assert_eq!(
        dispatch.get("language").and_then(|v| v.as_str()),
        Some("typescript"),
        "language should be typescript"
    );
}

// ============================================================================
// Scenario: Type nodes include line numbers and metadata after indexing
// ============================================================================
#[tokio::test]
async fn test_type_nodes_include_line_numbers_and_metadata() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_metadata_db(temp_dir.path()).await;

    // @step Given a Python project has been indexed with ast_index
    // (DB populated with Python types in setup)

    // @step When I search for a type using ast_search with entity_type "Type"
    let result = db
        .query_with_source(AST_QUERIES, "all_types", None)
        .await
        .expect("query all_types");
    let types = result.as_array().expect("array");
    let user_service = types
        .iter()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some("UserService"))
        .expect("find UserService type");

    // @step Then the result includes lineStart and lineEnd properties
    assert_eq!(
        user_service.get("lineStart").and_then(|v| v.as_i64()),
        Some(25),
        "lineStart should be 25"
    );
    assert_eq!(
        user_service.get("lineEnd").and_then(|v| v.as_i64()),
        Some(89),
        "lineEnd should be 89"
    );

    // @step And the result includes the extracted docstring
    assert_eq!(
        user_service.get("docstring").and_then(|v| v.as_str()),
        Some("Service for managing user operations."),
        "docstring should be extracted"
    );

    // @step And the result includes decorators as comma-separated list
    assert_eq!(
        user_service.get("decorators").and_then(|v| v.as_str()),
        Some("@dataclass"),
        "decorators should be captured"
    );

    // @step And the result includes the language identifier "python"
    assert_eq!(
        user_service.get("language").and_then(|v| v.as_str()),
        Some("python"),
        "language should be python"
    );
}

// ============================================================================
// Scenario: Source code is capped at 100 lines
// ============================================================================
#[test]
fn test_source_code_capped_at_100_lines() {
    // @step Given a project contains a function with more than 100 lines
    let long_source: String = (0..200)
        .map(|i| format!("    let x{i} = {i};\n"))
        .collect::<String>();
    let full_source = format!("function longFn() {{\n{long_source}}}");

    // @step When the project is indexed with ast_index
    let (capped, truncated) = metadata::extract_source(&full_source);

    // @step Then the function node source is truncated to at most 100 lines
    let line_count = capped.lines().count();
    assert!(
        line_count <= 100,
        "source should be capped at 100 lines, got {line_count}"
    );

    // @step And the function node has truncated set to true
    assert!(truncated, "truncated should be true for long source");
}

// ============================================================================
// Scenario: Short function source is stored in full
// ============================================================================
#[test]
fn test_short_function_source_stored_in_full() {
    // @step Given a project contains a function with fewer than 100 lines
    let short_source = "function short() {\n  return 42;\n}";

    // @step When the project is indexed with ast_index
    let (result, truncated) = metadata::extract_source(short_source);

    // @step Then the function node source contains the complete function body
    assert_eq!(result, short_source, "source should be complete");

    // @step And the function node has truncated set to false
    assert!(!truncated, "truncated should be false for short source");
}

// ============================================================================
// Scenario: Docstring extraction uses language-specific patterns
// ============================================================================
#[test]
fn test_docstring_extraction_language_specific() {
    // @step Given a project has functions with language-specific doc comments

    // @step When the project is indexed with ast_index
    // (extraction happens during index)

    // @step Then JSDoc comments are extracted for TypeScript functions
    let ts_text = "/** Dispatches an action. */\nfunction dispatch() {}";
    let ts_doc = metadata::extract_docstring(ts_text, "typescript");
    assert!(
        ts_doc.contains("Dispatches an action"),
        "JSDoc should be extracted for TS, got: {ts_doc}"
    );

    // @step And rustdoc comments are extracted for Rust functions
    let rust_text = "/// Finds all reachable nodes.\n/// Uses BFS traversal.\nfn find_all() {}";
    let rust_doc = metadata::extract_docstring(rust_text, "rust");
    assert!(
        rust_doc.contains("Finds all reachable nodes"),
        "rustdoc should be extracted for Rust, got: {rust_doc}"
    );

    // @step And triple-quoted docstrings are extracted for Python functions
    let py_text = "def process():\n    \"\"\"Process a user record.\"\"\"\n    pass";
    let py_doc = metadata::extract_docstring(py_text, "python");
    assert!(
        py_doc.contains("Process a user record"),
        "triple-quoted docstring should be extracted for Python, got: {py_doc}"
    );
}

// ============================================================================
// Scenario: Parameter names extracted without types
// ============================================================================
#[test]
fn test_parameter_names_extracted_without_types() {
    // @step Given a project has functions with typed parameters

    // @step When the project is indexed with ast_index

    // @step Then parameter names are stored as comma-separated string without types
    let ts_sig = "function dispatch(action: string, options: Options) {}";
    let ts_params = metadata::extract_parameters(ts_sig, "typescript");
    assert_eq!(ts_params, "action, options", "TS params should strip types");

    let rust_sig = "fn process(&self, name: String, age: i32) {}";
    let rust_params = metadata::extract_parameters(rust_sig, "rust");
    assert_eq!(
        rust_params, "name, age",
        "Rust params should strip self and types"
    );

    // @step And language-specific self parameters are filtered appropriately
    let py_sig = "def method(self, name: str, age: int):";
    let py_params = metadata::extract_parameters(py_sig, "python");
    assert_eq!(
        py_params, "name, age",
        "Python params should filter self and strip types"
    );

    let go_sig = "func (s *Server) Handle(ctx Context, req Request) {}";
    let go_params = metadata::extract_parameters(go_sig, "go");
    assert_eq!(
        go_params, "ctx, req",
        "Go params should filter receiver and strip types"
    );
}

// ============================================================================
// Scenario: Decorator extraction uses language-specific patterns
// ============================================================================
#[test]
fn test_decorator_extraction_language_specific() {
    // @step Given a project has functions and types with decorators or annotations

    // @step When the project is indexed with ast_index

    // @step Then Python @decorator syntax is captured
    let py_text = "@staticmethod\n@override\ndef method(): pass";
    let py_decs = metadata::extract_decorators(py_text, "python");
    assert!(
        py_decs.contains("@staticmethod"),
        "Python @decorator should be captured, got: {py_decs}"
    );
    assert!(
        py_decs.contains("@override"),
        "Multiple Python decorators should be captured"
    );

    // @step And Rust #[attribute] syntax is captured
    let rust_text = "#[derive(Debug, Clone)]\n#[serde(rename_all = \"camelCase\")]\npub fn something() {}";
    let rust_decs = metadata::extract_decorators(rust_text, "rust");
    assert!(
        rust_decs.contains("#[derive(Debug, Clone)]"),
        "Rust #[attr] should be captured, got: {rust_decs}"
    );

    // @step And Java @Annotation syntax is captured
    let java_text = "@Override\n@SuppressWarnings(\"unchecked\")\npublic void method() {}";
    let java_decs = metadata::extract_decorators(java_text, "java");
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
    // (plainHelper in setup has empty decorators and docstring)

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
