// Feature: spec/features/ast-dead-code-detection.feature
use super::*;

// ============================================================================
// Scenario: Detect uncalled functions with no incoming Calls edges
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_detect_uncalled_functions_no_calls() -> TestResult {
    // @step Given a graph with Function "app::main" that calls "app::helper"
    // @step And a graph with Function "app::unused" that is never called
    let app_content = r#"
function main() { helper(); }
function helper() { return 1; }
function unused() { return 'nobody calls me'; }
"#;
    let (_directory, root, _graph) = indexed(&[("src/app.ts", app_content)]).await?;

    // @step When the ast_dead_code action runs with entity_type "Function"
    let uncalled = dead_code(&root, "Function", "qualifiedName").await?;

    // @step Then the result should include "app::unused"
    assert!(
        uncalled.contains(&"src/app.ts::unused".to_string()),
        "Uncalled functions should include unused, got: {uncalled:?}"
    );

    // @step And the result should not include "app::helper"
    assert!(
        !uncalled.contains(&"src/app.ts::helper".to_string()),
        "Uncalled functions should NOT include helper (it is called by main), got: {uncalled:?}"
    );
    Ok(())
}

// ============================================================================
// Scenario: Detect unreferenced types with no incoming TypeRef edges
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_detect_unreferenced_types_no_typerefs() -> TestResult {
    // @step Given a graph with Type "handler-ts::Request" referenced by "handler-ts::handler"
    // @step And a graph with Type "handler-ts::OldInterface" referenced by no function
    let handler_content = r#"
interface Request { url: string; }
interface Response { status: number; }
interface OldInterface { legacy: boolean; }
function handler(req: Request): Response { return { status: 200 }; }
"#;
    let (_directory, root, _graph) = indexed(&[("src/handler.ts", handler_content)]).await?;

    // @step When the ast_dead_code action runs with entity_type "Type"
    let unreferenced = dead_code(&root, "Type", "qualifiedName").await?;

    // @step Then the result should include "handler-ts::OldInterface"
    assert!(
        unreferenced.contains(&"src/handler.ts::OldInterface".to_string()),
        "Unreferenced types should include OldInterface, got: {unreferenced:?}"
    );

    // @step And the result should not include "handler-ts::Request"
    assert!(
        !unreferenced.contains(&"src/handler.ts::Request".to_string()),
        "Unreferenced types should NOT include Request (it is referenced by handler), got: {unreferenced:?}"
    );
    Ok(())
}

// ============================================================================
// Scenario: Exclude test files from dead code results by default
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exclude_test_files_from_dead_code_results() -> TestResult {
    // @step Given a graph with File "src/__tests__/app.test.ts" that is a test file
    let test_content = r#"
export function testMain() { return true; }
"#;
    // A non-test source file for contrast.
    let src_content = r#"
export function realCode() { return 1; }
"#;

    // @step And that test file is never imported by any other file
    let (_directory, root, _graph) = indexed(&[
        ("src/__tests__/app.test.ts", test_content),
        ("src/real.ts", src_content),
    ])
    .await?;

    // @step When the ast_dead_code action runs with entity_type "File"
    let orphans = dead_code(&root, "File", "path").await?;

    // @step Then the result should not include "src/__tests__/app.test.ts"
    assert_eq!(
        orphans,
        vec!["src/real.ts".to_string()],
        "Dead code results should hold the orphan source file and no test file"
    );
    Ok(())
}
