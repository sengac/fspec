//! KGRAPH-077: code graph query layer performance and determinism.
//!
//! Feature: spec/features/code-graph-query-layer-performance-and-determinism.feature
//!
//! Every scenario indexes a temp project with `ast_index` and queries its
//! graph through the dispatch functions the GraphSearch handler calls.
//! Query counts come from `GraphDatabase::queries_run`; nothing depends on
//! timing or randomness (TEST-042).

use std::path::Path;

use codelet_graph::ast_call_chain::dispatch_ast_call_chain;
use codelet_graph::ast_complexity::dispatch_ast_complexity;
use codelet_graph::ast_dead_code::dispatch_ast_dead_code;
use codelet_graph::ast_dispatch::{
    dispatch_ast_neighbors, dispatch_ast_search, dispatch_ast_stats,
};
use codelet_graph::ast_hierarchy::dispatch_ast_hierarchy;
use codelet_graph::ast_transitive::dispatch_ast_callers;
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{entities, index, project, slug_of};

fn parse(output: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(output)?)
}

/// `output` parsed, failing when it is an error.
fn ok(output: &str) -> Result<Value, TestError> {
    let value = parse(output)?;
    if let Some(error) = value.get("error") {
        return Err(format!("unexpected error: {error}").into());
    }
    Ok(value)
}

fn number(value: &Value, field: &str) -> Result<u64, TestError> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("no {field} in {value}").into())
}

fn names(rows: &Value) -> Result<Vec<String>, TestError> {
    rows.as_array()
        .ok_or_else(|| format!("not an array: {rows}"))?
        .iter()
        .map(|row| {
            row["name"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("no name in {row}").into())
        })
        .collect()
}

/// The slug of the function `name` of module `m.py`.
async fn function(root: &Path, name: &str) -> Result<String, TestError> {
    slug_of(&entities(root).await?, "Function", &format!("m.py::{name}"))
}

/// A module whose functions each call `target`.
fn callers_module(callers: &[String], others: usize) -> String {
    let mut text = String::from("def target():\n    return 0\n\n");
    for caller in callers {
        text.push_str(&format!("def {caller}():\n    return target()\n\n"));
    }
    for other in 0..others {
        text.push_str(&format!("def other_{other:02}():\n    return {other}\n\n"));
    }
    text
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callers_come_back_in_depth_and_slug_order_and_a_limit_keeps_the_first_of_that_order(
) -> TestResult {
    // @step Given a Python module that defines target and 12 direct callers c11 down to c00 in reverse order has been indexed
    let callers: Vec<String> = (0..12).rev().map(|i| format!("c{i:02}")).collect();
    let (_directory, root) = project(&[("m.py", &callers_module(&callers, 0))])?;
    index(&root).await?;
    let target = function(&root, "target").await?;

    // @step When ast_callers runs for target with limit 5
    let first = {
        let graph = registry::read_code_graph(&root).await?;
        dispatch_ast_callers(&graph, &target, None, Some(5)).await
    };
    let result = ok(&first)?;

    // @step Then the results are c00, c01, c02, c03 and c04 at depth 1
    assert_eq!(
        names(&result["results"])?,
        vec!["c00", "c01", "c02", "c03", "c04"]
    );
    for row in result["results"].as_array().into_iter().flatten() {
        assert_eq!(row["depth"], 1, "{row}");
    }

    // @step And the result has count 5 and total 12
    assert_eq!(number(&result, "count")?, 5);
    assert_eq!(number(&result, "total")?, 12);

    // @step And after the graph is opened again the same request returns byte-identical output
    registry::reset_all_graphs();
    let graph = registry::read_code_graph(&root).await?;
    let second = dispatch_ast_callers(&graph, &target, None, Some(5)).await;
    assert_eq!(first, second);
    Ok(())
}

/// The queries one ast_callers of `target` runs on `root`'s graph, and its result.
async fn counted_callers(root: &Path, target: &str) -> Result<(u64, Value), TestError> {
    let graph = registry::read_code_graph(root).await?;
    let before = graph.queries_run();
    let result = ok(&dispatch_ast_callers(&graph, target, None, None).await)?;
    Ok((graph.queries_run() - before, result))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_query_index_is_loaded_once_per_graph_version_with_a_fixed_number_of_queries(
) -> TestResult {
    // @step Given a project of 3 functions and a project of 60 functions, each with one caller of target, have been indexed
    let caller = vec!["caller".to_string()];
    let (_small_directory, small) = project(&[("m.py", &callers_module(&caller, 1))])?;
    let (_large_directory, large) = project(&[("m.py", &callers_module(&caller, 58))])?;
    index(&small).await?;
    index(&large).await?;
    let small_target = function(&small, "target").await?;
    let large_target = function(&large, "target").await?;

    // @step When ast_callers runs for target in each project
    let (small_queries, small_result) = counted_callers(&small, &small_target).await?;
    let (large_queries, large_result) = counted_callers(&large, &large_target).await?;
    assert_eq!(names(&small_result["results"])?, vec!["caller"]);
    assert_eq!(names(&large_result["results"])?, vec!["caller"]);

    // @step Then both runs execute the same number of queries
    assert_eq!(small_queries, large_queries);

    // @step And a second ast_callers on the same graph version loads no index
    let (again, _) = counted_callers(&large, &large_target).await?;
    assert!(
        again < large_queries,
        "the second run ran {again} queries, the first {large_queries}"
    );

    // @step And after ast_index writes a new caller of target the next ast_callers returns it
    let two = vec!["caller".to_string(), "second_caller".to_string()];
    std::fs::write(large.join("m.py"), callers_module(&two, 58))?;
    index(&large).await?;
    let target = function(&large, "target").await?;
    let (_, result) = counted_callers(&large, &target).await?;
    assert_eq!(names(&result["results"])?, vec!["caller", "second_caller"]);
    Ok(())
}

/// The function names of each chain of an ast_call_chain result.
fn chains(result: &Value) -> Result<Vec<Vec<String>>, TestError> {
    result["chains"]
        .as_array()
        .ok_or_else(|| format!("no chains in {result}"))?
        .iter()
        .map(|chain| names(&chain["function_chain"]))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn call_chains_are_shortest_paths_to_a_caller_of_the_target_shortest_first() -> TestResult {
    // @step Given an indexed project where A calls B and C, B calls D, C calls E and E calls D
    let module = "def a():\n    b()\n    c()\n\ndef b():\n    d()\n\ndef c():\n    e()\n\n\
                  def d():\n    return 0\n\ndef e():\n    d()\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;
    let from = function(&root, "a").await?;
    let to = function(&root, "d").await?;

    // @step When ast_call_chain runs from A to D
    let graph = registry::read_code_graph(&root).await?;
    let result = ok(&dispatch_ast_call_chain(&graph, &from, &to, None).await)?;

    // @step Then the chains are A, B, D and then A, C, E, D
    assert_eq!(
        chains(&result)?,
        vec![vec!["a", "b", "d"], vec!["a", "c", "e", "d"]]
    );
    assert_eq!(result["truncated"], false, "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn call_chains_in_a_graph_with_thousands_of_shortest_paths_are_capped_at_20_in_slug_order(
) -> TestResult {
    // @step Given an indexed call graph of 12 layers of 2 functions below a top function and above a bottom function, each function calling both functions of the next layer
    let layer = |k: usize, side: &str| format!("l{k:02}_{side}");
    let mut module = String::from("def top():\n    l01_a()\n    l01_b()\n\n");
    for k in 1..=12 {
        for side in ["a", "b"] {
            let body = if k == 12 {
                "    bottom()\n".to_string()
            } else {
                format!("    {}()\n    {}()\n", layer(k + 1, "a"), layer(k + 1, "b"))
            };
            module.push_str(&format!("def {}():\n{body}\n", layer(k, side)));
        }
    }
    module.push_str("def bottom():\n    return 0\n");
    let (_directory, root) = project(&[("m.py", &module)])?;
    index(&root).await?;
    let from = function(&root, "top").await?;
    let to = function(&root, "bottom").await?;

    // @step When ast_call_chain runs from the top function to the bottom function with max_depth 13
    let graph = registry::read_code_graph(&root).await?;
    let result = ok(&dispatch_ast_call_chain(&graph, &from, &to, Some(13)).await)?;

    // @step Then it returns 20 chains of length 13 in the order of their slug sequences
    let expected: Vec<Vec<String>> = (0..20usize)
        .map(|i| {
            let mut chain = vec!["top".to_string()];
            for k in 1..=12 {
                let side = if i >> (12 - k) & 1 == 1 { "b" } else { "a" };
                chain.push(layer(k, side));
            }
            chain.push("bottom".to_string());
            chain
        })
        .collect();
    assert_eq!(chains(&result)?, expected);
    for chain in result["chains"].as_array().into_iter().flatten() {
        assert_eq!(chain["chain_length"], 13);
    }

    // @step And the result reports truncated true
    assert_eq!(result["truncated"], true, "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_name_search_reads_no_source_and_returns_the_source_of_its_result_rows() -> TestResult {
    // @step Given an indexed project with 5 functions whose names contain helper
    let module = "def zeta():\n    return 0\n\ndef helper_5():\n    return 5\n\n\
                  def helper_3():\n    return 3\n\ndef helper_1():\n    return 1\n\n\
                  def helper_4():\n    return 4\n\ndef helper_2():\n    return 2\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;

    // @step When ast_search runs for helper in name mode with limit 2
    let graph = registry::read_code_graph(&root).await?;
    let output = dispatch_ast_search(
        &graph,
        "helper",
        Some("Function"),
        Some(2),
        None,
        Some("name"),
        None,
        None,
    )
    .await;
    let result = ok(&output)?;

    // @step Then it returns the 2 functions with the lowest slugs, each with its source
    assert_eq!(names(&result["results"])?, vec!["helper_1", "helper_2"]);
    for (row, name) in result["results"]
        .as_array()
        .into_iter()
        .flatten()
        .zip(["helper_1", "helper_2"])
    {
        let source = row["source"].as_str().unwrap_or_default();
        assert!(source.starts_with(&format!("def {name}()")), "{row}");
    }

    // @step And the result has count 2 and total 5
    assert_eq!(number(&result, "count")?, 2);
    assert_eq!(number(&result, "total")?, 5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_invalid_path_glob_is_an_error() -> TestResult {
    // @step Given an indexed project
    let (_directory, root) = project(&[("m.py", "def helper():\n    return 0\n")])?;
    index(&root).await?;

    // @step When ast_search runs with the path glob "["
    let graph = registry::read_code_graph(&root).await?;
    let output =
        dispatch_ast_search(&graph, "helper", None, None, Some("["), None, None, None).await;

    // @step Then the result is an error naming the invalid glob
    let result = parse(&output)?;
    let error = result["error"].as_str().unwrap_or_default();
    assert!(error.contains("glob") && error.contains('['), "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_query_is_an_error_never_empty_results() -> TestResult {
    // @step Given an indexed project whose graph is open
    let module = "class Shape:\n    def area(self):\n        return 0\n\n\
                  def helper():\n    return 0\n\ndef main():\n    return helper()\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;
    let helper = function(&root, "helper").await?;
    let shape = slug_of(&entities(&root).await?, "Type", "m.py::Shape")?;
    registry::reset_all_graphs();
    let graph = registry::read_code_graph(&root).await?;

    // @step When the graph's datasets are removed from disk
    for entry in std::fs::read_dir(registry::code_graph_path(&root))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        }
    }

    // @step Then ast_callers, ast_search, ast_neighbors, ast_hierarchy, ast_dead_code and ast_complexity each return an error
    let outputs = [
        (
            "ast_callers",
            dispatch_ast_callers(&graph, &helper, None, None).await,
        ),
        (
            "ast_search",
            dispatch_ast_search(&graph, "helper", None, None, None, None, None, None).await,
        ),
        (
            "ast_neighbors",
            dispatch_ast_neighbors(&graph, &helper, None, None).await,
        ),
        (
            "ast_hierarchy",
            dispatch_ast_hierarchy(&graph, &shape, None, None).await,
        ),
        (
            "ast_dead_code",
            dispatch_ast_dead_code(&graph, None, None, None).await,
        ),
        (
            "ast_complexity",
            dispatch_ast_complexity(&graph, None, None, None, None).await,
        ),
    ];
    for (action, output) in outputs {
        let result = parse(&output)?;
        let error = result["error"].as_str().unwrap_or_default();
        assert!(
            error.contains("Graph query"),
            "{action} did not report the failed query: {result}"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn neighbours_run_only_the_queries_of_the_nodes_type() -> TestResult {
    // @step Given an indexed project where a function calls another function
    let module = "def callee():\n    return 0\n\ndef caller():\n    return callee()\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;
    let caller = function(&root, "caller").await?;
    let graph = registry::read_code_graph(&root).await?;
    ok(&dispatch_ast_neighbors(&graph, &caller, None, None).await)?;

    // @step When ast_neighbors runs for the calling function
    let before = graph.queries_run();
    let result = ok(&dispatch_ast_neighbors(&graph, &caller, None, None).await)?;
    let queries = graph.queries_run() - before;

    // @step Then it runs only the 4 function neighbour queries after finding the node's type
    assert_eq!(queries, 4, "{result}");

    // @step And it returns the called function and the containing file
    let neighbors = result["neighbors"]
        .as_array()
        .ok_or_else(|| format!("no neighbors in {result}"))?;
    assert!(
        neighbors
            .iter()
            .any(|n| n["edgeType"] == "Calls" && n["name"] == "callee"),
        "{result}"
    );
    assert!(
        neighbors
            .iter()
            .any(|n| n["edgeType"] == "ContainedBy" && n["path"] == "m.py"),
        "{result}"
    );

    // @step When ast_neighbors runs for a slug that is not in the graph
    let missing = parse(&dispatch_ast_neighbors(&graph, "m.py::nowhere@0", None, None).await)?;

    // @step Then the result is a not-found error
    let error = missing["error"].as_str().unwrap_or_default();
    assert!(error.contains("not found"), "{missing}");
    Ok(())
}

fn version(result: &Value) -> Result<(u64, String), TestError> {
    let indexed_at = result["indexed_at"]
        .as_str()
        .ok_or_else(|| format!("no indexed_at in {result}"))?;
    chrono::DateTime::parse_from_rfc3339(indexed_at)?;
    Ok((number(result, "graph_version")?, indexed_at.to_string()))
}

/// The versions ast_stats, ast_search and ast_callers report for `root`.
async fn reported_versions(root: &Path) -> Result<Vec<(u64, String)>, TestError> {
    let target = function(root, "target").await?;
    let graph = registry::read_code_graph(root).await?;
    let outputs = [
        dispatch_ast_stats(&graph).await,
        dispatch_ast_search(&graph, "target", None, None, None, None, None, None).await,
        dispatch_ast_callers(&graph, &target, None, None).await,
    ];
    outputs.iter().map(|o| version(&ok(o)?)).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_response_names_the_graph_version_it_read() -> TestResult {
    // @step Given a project has been indexed
    let (_directory, root) = project(&[("m.py", &callers_module(&["one".into()], 0))])?;
    let indexed = version(&index(&root).await?)?;

    // @step When ast_stats, ast_search and ast_callers run
    let reported = reported_versions(&root).await?;

    // @step Then each reports the graph_version and indexed_at of the index result
    assert_eq!(reported, vec![indexed.clone(); 3]);

    // @step And after a second index writes the graph each reports a higher graph_version
    std::fs::write(
        root.join("m.py"),
        callers_module(&["one".into(), "two".into()], 0),
    )?;
    let second = index(&root).await?;
    assert_eq!(second["written"], true, "{second}");
    for (graph_version, _) in reported_versions(&root).await? {
        assert!(
            graph_version > indexed.0,
            "{graph_version} <= {}",
            indexed.0
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dead_code_reports_its_uncapped_total_and_the_source_of_the_returned_rows_only(
) -> TestResult {
    // @step Given an indexed project with 5 uncalled functions
    let module = "def dead_4():\n    return 4\n\ndef dead_2():\n    return 2\n\n\
                  def dead_5():\n    return 5\n\ndef dead_1():\n    return 1\n\n\
                  def dead_3():\n    return 3\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;

    // @step When ast_dead_code runs for functions with limit 2
    let graph = registry::read_code_graph(&root).await?;
    let result = ok(&dispatch_ast_dead_code(&graph, Some("Function"), Some(2), None).await)?;

    // @step Then it returns the 2 uncalled functions with the lowest slugs, each with its source
    let functions = &result["results"]["Function"];
    assert_eq!(names(&functions["items"])?, vec!["dead_1", "dead_2"]);
    for item in functions["items"].as_array().into_iter().flatten() {
        let source = item["source"].as_str().unwrap_or_default();
        assert!(source.starts_with("def dead_"), "{item}");
    }

    // @step And the function result has count 2 and total 5
    assert_eq!(number(functions, "count")?, 2);
    assert_eq!(number(functions, "total")?, 5);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_most_complex_functions_are_ranked_without_a_query_per_function() -> TestResult {
    // @step Given an indexed project with functions of cyclomatic complexity 3, 3 and 1
    let branchy = |name: &str| {
        format!("def {name}(a):\n    if a:\n        return 1\n    if a > 2:\n        return 2\n    return 3\n\n")
    };
    let module = format!(
        "{}{}def r1(a):\n    return a\n",
        branchy("q3"),
        branchy("p3")
    );
    let (_directory, root) = project(&[("m.py", &module)])?;
    index(&root).await?;
    let graph = registry::read_code_graph(&root).await?;
    ok(&dispatch_ast_complexity(&graph, None, Some(2), None, None).await)?;

    // @step When ast_complexity runs for the top 2
    let before = graph.queries_run();
    let result = ok(&dispatch_ast_complexity(&graph, None, Some(2), None, None).await)?;
    let queries = graph.queries_run() - before;

    // @step Then it returns the two functions of complexity 3 in slug order with total 3
    assert_eq!(names(&result["results"])?, vec!["p3", "q3"]);
    for row in result["results"].as_array().into_iter().flatten() {
        assert_eq!(row["cyclomaticComplexity"], 3, "{row}");
    }
    assert_eq!(number(&result, "total")?, 3);

    // @step And it runs no query per function
    assert_eq!(queries, 0, "{result}");
    Ok(())
}
