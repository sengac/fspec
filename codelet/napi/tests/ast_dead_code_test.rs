// Feature: spec/features/ast-dead-code-detection.feature
//
// Dead Code Detection via AST Graph — Calls/TypeRef Edge Population + Orphan Query
//
// Tests for:
// 1. Calls edge extraction (same-file and cross-file function calls)
// 2. TypeRef edge extraction (parameter and return type references)
// 3. Filtering out method calls and builtins from Calls edges
// 4. Dead code queries via nanograph anti-join (orphan files, uncalled functions, unreferenced types)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::walk_and_extract;
use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{count_edges, write_test_file};

/// The AST code schema for loading extracted entities.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Helper: find edges by type with optional from/to slug filter
fn find_edges<'a>(
    entities: &'a [GraphEntity],
    edge_type: &str,
    from_contains: Option<&str>,
    to_contains: Option<&str>,
) -> Vec<&'a GraphEntity> {
    entities
        .iter()
        .filter(|e| match e {
            GraphEntity::Edge {
                edge_type: et,
                from_slug,
                to_slug,
                ..
            } => {
                et == edge_type
                    && from_contains.map_or(true, |f| from_slug.contains(f))
                    && to_contains.map_or(true, |t| to_slug.contains(t))
            }
            _ => false,
        })
        .collect()
}

// ============================================================================
// Scenario: Extract Calls edge for cross-file function call via import
// ============================================================================
#[tokio::test]
async fn test_extract_calls_edge_cross_file_via_import() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a TypeScript file "src/app.ts" with content:
    let app_content = r#"
import { validateConfig } from './config';
function main() { validateConfig(); }
"#;
    write_test_file(project_dir, "src/app.ts", app_content);

    // @step And a TypeScript file "src/config.ts" with content:
    let config_content = r#"
export function validateConfig() { return true; }
"#;
    write_test_file(project_dir, "src/config.ts", config_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the TS extractor processes both files
    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // @step Then a Calls edge should exist from "src-app-ts::main" to "src-config-ts::validateConfig"
    let calls = find_edges(&entities, "Calls", Some("app"), Some("config"));
    assert!(
        !calls.is_empty(),
        "Should have a Calls edge from app::main to config::validateConfig, got 0 Calls edges. All edges: {:?}",
        entities.iter().filter(|e| matches!(e, GraphEntity::Edge { .. })).collect::<Vec<_>>()
    );
}

// ============================================================================
// Scenario: Extract Calls edge for same-file function call
// ============================================================================
#[tokio::test]
async fn test_extract_calls_edge_same_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a TypeScript file "src/utils.ts" with content:
    let utils_content = r#"
function foo() { bar(); }
function bar() { return 1; }
"#;
    write_test_file(project_dir, "src/utils.ts", utils_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the TS extractor processes the file
    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // @step Then a Calls edge should exist from "src-utils-ts::foo" to "src-utils-ts::bar"
    let calls = find_edges(&entities, "Calls", Some("foo"), Some("bar"));
    assert!(
        !calls.is_empty(),
        "Should have a Calls edge from foo to bar (same file), got 0. All Calls edges: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}

// ============================================================================
// Scenario: Ignore method calls and builtins — no Calls edges for dotted expressions
// ============================================================================
#[tokio::test]
async fn test_ignore_method_calls_and_builtins() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a TypeScript file "src/main.ts" with content:
    let main_content = r#"
function run() {
  console.log('hello');
  process.exit(0);
  obj.method();
}
"#;
    write_test_file(project_dir, "src/main.ts", main_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the TS extractor processes the file
    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // @step Then no Calls edges should be emitted
    let calls_count = count_edges(&entities, "Calls");
    assert_eq!(
        calls_count, 0,
        "Should have 0 Calls edges (console.log, process.exit, obj.method are not tracked), got {}",
        calls_count
    );
}

// ============================================================================
// Scenario: Extract TypeRef edges from function parameter and return types
// ============================================================================
#[tokio::test]
async fn test_extract_typeref_edges_from_function_signatures() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a TypeScript file "src/handler.ts" with content:
    // Note: The TS extractor needs interface/type declarations + function with type annotations
    let handler_content = r#"
interface Request { url: string; }
interface Response { status: number; }
function handler(req: Request): Response { return { status: 200 }; }
"#;
    write_test_file(project_dir, "src/handler.ts", handler_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the TS extractor processes the file
    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // @step Then a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Request"
    let request_refs = find_edges(&entities, "TypeRef", Some("handler"), Some("Request"));
    assert!(
        !request_refs.is_empty(),
        "Should have a TypeRef edge from handler to Request"
    );

    // @step And a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Response"
    let response_refs = find_edges(&entities, "TypeRef", Some("handler"), Some("Response"));
    assert!(
        !response_refs.is_empty(),
        "Should have a TypeRef edge from handler to Response"
    );
}

// ============================================================================
// Scenario: Detect orphan files with no incoming Imports edges
// ============================================================================
#[tokio::test]
async fn test_detect_orphan_files_no_imports() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a graph with File "src/used.ts" imported by "src/app.ts"
    let app_content = r#"
import { helper } from './used';
export function main() { helper(); }
"#;
    let used_content = r#"
export function helper() { return 1; }
"#;
    write_test_file(project_dir, "src/app.ts", app_content);
    write_test_file(project_dir, "src/used.ts", used_content);

    // @step And a graph with File "src/orphan.ts" imported by no other file
    let orphan_content = r#"
export function orphanFn() { return 'nobody calls me'; }
"#;
    write_test_file(project_dir, "src/orphan.ts", orphan_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // Load into graph
    let db_path = temp_dir.path().join("test-orphan.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");
    db.load_entities(&entities)
        .await
        .expect("load should succeed");

    // @step When the ast_dead_code action runs with entity_type "File"
    // Use the dead code query directly
    let dead_code_queries = r#"
query orphan_files() {
    match {
        $f: File
        not { $other imports $f }
    }
    return { $f.slug, $f.path, $f.language, $f.isTest }
}
"#;
    let db = db.with_query_source(dead_code_queries);
    let result = db.query("orphan_files", None).await.expect("query should succeed");
    let orphans = result.as_array().expect("should be array");

    // Filter: exclude test files and stubs (language=null)
    let orphan_paths: Vec<&str> = orphans
        .iter()
        .filter_map(|o| {
            let is_test = o.get("isTest").and_then(|v| v.as_bool()).unwrap_or(false);
            let has_language = o.get("language").and_then(|v| v.as_str()).is_some();
            if !is_test && has_language {
                o.get("path").and_then(|v| v.as_str())
            } else {
                None
            }
        })
        .collect();

    // @step Then the result should include "src/orphan.ts"
    assert!(
        orphan_paths.contains(&"src/orphan.ts"),
        "Orphan files should include src/orphan.ts, got: {:?}",
        orphan_paths
    );

    // @step And the result should not include "src/used.ts"
    assert!(
        !orphan_paths.contains(&"src/used.ts"),
        "Orphan files should NOT include src/used.ts (it is imported)"
    );
}

// ============================================================================
// Scenario: Detect uncalled functions with no incoming Calls edges
// ============================================================================
#[tokio::test]
async fn test_detect_uncalled_functions_no_calls() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a graph with Function "app::main" that calls "app::helper"
    // @step And a graph with Function "app::unused" that is never called
    let app_content = r#"
function main() { helper(); }
function helper() { return 1; }
function unused() { return 'nobody calls me'; }
"#;
    write_test_file(project_dir, "src/app.ts", app_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // Load into graph
    let db_path = temp_dir.path().join("test-uncalled.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");
    db.load_entities(&entities)
        .await
        .expect("load should succeed");

    // @step When the ast_dead_code action runs with entity_type "Function"
    let dead_code_queries = r#"
query uncalled_functions() {
    match {
        $fn: Function
        not { $caller calls $fn }
    }
    return { $fn.slug, $fn.name, $fn.isPublic }
}
"#;
    let db = db.with_query_source(dead_code_queries);
    let result = db
        .query("uncalled_functions", None)
        .await
        .expect("query should succeed");
    let uncalled = result.as_array().expect("should be array");

    let uncalled_names: Vec<&str> = uncalled
        .iter()
        .filter_map(|f| f.get("name").and_then(|v| v.as_str()))
        .collect();

    // @step Then the result should include "app::unused"
    assert!(
        uncalled_names.contains(&"unused"),
        "Uncalled functions should include 'unused', got: {:?}",
        uncalled_names
    );

    // @step And the result should not include "app::helper"
    assert!(
        !uncalled_names.contains(&"helper"),
        "Uncalled functions should NOT include 'helper' (it is called by main)"
    );
}

// ============================================================================
// Scenario: Detect unreferenced types with no incoming TypeRef edges
// ============================================================================
#[tokio::test]
async fn test_detect_unreferenced_types_no_typerefs() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a graph with Type "handler-ts::Request" referenced by "handler-ts::handler"
    // @step And a graph with Type "handler-ts::OldInterface" referenced by no function
    let handler_content = r#"
interface Request { url: string; }
interface Response { status: number; }
interface OldInterface { legacy: boolean; }
function handler(req: Request): Response { return { status: 200 }; }
"#;
    write_test_file(project_dir, "src/handler.ts", handler_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    // Load into graph
    let db_path = temp_dir.path().join("test-unreferenced.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");
    db.load_entities(&entities)
        .await
        .expect("load should succeed");

    // @step When the ast_dead_code action runs with entity_type "Type"
    let dead_code_queries = r#"
query unreferenced_types() {
    match {
        $t: Type
        not { $fn typeRef $t }
    }
    return { $t.slug, $t.name, $t.typeKind }
}
"#;
    let db = db.with_query_source(dead_code_queries);
    let result = db
        .query("unreferenced_types", None)
        .await
        .expect("query should succeed");
    let unreferenced = result.as_array().expect("should be array");

    let unreferenced_names: Vec<&str> = unreferenced
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
        .collect();

    // @step Then the result should include "handler-ts::OldInterface"
    assert!(
        unreferenced_names.contains(&"OldInterface"),
        "Unreferenced types should include 'OldInterface', got: {:?}",
        unreferenced_names
    );

    // @step And the result should not include "handler-ts::Request"
    assert!(
        !unreferenced_names.contains(&"Request"),
        "Unreferenced types should NOT include 'Request' (it is referenced by handler)"
    );
}

// ============================================================================
// Scenario: Exclude test files from dead code results by default
// ============================================================================
#[tokio::test]
async fn test_exclude_test_files_from_dead_code_results() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a graph with File "src/__tests__/app.test.ts" that is a test file
    let test_content = r#"
export function testMain() { return true; }
"#;
    write_test_file(project_dir, "src/__tests__/app.test.ts", test_content);

    // Add a non-test source file for contrast
    let src_content = r#"
export function realCode() { return 1; }
"#;
    write_test_file(project_dir, "src/real.ts", src_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step And that test file is never imported by any other file
    let entities = walk_and_extract(project_dir, true).expect("extraction should succeed");

    let db_path = temp_dir.path().join("test-exclude-tests.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");
    db.load_entities(&entities)
        .await
        .expect("load should succeed");

    // @step When the ast_dead_code action runs with entity_type "File"
    let dead_code_queries = r#"
query orphan_files() {
    match {
        $f: File
        not { $other imports $f }
    }
    return { $f.slug, $f.path, $f.language, $f.isTest }
}
"#;
    let db = db.with_query_source(dead_code_queries);
    let result = db.query("orphan_files", None).await.expect("query should succeed");
    let orphans = result.as_array().expect("should be array");

    // Apply the test file filter (same as dispatch_ast_dead_code will do)
    let non_test_orphan_paths: Vec<&str> = orphans
        .iter()
        .filter_map(|o| {
            let is_test = o.get("isTest").and_then(|v| v.as_bool()).unwrap_or(false);
            let has_language = o.get("language").and_then(|v| v.as_str()).is_some();
            if !is_test && has_language {
                o.get("path").and_then(|v| v.as_str())
            } else {
                None
            }
        })
        .collect();

    // @step Then the result should not include "src/__tests__/app.test.ts"
    assert!(
        !non_test_orphan_paths.iter().any(|p| p.contains("test")),
        "Filtered dead code results should NOT include test files, got: {:?}",
        non_test_orphan_paths
    );
}
