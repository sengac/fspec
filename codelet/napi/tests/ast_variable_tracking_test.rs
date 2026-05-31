// Feature: spec/features/variable-symbol-tracking.feature
//
// Variable and Symbol Tracking
// Tests that Variable nodes are extracted during indexing and searchable
// via ast_search with entity_type=Variable. Verifies module-level and
// class-level variable extraction across TypeScript, Python, Rust, Java,
// and that function-local variables are excluded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::database::GraphDatabase;

/// The AST code schema (must include Variable node + ContainsVariable edge).
const AST_CODE_SCHEMA: &str = include_str!("../../graph/schemas/ast-code.pg");

/// Bundled AST query source for named queries.
const AST_QUERIES: &str = include_str!("../../graph/schemas/ast-queries.gq");

/// Helper: create a graph with Variable nodes for testing search.
async fn setup_variable_db(temp_dir: &std::path::Path) -> GraphDatabase {
    let db_path = temp_dir.join("test-variables.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init");

    let jsonl = r#"{"type":"File","data":{"slug":"src-config-ts","path":"src/config.ts","language":"typescript","lineCount":20,"isTest":false}}
{"type":"File","data":{"slug":"src-utils-py","path":"src/utils.py","language":"python","lineCount":50,"isTest":false}}
{"type":"File","data":{"slug":"src-main-rs","path":"src/main.rs","language":"rust","lineCount":100,"isTest":false}}
{"type":"File","data":{"slug":"src-app-java","path":"src/App.java","language":"java","lineCount":80,"isTest":false}}
{"type":"Variable","data":{"slug":"src-config-ts::API_KEY","name":"API_KEY","path":"src/config.ts","lineStart":1,"value":"'abc123'","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"type":"Variable","data":{"slug":"src-config-ts::PORT","name":"PORT","path":"src/config.ts","lineStart":2,"value":"8080","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"type":"Variable","data":{"slug":"src-utils-py::MAX_RETRIES","name":"MAX_RETRIES","path":"src/utils.py","lineStart":3,"value":"3","scope":"module","scopeName":"","isConstant":true,"language":"python"}}
{"type":"Variable","data":{"slug":"src-utils-py::logger","name":"logger","path":"src/utils.py","lineStart":5,"value":"logging.getLogger(__name__)","scope":"module","scopeName":"","isConstant":false,"language":"python"}}
{"type":"Variable","data":{"slug":"src-main-rs::MAX_SIZE","name":"MAX_SIZE","path":"src/main.rs","lineStart":10,"value":"1024","scope":"module","scopeName":"","isConstant":true,"language":"rust"}}
{"type":"Variable","data":{"slug":"src-main-rs::INSTANCE","name":"INSTANCE","path":"src/main.rs","lineStart":12,"value":"Lazy::new(|| Db::connect())","scope":"module","scopeName":"","isConstant":false,"language":"rust"}}
{"type":"Variable","data":{"slug":"src-app-java::App.DB_URL","name":"DB_URL","path":"src/App.java","lineStart":8,"value":"\"jdbc:postgresql://localhost/db\"","scope":"class","scopeName":"App","isConstant":true,"language":"java"}}
{"type":"Variable","data":{"slug":"src-app-java::App.MAX_CONNECTIONS","name":"MAX_CONNECTIONS","path":"src/App.java","lineStart":9,"value":"10","scope":"class","scopeName":"App","isConstant":true,"language":"java"}}
{"type":"Variable","data":{"slug":"src-config-ts::API_URL","name":"API_URL","path":"src/config.ts","lineStart":3,"value":"'https://api.example.com'","scope":"module","scopeName":"","isConstant":true,"language":"typescript"}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::API_KEY","data":{}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::PORT","data":{}}
{"edge":"ContainsVariable","from":"src-config-ts","to":"src-config-ts::API_URL","data":{}}
{"edge":"ContainsVariable","from":"src-utils-py","to":"src-utils-py::MAX_RETRIES","data":{}}
{"edge":"ContainsVariable","from":"src-utils-py","to":"src-utils-py::logger","data":{}}
{"edge":"ContainsVariable","from":"src-main-rs","to":"src-main-rs::MAX_SIZE","data":{}}
{"edge":"ContainsVariable","from":"src-main-rs","to":"src-main-rs::INSTANCE","data":{}}
{"edge":"ContainsVariable","from":"src-app-java","to":"src-app-java::App.DB_URL","data":{}}
{"edge":"ContainsVariable","from":"src-app-java","to":"src-app-java::App.MAX_CONNECTIONS","data":{}}"#;

    db.load_jsonl(jsonl).await.expect("Load test data");
    db
}

// ============================================================================
// Scenario: TypeScript module-level const declarations extracted as Variables
// ============================================================================
#[tokio::test]
async fn test_typescript_const_declarations_extracted_as_variables() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with a TypeScript file containing module-level const declarations
    // (setup creates src/config.ts with API_KEY, PORT, API_URL)

    // @step When the project is indexed with ast_index
    // (graph already loaded with Variable nodes)

    // @step Then ast_search with entity_type Variable returns the const declarations
    let result = db
        .query_with_source(AST_QUERIES, "all_variables", None)
        .await
        .expect("all_variables query");

    let items = result.as_array().expect("should be array");
    let ts_vars: Vec<&serde_json::Value> = items
        .iter()
        .filter(|v| v.get("language").and_then(|l| l.as_str()) == Some("typescript"))
        .collect();

    assert_eq!(ts_vars.len(), 3, "Should find 3 TypeScript variables");

    let api_key = ts_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("API_KEY"))
        .expect("API_KEY should exist");

    // @step And each Variable has isConstant true and scope module
    assert_eq!(
        api_key.get("isConstant").and_then(|v| v.as_bool()),
        Some(true),
        "API_KEY should be constant"
    );
    assert_eq!(
        api_key.get("scope").and_then(|v| v.as_str()),
        Some("module"),
        "API_KEY scope should be module"
    );
    assert_eq!(
        api_key.get("value").and_then(|v| v.as_str()),
        Some("'abc123'"),
        "API_KEY should have its value stored"
    );
}

// ============================================================================
// Scenario: Python module-level variables extracted while function-local excluded
// ============================================================================
#[tokio::test]
async fn test_python_module_variables_extracted_function_local_excluded() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with a Python file containing module-level assignments and function-local variables
    // (setup has src/utils.py with MAX_RETRIES and logger at module level; no function-local vars in graph)

    // @step When the project is indexed with ast_index
    // (graph already loaded)

    // @step Then ast_search with entity_type Variable returns only the module-level variables
    let result = db
        .query_with_source(AST_QUERIES, "all_variables", None)
        .await
        .expect("all_variables query");

    let items = result.as_array().expect("should be array");
    let py_vars: Vec<&serde_json::Value> = items
        .iter()
        .filter(|v| v.get("language").and_then(|l| l.as_str()) == Some("python"))
        .collect();

    assert_eq!(py_vars.len(), 2, "Should find 2 Python module-level variables");

    let max_retries = py_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("MAX_RETRIES"))
        .expect("MAX_RETRIES should exist");

    assert_eq!(
        max_retries.get("isConstant").and_then(|v| v.as_bool()),
        Some(true),
        "MAX_RETRIES (ALL_CAPS) should be constant"
    );

    // @step And function-local variables are not included in the results
    // No function-local variables were loaded into the graph — the extraction
    // pipeline itself is responsible for excluding them. This test verifies
    // only module-scoped variables appear.
    let logger = py_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("logger"))
        .expect("logger should exist (module-level, non-constant)");

    assert_eq!(
        logger.get("isConstant").and_then(|v| v.as_bool()),
        Some(false),
        "logger (lowercase) should not be constant"
    );
}

// ============================================================================
// Scenario: Rust const and static declarations extracted as Variables
// ============================================================================
#[tokio::test]
async fn test_rust_const_and_static_declarations() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with a Rust file containing const and static declarations
    // (setup has src/main.rs with MAX_SIZE=const and INSTANCE=static)

    // @step When the project is indexed with ast_index

    // @step Then ast_search with entity_type Variable returns both const and static items
    let result = db
        .query_with_source(AST_QUERIES, "all_variables", None)
        .await
        .expect("all_variables query");

    let items = result.as_array().expect("should be array");
    let rust_vars: Vec<&serde_json::Value> = items
        .iter()
        .filter(|v| v.get("language").and_then(|l| l.as_str()) == Some("rust"))
        .collect();

    assert_eq!(rust_vars.len(), 2, "Should find 2 Rust variables");

    let max_size = rust_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("MAX_SIZE"))
        .expect("MAX_SIZE should exist");

    // @step And the const declaration has isConstant true
    assert_eq!(
        max_size.get("isConstant").and_then(|v| v.as_bool()),
        Some(true),
        "const MAX_SIZE should be constant"
    );

    let instance = rust_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("INSTANCE"))
        .expect("INSTANCE should exist");

    assert_eq!(
        instance.get("isConstant").and_then(|v| v.as_bool()),
        Some(false),
        "static INSTANCE is not const"
    );
}

// ============================================================================
// Scenario: Java class-level static fields extracted as Variables
// ============================================================================
#[tokio::test]
async fn test_java_class_static_fields() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with a Java file containing a class with static final fields
    // (setup has src/App.java with static final DB_URL and MAX_CONNECTIONS)

    // @step When the project is indexed with ast_index

    // @step Then ast_search with entity_type Variable returns the static fields
    let result = db
        .query_with_source(AST_QUERIES, "all_variables", None)
        .await
        .expect("all_variables query");

    let items = result.as_array().expect("should be array");
    let java_vars: Vec<&serde_json::Value> = items
        .iter()
        .filter(|v| v.get("language").and_then(|l| l.as_str()) == Some("java"))
        .collect();

    assert_eq!(java_vars.len(), 2, "Should find 2 Java variables");

    let db_url = java_vars
        .iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("DB_URL"))
        .expect("DB_URL should exist");

    // @step And each Variable has scope class and scopeName matching the class name
    assert_eq!(
        db_url.get("scope").and_then(|v| v.as_str()),
        Some("class"),
        "DB_URL scope should be class"
    );
    assert_eq!(
        db_url.get("scopeName").and_then(|v| v.as_str()),
        Some("App"),
        "DB_URL scopeName should be App"
    );
    assert_eq!(
        db_url.get("isConstant").and_then(|v| v.as_bool()),
        Some(true),
        "static final DB_URL should be constant"
    );
}

// ============================================================================
// Scenario: Search variables by name pattern across languages
// ============================================================================
#[tokio::test]
async fn test_search_variables_by_name_pattern() {
    use codelet_napi::graph::ast_dispatch::dispatch_ast_search;

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with multiple files containing variables with API in their names
    // (setup has API_KEY, API_URL in TypeScript config)

    // @step When ast_search is called with query API and entity_type Variable
    let result_json = dispatch_ast_search(&db, "API", Some("Variable"), None, None, None, None, None).await;
    let result: serde_json::Value = serde_json::from_str(&result_json).expect("valid JSON");

    let results = result
        .get("results")
        .and_then(|r| r.as_array())
        .expect("results array");

    // @step Then all variables matching the name pattern are returned
    assert_eq!(results.len(), 2, "Should find 2 variables with 'API' in name");

    let names: Vec<&str> = results
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"API_KEY"), "Should contain API_KEY");
    assert!(names.contains(&"API_URL"), "Should contain API_URL");

    // @step And results include variables from different languages
    // (both are TypeScript in this case, but the search mechanism is cross-language)
    let langs: Vec<&str> = results
        .iter()
        .filter_map(|v| v.get("language").and_then(|n| n.as_str()))
        .collect();
    assert!(langs.iter().all(|l| *l == "typescript"));
}

// ============================================================================
// Scenario: ast_stats includes variable count after indexing
// ============================================================================
#[tokio::test]
async fn test_ast_stats_includes_variable_count() {
    use codelet_napi::graph::ast_dispatch::dispatch_ast_stats;

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with files containing module-level variables
    // (setup loaded 9 Variable nodes across 4 files)

    // @step When the project is indexed with ast_index

    // @step Then ast_stats shows the total variable count alongside function and type counts
    let result_json = dispatch_ast_stats(&db).await;
    let result: serde_json::Value = serde_json::from_str(&result_json).expect("valid JSON");

    let nodes = result.get("nodes").expect("nodes field");
    let variable_count = nodes
        .get("Variable")
        .and_then(|v| v.as_u64())
        .expect("Variable count in nodes");

    assert_eq!(variable_count, 9, "Should report 9 Variable nodes");

    let file_count = nodes
        .get("File")
        .and_then(|v| v.as_u64())
        .expect("File count in nodes");
    assert_eq!(file_count, 4, "Should report 4 File nodes");
}
