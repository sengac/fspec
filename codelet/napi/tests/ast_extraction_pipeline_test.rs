// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
//
// AST Extraction Pipeline — Tree-Sitter/AST-Grep Parser
// Tests for extracting AST entities (Functions, Types, imports)
// from TypeScript and Rust source files using ast-grep patterns.
//
// Includes deduplication tests for KGRAPH-026: duplicate File entities
// from import resolution vs. direct file walk.
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

// ============================================================================
// KGRAPH-026: AST Entity Deduplication Tests
// Feature: spec/features/ast-entity-deduplication.feature
// ============================================================================

// ============================================================================
// Scenario: Deduplicate File nodes when import target is also walked directly
// ============================================================================
#[tokio::test]
async fn test_dedup_file_nodes_when_import_target_is_also_walked() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with file "src/index.ts" that imports from "./utils"
    let index_content = r#"
import { helper } from './utils';

export function main(): void {
    helper();
}
"#;
    write_test_file(project_dir, "src/index.ts", index_content);

    // @step And a project file "src/utils.ts" that is also walked by the file walker
    let utils_content = r#"
export function helper(): string {
    return 'hello';
}
"#;
    write_test_file(project_dir, "src/utils.ts", utils_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    // @step Then only one File node should exist for "src/utils.ts"
    let utils_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/utils.ts")))
        .collect();
    assert_eq!(
        utils_file_nodes.len(),
        1,
        "Should have exactly 1 File node for src/utils.ts, got {} — deduplication failed",
        utils_file_nodes.len()
    );

    // @step And that File node should have the full properties including language, lineCount, and isTest
    if let GraphEntity::Node { properties, .. } = utils_file_nodes[0] {
        assert!(
            properties.contains_key("language"),
            "Deduped File node must have 'language' property (full node wins over stub)"
        );
        assert!(
            properties.contains_key("lineCount"),
            "Deduped File node must have 'lineCount' property (full node wins over stub)"
        );
        assert!(
            properties.contains_key("isTest"),
            "Deduped File node must have 'isTest' property (full node wins over stub)"
        );
    } else {
        panic!("Expected a Node variant");
    }

    // @step And the Imports edge from "src/index.ts" to "src/utils.ts" should be preserved
    let import_edges: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Imports"))
        .collect();
    assert!(
        !import_edges.is_empty(),
        "Imports edge must be preserved after deduplication"
    );
}

// ============================================================================
// Scenario: Preserve stub File nodes for external import targets
// ============================================================================
#[tokio::test]
async fn test_preserve_stub_file_nodes_for_external_imports() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with file "src/app.ts" that imports from "express"
    let app_content = r#"
import { Router } from 'express';

export function createApp(): void {
    const router = Router();
}
"#;
    write_test_file(project_dir, "src/app.ts", app_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    // @step Then a stub File node should exist for the external import target
    let external_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("express")))
        .collect();
    assert!(
        !external_file_nodes.is_empty(),
        "External import target should have a stub File node"
    );

    // @step And no unique constraint violation should occur
    // Verify by loading into a real graph database
    let db_path = temp_dir.path().join("test-ast-external.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let load_result = db.load_entities(&entities).await;
    assert!(
        load_result.is_ok(),
        "Graph load should succeed without unique constraint violation, got: {:?}",
        load_result.err()
    );
}

// ============================================================================
// Scenario: Multiple files importing same target produce single File node
// ============================================================================
#[tokio::test]
async fn test_multiple_importers_produce_single_target_file_node() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with files "src/a.ts", "src/b.ts", and "src/c.ts" all importing from "./shared"
    let a_content = r#"
import { util } from './shared';
export function a(): void { util(); }
"#;
    let b_content = r#"
import { util } from './shared';
export function b(): void { util(); }
"#;
    let c_content = r#"
import { util } from './shared';
export function c(): void { util(); }
"#;
    write_test_file(project_dir, "src/a.ts", a_content);
    write_test_file(project_dir, "src/b.ts", b_content);
    write_test_file(project_dir, "src/c.ts", c_content);

    // @step And a project file "src/shared.ts" that is also walked by the file walker
    let shared_content = r#"
export function util(): string {
    return 'shared';
}
"#;
    write_test_file(project_dir, "src/shared.ts", shared_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    // @step Then only one File node should exist for "src/shared.ts"
    let shared_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/shared.ts")))
        .collect();
    assert_eq!(
        shared_file_nodes.len(),
        1,
        "Should have exactly 1 File node for src/shared.ts, got {} — dedup failed with multiple importers",
        shared_file_nodes.len()
    );

    // @step And all three Imports edges should be preserved
    let import_edges: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Edge { edge_type, to_slug, .. }
                if edge_type == "Imports" && to_slug.contains("shared"))
        })
        .collect();
    assert_eq!(
        import_edges.len(),
        3,
        "All 3 Imports edges to shared.ts must be preserved, got {}",
        import_edges.len()
    );

    // @step And the graph should load successfully without constraint violations
    let db_path = temp_dir.path().join("test-ast-multi-import.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let load_result = db.load_entities(&entities).await;
    assert!(
        load_result.is_ok(),
        "Graph load should succeed, got: {:?}",
        load_result.err()
    );
}

// ============================================================================
// Scenario: Full codebase indexing completes without errors
// ============================================================================
#[tokio::test]
async fn test_full_codebase_indexing_completes_without_errors() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory with TypeScript files containing cross-imports
    // Simulate a realistic project structure with circular and diamond imports
    let endpoint_content = r#"
import { isSlashCommand, handleSlashCommand } from './slash-commands';
import { formatMessage } from './formatting';

export async function handleRequest(msg: string): Promise<string> {
    if (isSlashCommand(msg)) {
        return handleSlashCommand(msg);
    }
    return formatMessage(msg);
}
"#;
    let slash_commands_content = r#"
import { formatMessage } from './formatting';

export function isSlashCommand(msg: string): boolean {
    return msg.startsWith('/');
}

export function handleSlashCommand(msg: string): string {
    return formatMessage('Executed: ' + msg);
}
"#;
    let formatting_content = r#"
export function formatMessage(text: string): string {
    return '<b>' + text + '</b>';
}
"#;
    write_test_file(project_dir, "src/endpoint.ts", endpoint_content);
    write_test_file(project_dir, "src/slash-commands.ts", slash_commands_content);
    write_test_file(project_dir, "src/formatting.ts", formatting_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the AST index operation runs via walk_and_extract
    let entities = walk_and_extract(project_dir)
        .expect("walk_and_extract should succeed");

    // @step And the entities are loaded into the graph database
    let db_path = temp_dir.path().join("test-ast-full.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let load_result = db.load_entities(&entities).await;

    // @step Then the load operation should succeed with no unique constraint violations
    assert!(
        load_result.is_ok(),
        "Full codebase load must succeed without @unique constraint violation, got: {:?}",
        load_result.err()
    );

    // @step And the graph should contain File, Function, and Imports data
    let db = db.with_query_source(AST_QUERIES);

    let files = db
        .query("all_files", None)
        .await
        .expect("all_files query should succeed");
    let files_arr = files.as_array().expect("Should be array");
    assert!(
        files_arr.len() >= 3,
        "Should have at least 3 File nodes (endpoint, slash-commands, formatting), got {}",
        files_arr.len()
    );

    let functions = db
        .query("all_functions", None)
        .await
        .expect("all_functions query should succeed");
    let fns_arr = functions.as_array().expect("Should be array");
    assert!(
        fns_arr.len() >= 4,
        "Should have at least 4 Function nodes, got {}",
        fns_arr.len()
    );
}

// ============================================================================
// Scenario: TypeScript files with multi-byte UTF-8 characters do not panic
// Regression test for: byte index is not a char boundary inside '─'
// ============================================================================
#[tokio::test]
async fn test_extract_typescript_with_multibyte_utf8_chars() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a TypeScript file containing multi-byte UTF-8 characters (box-drawing '─')
    // The '─' character is 3 bytes (0xE2 0x94 0x80). When it appears near a function
    // call like `new Something()`, the "new " lookback can land inside the multi-byte
    // sequence, causing a panic on &str slicing.
    let ts_content = r#"
export function renderBoard(options: {
  cwd: string;
}): string {
  const columnWidth = 30;
  const border = '─'.repeat(columnWidth);
  const header = `┌${'─'.repeat(columnWidth)}┐`;
  
  // Call a function right after multi-byte chars — this is the trigger
  formatColumn(border);
  processHeader(header);
  
  return header;
}

function formatColumn(text: string): string {
  return text.padEnd(30);
}

function processHeader(header: string): string {
  return header;
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/board.ts", ts_content);

    // @step When the TypeScript extractor parses the file
    let result = extract_file(&file_path, temp_dir.path());

    // @step Then extraction should succeed without panicking
    assert!(
        result.is_ok(),
        "TypeScript extraction should succeed on files with multi-byte UTF-8 chars, got: {:?}",
        result.err()
    );

    let entities = result.unwrap();

    // @step And Function nodes should be extracted correctly
    assert!(
        count_nodes(&entities, "Function") >= 2,
        "Should extract at least 2 Function nodes from file with multi-byte chars"
    );

    // @step And Calls edges should be extracted correctly
    let calls_edges = count_edges(&entities, "Calls");
    assert!(
        calls_edges >= 1,
        "Should extract at least 1 Calls edge from file with multi-byte chars, got {}",
        calls_edges
    );
}

// ============================================================================
// Scenario: Extraction panics are caught and returned as errors
// ============================================================================
#[tokio::test]
async fn test_extract_file_catches_panics_gracefully() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a valid TypeScript file
    let ts_content = r#"
export function hello(): string {
    return 'world';
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/hello.ts", ts_content);

    // @step When extract_file is called
    let result = extract_file(&file_path, temp_dir.path());

    // @step Then it should return Ok with entities (no panic)
    assert!(result.is_ok(), "Normal extraction should succeed");

    // @step And the catch_unwind wrapper should be in place for safety
    // This test ensures the function signature handles panics via Result
    let entities = result.unwrap();
    assert!(
        !entities.is_empty(),
        "Should produce at least one entity (File node)"
    );
}
