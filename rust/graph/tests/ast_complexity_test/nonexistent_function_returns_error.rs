// Feature: spec/features/cyclomatic-complexity-analysis.feature
use super::*;

// ============================================================================
// Scenario: Non-existent function returns error
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_nonexistent_function_returns_error() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    // @step Given I have a codebase indexed in the AST graph
    // (project indexed above)

    // @step When I request ast_complexity for a non-existent function slug
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        Some("nonexistent::function_xyz"),
        None,
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    // @step Then I should receive an error indicating the function was not found
    assert_eq!(result["action"], "ast_complexity");
    assert!(
        result.get("error").is_some(),
        "Should have error field for non-existent function"
    );
    let error_msg = result["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("not found") || error_msg.contains("No function"),
        "Error should mention function not found, got: {error_msg}"
    );
    Ok(())
}

#[test]
fn test_complexity_python() {
    let source = r#"
def process(data):
    if data is None:
        return None
    for item in data:
        if item > 0 and item < 100:
            yield item
        elif item < 0:
            raise ValueError("negative")
    while len(data) > 0:
        data.pop()
"#;
    // 1 base + if + for + if + and + elif + while = 7
    let result = complexity_of("src/process.py", source, "process");
    assert_eq!(
        result, 7,
        "Python: 1 + if + for + (if + and) + elif + while"
    );
}

#[test]
fn test_complexity_rust() {
    let source = r#"
fn dispatch(action: Action) -> Result<()> {
    if action.is_valid() {
        for item in action.items() {
            match item.kind {
                Kind::A => handle_a(),
                Kind::B => handle_b(),
                Kind::C => handle_c(),
            }
        }
    }
    while let Some(pending) = queue.pop() {
        if pending.ready() && pending.valid() {
            process(pending);
        }
    }
    Ok(())
}
"#;
    // 1 base + if + for + 3 match arms (=>) + while + if + && = 9
    let result = complexity_of("src/lib.rs", source, "dispatch");
    assert_eq!(
        result, 9,
        "Rust: 1 + if + for + 3 match arms + while + if + &&"
    );
}

#[test]
fn test_complexity_go() {
    let source = r#"
func handleRequest(w http.ResponseWriter, r *http.Request) {
    if r.Method == "GET" {
        getHandler(w, r)
    }
    for _, header := range r.Header {
        if header == "auth" || header == "token" {
            authenticate(w, r)
        }
    }
    switch r.URL.Path {
    case "/api":
        apiHandler(w, r)
    case "/health":
        healthHandler(w, r)
    }
}
"#;
    // 1 base + if + for + if + || + 2 cases = 7
    let result = complexity_of("server/handler.go", source, "handleRequest");
    assert_eq!(result, 7, "Go: 1 + if + for + if + || + 2 cases");
}

#[test]
fn test_complexity_ruby() {
    let source = r#"
def process(input)
    if input.nil?
        return nil
    end
    unless input.empty?
        input.each do |item|
            case item.type
            when :a
                handle_a
            when :b
                handle_b
            end
            rescue StandardError => e
                log_error(e)
        end
    end
end
"#;
    // 1 base + if + unless + 2 when + rescue = 6 (a block passed to `each`
    // is a call, not a loop node: the Ruby table declares no decision for it)
    let result = complexity_of("lib/process.rb", source, "process");
    assert_eq!(result, 6, "Ruby: 1 + if + unless + 2 when + rescue");
}

#[test]
fn test_complexity_empty_function() {
    let source = "function noop() {}";
    let result = complexity_of("src/noop.ts", source, "noop");
    assert_eq!(result, 1, "Empty function should have complexity 1");
}

#[test]
fn test_complexity_operators_only() {
    let source = r#"
function check(a, b, c, d) {
    return a && b || c && d;
}
"#;
    // 1 base + && + || + && = 4
    let result = complexity_of("src/check.ts", source, "check");
    assert_eq!(result, 4, "TS: 1 + 3 logical operators");
}

// ============================================================================
// Path-filtered top-N query
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_find_top_n_with_path_filter() -> TestResult {
    let (_directory, root) = setup_complexity_project().await?;
    let db = registry::read_code_graph(&root).await?;

    // Filter to only src/handler.ts functions
    let result_str = ast_complexity::dispatch_ast_complexity(
        &db,
        None,
        Some(20),
        None,
        Some("src/handler"), // path glob filter
    )
    .await;
    let result: Value = serde_json::from_str(&result_str).expect("valid JSON");

    let results = result["results"].as_array().expect("results array");
    // Only 3 functions in src/handler.ts
    assert_eq!(results.len(), 3);
    for r in results {
        let path = r["path"].as_str().unwrap_or("");
        assert!(
            path.contains("handler"),
            "All results should be from handler path, got: {path}"
        );
    }
    Ok(())
}
