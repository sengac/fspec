// Feature: spec/features/cyclomatic-complexity-analysis.feature
use super::*;

// ============================================================================
// Scenario: Find top N most complex functions in a codebase
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_find_top_n_most_complex_functions() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    // @step Given I have a codebase indexed with cyclomatic complexity calculated for all functions
    // (project indexed above: 5 functions of varying complexity)

    // @step When I request ast_complexity with limit 10
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,     // no specific function — top-N mode
        Some(10), // limit
        None,     // no min_threshold
        None,     // no path filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive a list of functions sorted by complexity descending
    assert_eq!(result["action"], "ast_complexity");
    let results = result["results"].as_array().expect("results array");
    // We have 5 functions, so we get all 5 (limit 10 > 5)
    assert_eq!(results.len(), 5);
    // Verify descending order
    assert_eq!(results[0]["name"], "complex_handler");
    assert_eq!(results[0]["cyclomaticComplexity"], 12);
    assert_eq!(results[1]["name"], "process_data");
    assert_eq!(results[1]["cyclomaticComplexity"], 8);
    assert_eq!(results[2]["name"], "validate_input");
    assert_eq!(results[2]["cyclomaticComplexity"], 6);

    // @step And each result should include function name, file path, line numbers, and complexity score
    let first = &results[0];
    assert!(first.get("name").is_some());
    assert!(first.get("path").is_some());
    assert!(first.get("lineStart").is_some());
    assert!(first.get("lineEnd").is_some());
    assert!(first.get("cyclomaticComplexity").is_some());
    Ok(())
}

// ============================================================================
// Scenario: Find top N with limit smaller than total
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_find_top_n_with_limit() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(3), // only top 3
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["cyclomaticComplexity"], 12);
    assert_eq!(results[1]["cyclomaticComplexity"], 8);
    assert_eq!(results[2]["cyclomaticComplexity"], 6);
    Ok(())
}

// ============================================================================
// Scenario: Find top N with min_threshold filter
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_find_top_n_with_threshold() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(20),
        Some(5), // only complexity >= 5
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    assert_eq!(results.len(), 3); // complex_handler(12), process_data(8), validate_input(6)
    for r in results {
        assert!(r["cyclomaticComplexity"].as_u64().unwrap() >= 5);
    }
    Ok(())
}

// ============================================================================
// Scenario: Query complexity of a specific function
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_query_specific_function_complexity() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    // @step Given I have a codebase indexed with cyclomatic complexity calculated
    // (project indexed above)

    // @step When I request ast_complexity for a specific function slug
    let process_data = slug_of(
        &entities(&root).await?,
        "Function",
        "src/handler.ts::process_data",
    )?;
    let result_str =
        ast_complexity::dispatch_ast_complexity(&db, Some(&process_data), None, None, None).await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive that function's cyclomatic complexity score
    assert_eq!(result["action"], "ast_complexity");
    assert_eq!(result["cyclomaticComplexity"], 8);

    // @step And the response should include the function name, file path, and line numbers
    assert_eq!(result["name"], "process_data");
    assert!(result.get("path").is_some());
    assert!(result.get("lineStart").is_some());
    assert!(result.get("lineEnd").is_some());
    Ok(())
}

// ============================================================================
// Scenario: Simple function has complexity 1
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_function_complexity_one() {
    // @step Given I have indexed a function with no branches or decision points
    let source = "function getter() { return 42; }";

    // @step When I query its cyclomatic complexity
    let result = complexity_of("src/getter.ts", source, "getter");

    // @step Then the complexity score should be 1
    assert_eq!(result, 1);
}

// ============================================================================
// Scenario: Function with multiple branches has correct complexity
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_function_with_branches_correct_complexity() {
    // @step Given I have indexed a function with 5 if/else branches
    let source = r#"function classify(x) {
        if (x > 100) {
            return "huge";
        } else if (x > 50) {
            return "big";
        } else if (x > 20) {
            return "medium";
        } else if (x > 5) {
            return "small";
        } else if (x > 0) {
            return "tiny";
        }
        return "zero";
    }"#;

    // @step When I query its cyclomatic complexity
    let result = complexity_of("src/classify.ts", source, "classify");

    // @step Then the complexity score should be 6
    assert_eq!(result, 6, "1 base + 5 if/else if branches = 6");
}

// ============================================================================
// Scenario: Complexity is populated during ast_index
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_complexity_populated_during_extraction() -> TestResult {
    // @step Given I have a codebase that has not been indexed
    let source = r#"
export function handleRequest(req: Request): Response {
    if (req.method === "GET") {
        return getHandler(req);
    } else if (req.method === "POST") {
        return postHandler(req);
    }
    for (const header of req.headers) {
        if (header.key === "auth") {
            return unauthorized();
        }
    }
    return notFound();
}

export function simpleReturn(): string {
    return "hello";
}
"#;
    let (_directory, root) = project(&[("src/server.ts", source)])?;

    // @step When I run ast_index on the codebase
    index(&root).await?;

    // @step Then the Function nodes in the graph should have cyclomaticComplexity values
    let graph = entities(&root).await?;
    let functions = code_graph_fixture::nodes(&graph, "Function");
    assert_eq!(functions.len(), 2, "two functions: {functions:?}");
    for (slug, properties) in &functions {
        assert!(
            properties.get("cyclomaticComplexity").is_some(),
            "{slug} has no cyclomaticComplexity"
        );
    }
    let complexity = |qualified_name: &str| {
        functions
            .iter()
            .find(|(_, p)| p.get("qualifiedName").and_then(Value::as_str) == Some(qualified_name))
            .and_then(|(_, p)| p.get("cyclomaticComplexity").and_then(Value::as_i64))
    };

    // @step And functions with decision points should have complexity greater than 1
    // 1 + if + else-if (if_statement) + for-of (for_in_statement) + if = 5
    assert_eq!(complexity("src/server.ts::handleRequest"), Some(5));
    assert_eq!(complexity("src/server.ts::simpleReturn"), Some(1));
    Ok(())
}
