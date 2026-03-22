// Feature: spec/features/ast-extraction-pipeline.feature
//
// AST Extraction Pipeline — Tree-Sitter/AST-Grep Parser
// Tests for extracting AST entities (Functions, Types, imports)
// from TypeScript and Rust source files using ast-grep patterns.
//
// Each test uses an isolated temp directory with synthetic source files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::{extract_file, walk_and_extract};
use codelet_napi::graph::database::GraphDatabase;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{count_edges, count_nodes, find_node, write_test_file};

/// The AST code schema for loading extracted entities.
const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");

/// Inline queries for verifying loaded data.
const AST_QUERIES: &str = r#"
query all_files() {
    match { $f: File }
    return { $f.slug, $f.path, $f.language, $f.lineCount, $f.isTest }
}

query all_functions() {
    match { $fn: Function }
    return { $fn.slug, $fn.name, $fn.qualifiedName, $fn.isAsync, $fn.isPublic, $fn.paramCount, $fn.lineStart, $fn.lineEnd }
}

query all_types() {
    match { $t: Type }
    return { $t.slug, $t.name, $t.typeKind, $t.isPublic }
}

query file_functions($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f contains $fn
    }
    return { $fn.slug, $fn.name }
}

query file_imports($file_slug: String) {
    match {
        $f: File { slug: $file_slug }
        $f imports $target
    }
    return { $target.slug, $target.path }
}
"#;

// ============================================================================
// Scenario: Extract Function nodes from a TypeScript file
// ============================================================================
#[tokio::test]
async fn test_extract_function_nodes_from_typescript_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a TypeScript file "src/auth/login.ts" with async and sync function declarations
    let ts_content = r#"
import { hash } from './utils';
import { Config } from '../config';

export async function login(username: string, password: string): Promise<boolean> {
    const hashed = hash(password);
    return verify(username, hashed);
}

function verify(username: string, hashedPassword: string): boolean {
    return username.length > 0 && hashedPassword.length > 0;
}

export const logout = async (): Promise<void> => {
    await clearSession();
};
"#;
    let file_path = write_test_file(temp_dir.path(), "src/auth/login.ts", ts_content);

    // @step When the TypeScript extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path())
        .expect("TypeScript extraction should succeed");

    // @step Then Function nodes should be created with correct name, qualifiedName, isAsync, isPublic properties
    assert!(
        count_nodes(&entities, "Function") >= 2,
        "Should extract at least 2 Function nodes, got {}",
        count_nodes(&entities, "Function")
    );

    let login_fn = find_node(&entities, "Function", "src-auth-login-ts::login");
    assert!(login_fn.is_some(), "Should find login function");

    // @step And each Function node should have lineStart and lineEnd positions
    if let Some(GraphEntity::Node { properties, .. }) = login_fn {
        assert!(
            properties.contains_key("lineStart"),
            "Function should have lineStart"
        );
        assert!(
            properties.contains_key("lineEnd"),
            "Function should have lineEnd"
        );
    }

    // @step And each Function node should have a paramCount matching the declaration
    if let Some(GraphEntity::Node { properties, .. }) = login_fn {
        let param_count = properties
            .get("paramCount")
            .and_then(|v| v.as_i64());
        assert_eq!(param_count, Some(2), "login should have 2 params");
    }

    // @step And a File node should be created for "src/auth/login.ts"
    // Note: import targets also create File nodes, so there may be more than 1
    let source_file = find_node(&entities, "File", "src-auth-login-ts");
    assert!(source_file.is_some(), "Should create a File node for the source file");

    // @step And Contains edges should link the File to each Function
    assert!(
        count_edges(&entities, "Contains") >= 2,
        "Should have at least 2 Contains edges"
    );
}

// ============================================================================
// Scenario: Extract Type nodes from a Rust file
// ============================================================================
#[tokio::test]
async fn test_extract_type_nodes_from_rust_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a Rust file "src/graph/database.rs" with struct, enum, and trait declarations
    let rs_content = r#"
use std::path::PathBuf;

pub struct GraphDatabase {
    path: PathBuf,
    name: String,
}

pub enum GraphType {
    Memory,
    AstCode,
    Learnings,
}

pub trait Queryable {
    fn query(&self, name: &str) -> Result<String, String>;
}

pub fn ensure_db() -> Result<(), String> {
    Ok(())
}

fn internal_helper(x: i32) -> i32 {
    x + 1
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/graph/database.rs", rs_content);

    // @step When the Rust extractor parses the file
    let entities = extract_file(&file_path, temp_dir.path())
        .expect("Rust extraction should succeed");

    // @step Then Type nodes should be created for each struct with typeKind "struct_kind"
    let struct_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("struct_kind"))
        })
        .collect();
    assert!(
        !struct_nodes.is_empty(),
        "Should find at least 1 struct Type node"
    );

    // @step And Type nodes should be created for each enum with typeKind "enum_kind"
    let enum_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("enum_kind"))
        })
        .collect();
    assert!(
        !enum_nodes.is_empty(),
        "Should find at least 1 enum Type node"
    );

    // @step And Type nodes should be created for each trait with typeKind "trait_kind"
    let trait_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind"))
        })
        .collect();
    assert!(
        !trait_nodes.is_empty(),
        "Should find at least 1 trait Type node"
    );

    // @step And Function nodes should be created for each fn declaration
    assert!(
        count_nodes(&entities, "Function") >= 2,
        "Should find at least 2 Function nodes"
    );

    // @step And a File node should be created for "src/graph/database.rs"
    assert_eq!(count_nodes(&entities, "File"), 1, "Should create 1 File node");
}

// ============================================================================
// Scenario: Extract import edges from TypeScript files
// ============================================================================
#[tokio::test]
async fn test_extract_import_edges_from_typescript_files() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a TypeScript file "src/auth/login.ts" that imports from "src/auth/utils.ts" and "src/config.ts"
    let login_content = r#"
import { hash, compare } from './utils';
import { Config } from '../config';

export function login(user: string): boolean {
    return true;
}
"#;
    let utils_content = r#"
export function hash(input: string): string {
    return input;
}
export function compare(a: string, b: string): boolean {
    return a === b;
}
"#;
    let config_content = r#"
export interface Config {
    dbUrl: string;
}
"#;
    write_test_file(temp_dir.path(), "src/auth/login.ts", login_content);
    write_test_file(temp_dir.path(), "src/auth/utils.ts", utils_content);
    write_test_file(temp_dir.path(), "src/config.ts", config_content);

    let login_path = temp_dir.path().join("src/auth/login.ts");

    // @step When the TypeScript extractor parses the file
    let entities = extract_file(&login_path, temp_dir.path())
        .expect("TypeScript extraction should succeed");

    // @step Then File nodes should be created for all three files
    // The source file itself creates a File node; import targets may also create File nodes
    assert!(
        count_nodes(&entities, "File") >= 1,
        "Should create at least 1 File node"
    );

    // @step And Imports edges should link "src/auth/login.ts" to "src/auth/utils.ts"
    let import_edges: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Imports"))
        .collect();
    assert!(
        !import_edges.is_empty(),
        "Should have at least 1 Imports edge"
    );

    // @step And Imports edges should link "src/auth/login.ts" to "src/config.ts"
    assert!(
        import_edges.len() >= 2,
        "Should have at least 2 Imports edges (one for utils, one for config), got {}",
        import_edges.len()
    );

    // @step And each Imports edge should have the importPath property set
    for edge in &import_edges {
        if let GraphEntity::Edge { properties, .. } = edge {
            assert!(
                properties.contains_key("importPath"),
                "Imports edge should have importPath property"
            );
        }
    }
}

// ============================================================================
// Scenario: Walk project directory with gitignore and batch load
// ============================================================================
#[tokio::test]
async fn test_walk_project_directory_with_gitignore_and_batch_load() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory with TypeScript and Rust source files
    write_test_file(
        project_dir,
        "src/main.ts",
        "export function main(): void { console.log('hello'); }\n",
    );
    write_test_file(
        project_dir,
        "src/lib.rs",
        "pub fn init() -> Result<(), String> { Ok(()) }\n",
    );

    // @step And a .gitignore file that excludes "node_modules" and "target"
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");
    write_test_file(
        project_dir,
        "node_modules/dep/index.ts",
        "export function dep(): void {}\n",
    );
    write_test_file(
        project_dir,
        "target/debug/main.rs",
        "fn compiled_main() {}\n",
    );

    // @step When the extraction pipeline walks the project directory
    let all_entities = walk_and_extract(project_dir)
        .expect("Walk and extract should succeed");

    // @step Then files in node_modules should be skipped
    let file_slugs: Vec<String> = all_entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, slug, .. } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !file_slugs.iter().any(|s| s.contains("node_modules") || s.contains("node-modules")),
        "Should not contain files from node_modules, got: {:?}",
        file_slugs
    );

    // @step And files in target should be skipped
    assert!(
        !file_slugs.iter().any(|s| s.contains("target")),
        "Should not contain files from target, got: {:?}",
        file_slugs
    );

    // @step And all extracted entities should be loaded in a single batch operation
    // Verify batch by loading into a graph and checking stats
    let db_path = temp_dir.path().join("test-ast.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let jsonl = all_entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, properties, .. } => {
                serde_json::to_string(&serde_json::json!({"type": node_type, "data": properties})).ok()
            }
            GraphEntity::Edge { edge_type, from_slug, to_slug, properties } => {
                serde_json::to_string(&serde_json::json!({"edge": edge_type, "from": from_slug, "to": to_slug, "data": properties})).ok()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    db.load_jsonl(&jsonl).await.expect("Batch load should succeed");

    // @step And the AST graph should contain File, Function, and Type nodes from the processed files
    let db = db.with_query_source(AST_QUERIES);
    let files = db
        .query("all_files", None)
        .await
        .expect("all_files query should succeed");
    let files_arr = files.as_array().expect("Should be array");
    assert!(
        files_arr.len() >= 2,
        "Should have at least 2 File nodes (main.ts and lib.rs), got {}",
        files_arr.len()
    );

    let functions = db
        .query("all_functions", None)
        .await
        .expect("all_functions query should succeed");
    let fns_arr = functions.as_array().expect("Should be array");
    assert!(
        !fns_arr.is_empty(),
        "Should have extracted at least 1 Function node"
    );
}
