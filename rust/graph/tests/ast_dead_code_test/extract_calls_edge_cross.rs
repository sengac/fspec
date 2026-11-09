// Feature: spec/features/ast-dead-code-detection.feature
use super::*;

// ============================================================================
// Scenario: Extract Calls edge for cross-file function call via import
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_extract_calls_edge_cross_file_via_import() -> TestResult {
    // @step Given a TypeScript file "src/app.ts" with content:
    let app_content = r#"
import { validateConfig } from './config';
function main() { validateConfig(); }
"#;

    // @step And a TypeScript file "src/config.ts" with content:
    let config_content = r#"
export function validateConfig() { return true; }
"#;

    // @step When the TS extractor processes both files
    let (_directory, _root, graph) = indexed(&[
        ("src/app.ts", app_content),
        ("src/config.ts", config_content),
    ])
    .await?;

    // @step Then a Calls edge should exist from "src-app-ts::main" to "src-config-ts::validateConfig"
    assert!(
        has_edge(
            &graph,
            "Calls",
            ("Function", "src/app.ts::main"),
            ("Function", "src/config.ts::validateConfig"),
        )?,
        "Should have a Calls edge from app::main to config::validateConfig, got: {:?}",
        edges(&graph, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edge for same-file function call
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_extract_calls_edge_same_file() -> TestResult {
    // @step Given a TypeScript file "src/utils.ts" with content:
    let utils_content = r#"
function foo() { bar(); }
function bar() { return 1; }
"#;

    // @step When the TS extractor processes the file
    let (_directory, _root, graph) = indexed(&[("src/utils.ts", utils_content)]).await?;

    // @step Then a Calls edge should exist from "src-utils-ts::foo" to "src-utils-ts::bar"
    assert!(
        has_edge(
            &graph,
            "Calls",
            ("Function", "src/utils.ts::foo"),
            ("Function", "src/utils.ts::bar"),
        )?,
        "Should have a Calls edge from foo to bar (same file), got: {:?}",
        edges(&graph, "Calls")
    );
    Ok(())
}

// ============================================================================
// Scenario: Ignore method calls and builtins — no Calls edges for dotted expressions
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ignore_method_calls_and_builtins() -> TestResult {
    // @step Given a TypeScript file "src/main.ts" with content:
    let main_content = r#"
function run() {
  console.log('hello');
  process.exit(0);
  obj.method();
}
"#;

    // @step When the TS extractor processes the file
    let (_directory, _root, graph) = indexed(&[("src/main.ts", main_content)]).await?;

    // @step Then no Calls edges should be emitted
    let calls = edges(&graph, "Calls");
    assert!(
        calls.is_empty(),
        "Should have 0 Calls edges (console.log, process.exit, obj.method are not tracked), got {calls:?}"
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract TypeRef edges from function parameter and return types
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_extract_typeref_edges_from_function_signatures() -> TestResult {
    // @step Given a TypeScript file "src/handler.ts" with content:
    let handler_content = r#"
interface Request { url: string; }
interface Response { status: number; }
function handler(req: Request): Response { return { status: 200 }; }
"#;

    // @step When the TS extractor processes the file
    let (_directory, _root, graph) = indexed(&[("src/handler.ts", handler_content)]).await?;

    // @step Then a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Request"
    assert!(
        has_edge(
            &graph,
            "TypeRef",
            ("Function", "src/handler.ts::handler"),
            ("Type", "src/handler.ts::Request"),
        )?,
        "Should have a TypeRef edge from handler to Request, got: {:?}",
        edges(&graph, "TypeRef")
    );

    // @step And a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Response"
    assert!(
        has_edge(
            &graph,
            "TypeRef",
            ("Function", "src/handler.ts::handler"),
            ("Type", "src/handler.ts::Response"),
        )?,
        "Should have a TypeRef edge from handler to Response, got: {:?}",
        edges(&graph, "TypeRef")
    );
    Ok(())
}

// ============================================================================
// Scenario: Detect orphan files with no incoming Imports edges
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_detect_orphan_files_no_imports() -> TestResult {
    // @step Given a graph with File "src/used.ts" imported by "src/app.ts"
    let app_content = r#"
import { helper } from './used';
export function main() { helper(); }
"#;
    let used_content = r#"
export function helper() { return 1; }
"#;

    // @step And a graph with File "src/orphan.ts" imported by no other file
    let orphan_content = r#"
export function orphanFn() { return 'nobody calls me'; }
"#;
    let (_directory, root, _graph) = indexed(&[
        ("src/app.ts", app_content),
        ("src/used.ts", used_content),
        ("src/orphan.ts", orphan_content),
    ])
    .await?;

    // @step When the ast_dead_code action runs with entity_type "File"
    let orphan_paths = dead_code(&root, "File", "path").await?;

    // @step Then the result should include "src/orphan.ts"
    assert!(
        orphan_paths.contains(&"src/orphan.ts".to_string()),
        "Orphan files should include src/orphan.ts, got: {orphan_paths:?}"
    );

    // @step And the result should not include "src/used.ts"
    assert!(
        !orphan_paths.contains(&"src/used.ts".to_string()),
        "Orphan files should NOT include src/used.ts (it is imported), got: {orphan_paths:?}"
    );
    Ok(())
}
