// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
use super::*;

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
    let entities = walk_and_extract(project_dir, true);

    // @step Then only one File node should exist for "src/shared.ts"
    let shared_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/shared.ts"))
        })
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
    let entities = walk_and_extract(project_dir, true);

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
    let result = extract_path(&file_path, "src/board.ts");

    // @step Then extraction should succeed without panicking
    assert!(
        result.is_ok(),
        "TypeScript extraction should succeed on files with multi-byte UTF-8 chars, got: {:?}",
        result.err()
    );

    let facts = result.unwrap();
    assert_eq!(
        facts.definitions.len(),
        3,
        "The file's facts should hold its 3 functions: {:?}",
        facts.definitions
    );
    let entities = walk_and_extract(temp_dir.path(), false);

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
    let result = extract_path(&file_path, "src/hello.ts");

    // @step Then it should return Ok with entities (no panic)
    assert!(result.is_ok(), "Normal extraction should succeed");
    let facts = result.unwrap();
    assert_eq!(facts.path, "src/hello.ts");
    assert_eq!(facts.definitions.len(), 1, "{:?}", facts.definitions);

    // @step And the catch_unwind wrapper should be in place for safety
    // A file that cannot be extracted is a failed file of the index (with
    // its error), never a panic of the index.
    std::fs::write(temp_dir.path().join("src/broken.ts"), [0xff, 0xfe, 0x00])
        .expect("write non-UTF-8 file");
    let failed = extract_path(&temp_dir.path().join("src/broken.ts"), "src/broken.ts")
        .expect_err("non-UTF-8 file should fail");
    assert_eq!(failed.path, "src/broken.ts");
    let indexed = index_tree(temp_dir.path(), false, 1);
    assert_eq!(indexed.files_parsed, 1, "hello.ts is still indexed");
    assert_eq!(
        indexed
            .failed
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        ["src/broken.ts"]
    );
    assert!(
        !indexed.entities.is_empty(),
        "Should produce at least one entity (File node)"
    );
}
