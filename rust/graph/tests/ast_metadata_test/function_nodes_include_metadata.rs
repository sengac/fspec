// Feature: spec/features/source-code-metadata-storage.feature
use super::*;

// ============================================================================
// Scenario: Function nodes include metadata after indexing
// ============================================================================
#[tokio::test]
async fn test_function_nodes_include_metadata_after_indexing() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_metadata_db(temp_dir.path()).await;

    // @step Given a TypeScript project has been indexed with ast_index
    // (src/handler.ts of the fixture)

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
        source.contains("async dispatch(action: string, options: Options)"),
        "source should contain the method signature: {source}"
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
    // (src/service.py of the fixture)

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
    // class UserService: is on line 10 of src/service.py, `pass` on 12.
    assert_eq!(
        user_service.get("lineStart").and_then(|v| v.as_i64()),
        Some(10),
        "lineStart should be 10: {user_service}"
    );
    assert_eq!(
        user_service.get("lineEnd").and_then(|v| v.as_i64()),
        Some(12),
        "lineEnd should be 12: {user_service}"
    );

    // @step And the result includes the extracted docstring
    let docstring = user_service
        .get("docstring")
        .and_then(|v| v.as_str())
        .expect("docstring present");
    assert!(
        docstring.contains("Service for managing user operations."),
        "docstring should be extracted: {docstring}"
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
    let props = node_props("src/long.ts", &full_source, "Function", "longFn");
    let capped = prop(&props, "source");
    let truncated = props.get("truncated").and_then(|v| v.as_bool());

    // @step Then the function node source is truncated to at most 100 lines
    let line_count = capped.lines().count();
    assert!(
        line_count <= 100,
        "source should be capped at 100 lines, got {line_count}"
    );

    // @step And the function node has truncated set to true
    assert_eq!(
        truncated,
        Some(true),
        "truncated should be true for long source"
    );
}

// ============================================================================
// Scenario: Short function source is stored in full
// ============================================================================
#[test]
fn test_short_function_source_stored_in_full() {
    // @step Given a project contains a function with fewer than 100 lines and under 4096 bytes
    let short_source = "function short() {\n  return 42;\n}";

    // @step When the project is indexed with ast_index
    let props = node_props("src/short.ts", short_source, "Function", "short");
    let result = prop(&props, "source");
    let truncated = props.get("truncated").and_then(|v| v.as_bool());

    // @step Then the function node source contains the complete function body
    assert_eq!(result, short_source, "source should be complete");

    // @step And the function node has truncated set to false
    assert_eq!(
        truncated,
        Some(false),
        "truncated should be false for short source"
    );
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
    let ts_text = "/** Dispatches an action. */\nfunction dispatch() {}\n";
    let ts_props = node_props("src/dispatch.ts", ts_text, "Function", "dispatch");
    let ts_doc = prop(&ts_props, "docstring");
    assert!(
        ts_doc.contains("Dispatches an action"),
        "JSDoc should be extracted for TS, got: {ts_doc}"
    );

    // @step And rustdoc comments are extracted for Rust functions
    let rust_text = "/// Finds all reachable nodes.\n/// Uses BFS traversal.\nfn find_all() {}\n";
    let rust_props = node_props("src/lib.rs", rust_text, "Function", "find_all");
    let rust_doc = prop(&rust_props, "docstring");
    assert!(
        rust_doc.contains("Finds all reachable nodes"),
        "rustdoc should be extracted for Rust, got: {rust_doc}"
    );

    // @step And triple-quoted docstrings are extracted for Python functions
    let py_text = "def process():\n    \"\"\"Process a user record.\"\"\"\n    pass\n";
    let py_props = node_props("service.py", py_text, "Function", "process");
    let py_doc = prop(&py_props, "docstring");
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
    let ts_sig = "function dispatch(action: string, options: Options) {}\n";
    let ts_props = node_props("src/dispatch.ts", ts_sig, "Function", "dispatch");
    let ts_params = prop(&ts_props, "parameters");
    assert_eq!(ts_params, "action, options", "TS params should strip types");

    let rust_sig = "struct S;\nimpl S {\n    fn process(&self, name: String, age: i32) {}\n}\n";
    let rust_props = node_props("src/lib.rs", rust_sig, "Function", "process");
    let rust_params = prop(&rust_props, "parameters");
    assert_eq!(
        rust_params, "name, age",
        "Rust params should strip self and types"
    );

    // @step And language-specific self parameters are filtered appropriately
    let py_sig = "class Service:\n    def method(self, name: str, age: int):\n        pass\n";
    let py_props = node_props("service.py", py_sig, "Function", "method");
    let py_params = prop(&py_props, "parameters");
    assert_eq!(
        py_params, "name, age",
        "Python params should filter self and strip types"
    );

    let go_sig = "package server\n\nfunc (s *Server) Handle(ctx Context, req Request) {}\n";
    let go_props = node_props("server.go", go_sig, "Function", "Handle");
    let go_params = prop(&go_props, "parameters");
    assert_eq!(
        go_params, "ctx, req",
        "Go params should filter receiver and strip types"
    );
}
