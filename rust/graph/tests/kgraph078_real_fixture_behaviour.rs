//! KGRAPH-078: KGRAPH features match behaviour, and their tests index real
//! fixtures.
//!
//! Feature: spec/features/kgraph-features-match-behaviour-and-tests-index-real-fixtures.feature
//!
//! Every scenario builds its graph with `ast_index` from a temp Python
//! project; nothing is loaded from hand-written JSONL (TEST-042: no timing,
//! no randomness).

use std::path::Path;

use codelet_graph::ast_call_chain::dispatch_ast_call_chain;
use codelet_graph::ast_dead_code::dispatch_ast_dead_code;
use codelet_graph::registry;
use codelet_test_helpers::{TestError, TestResult};
use serde_json::Value;

pub mod code_graph_fixture;
use code_graph_fixture::{entities, index, project, slug_of};

fn ok(output: &str) -> Result<Value, TestError> {
    let value: Value = serde_json::from_str(output)?;
    if let Some(error) = value.get("error") {
        return Err(format!("unexpected error: {error}").into());
    }
    Ok(value)
}

/// The one hop of the one chain from a to b in `root`'s graph.
async fn hop_from_a_to_b(root: &Path) -> Result<Value, TestError> {
    let all = entities(root).await?;
    let from = slug_of(&all, "Function", "m.py::a")?;
    let to = slug_of(&all, "Function", "m.py::b")?;
    let graph = registry::read_code_graph(root).await?;
    let result = ok(&dispatch_ast_call_chain(&graph, &from, &to, None).await)?;
    let chains = result["chains"]
        .as_array()
        .ok_or_else(|| format!("no chains in {result}"))?;
    let [chain] = chains.as_slice() else {
        return Err(format!("expected one chain: {result}").into());
    };
    let hops = chain["call_details"]
        .as_array()
        .ok_or_else(|| format!("no call_details in {chain}"))?;
    let [hop] = hops.as_slice() else {
        return Err(format!("expected one hop: {chain}").into());
    };
    Ok(hop.clone())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hop_of_a_call_chain_carries_the_call_count_and_conditionality_of_its_calls_edge(
) -> TestResult {
    // @step Given an indexed Python fixture where a calls b twice, both inside an if
    let module = "def b():\n    return 0\n\ndef a(x):\n    if x:\n        b()\n        b()\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;

    // @step When ast_call_chain runs from a to b
    let hop = hop_from_a_to_b(&root).await?;

    // @step Then the chain's one hop has callCount 2 and isConditional true
    assert_eq!(hop["callCount"], 2, "{hop}");
    assert_eq!(hop["isConditional"], true, "{hop}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unconditional_single_call_has_call_count_1_and_is_not_conditional() -> TestResult {
    // @step Given an indexed Python fixture where a calls b once outside any branch
    let module = "def b():\n    return 0\n\ndef a():\n    return b()\n";
    let (_directory, root) = project(&[("m.py", module)])?;
    index(&root).await?;

    // @step When ast_call_chain runs from a to b
    let hop = hop_from_a_to_b(&root).await?;

    // @step Then the chain's one hop has callCount 1 and isConditional false
    assert_eq!(hop["callCount"], 1, "{hop}");
    assert_eq!(hop["isConditional"], false, "{hop}");
    Ok(())
}

const UNRESOLVED_MODULE: &str =
    "def unused():\n    return 0\n\ndef main(client):\n    return client.fetch()\n";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dead_code_reports_the_unresolved_reference_counts_of_the_last_index() -> TestResult {
    // @step Given a Python fixture with an uncalled function and a call on an unknown receiver has been indexed
    let (_directory, root) = project(&[("m.py", UNRESOLVED_MODULE)])?;
    let indexed = index(&root).await?;
    assert!(indexed["unresolved"].is_object(), "{indexed}");

    // @step When ast_dead_code runs
    let graph = registry::read_code_graph(&root).await?;
    let result = ok(&dispatch_ast_dead_code(&graph, None, None, None).await)?;

    // @step Then its unresolved counts equal the unresolved counts of the ast_index result
    assert_eq!(result["unresolved"], indexed["unresolved"], "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_bundle_import_dead_code_says_the_unresolved_counts_are_not_known() -> TestResult {
    // @step Given an indexed Python fixture whose graph was exported to a bundle
    let (directory, root) = project(&[("m.py", UNRESOLVED_MODULE)])?;
    index(&root).await?;
    let bundle = directory.path().join("graph.astbundle");
    {
        let graph = registry::read_code_graph(&root).await?;
        graph
            .export_bundle(&bundle, registry::AST_CODE_SCHEMA)
            .await?;
    }

    // @step When the bundle is imported into the graph and ast_dead_code runs
    {
        let mut lease = registry::write_code_graph(&root).await?;
        let db = lease.database().await?;
        db.import_bundle(&bundle, registry::AST_CODE_SCHEMA, "overwrite")
            .await?;
    }
    let graph = registry::read_code_graph(&root).await?;
    let result = ok(&dispatch_ast_dead_code(&graph, None, None, None).await)?;

    // @step Then its unresolved is null
    assert!(result.get("unresolved") == Some(&Value::Null), "{result}");

    // @step And unresolved_unknown says the graph was not last written by ast_index
    let reason = result["unresolved_unknown"].as_str().unwrap_or_default();
    assert!(reason.contains("ast_index"), "{result}");
    Ok(())
}
