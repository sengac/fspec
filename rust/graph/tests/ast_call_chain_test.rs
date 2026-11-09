// Feature: spec/features/call-chain-path-tracing-between-two-functions.feature
//
// Call Chain / Path Tracing Between Two Functions
// Tests for ast_call_chain action that finds the shortest call path(s)
// between two functions via multi-hop CALLS edge traversal.
//
// Each test indexes an isolated Python fixture with ast_index (KGRAPH-078),
// then exercises the dispatch function directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_call_chain;
use codelet_graph::registry::{self, GraphRead};
use serde_json::Value;
use std::collections::HashMap;

pub mod code_graph_fixture;

/// A Python project indexed with ast_index (KGRAPH-078: the graph is built
/// by the pipeline, never from hand-written JSONL), with its graph open and
/// the slug of every function by name.
pub struct Fixture {
    _directory: tempfile::TempDir,
    pub graph: GraphRead,
    slugs: HashMap<String, String>,
}

impl Fixture {
    /// The slug of the function `name`.
    pub fn s(&self, name: &str) -> &str {
        self.slugs
            .get(name)
            .unwrap_or_else(|| panic!("no function {name} in {:?}", self.slugs))
    }
}

async fn indexed(module: &str) -> Fixture {
    let (directory, root) = code_graph_fixture::project(&[("m.py", module)]).expect("project");
    code_graph_fixture::index(&root).await.expect("ast_index");
    let all = code_graph_fixture::entities(&root).await.expect("entities");
    let slugs = code_graph_fixture::nodes(&all, "Function")
        .into_iter()
        .map(|(slug, props)| (props["name"].as_str().expect("name").to_string(), slug))
        .collect();
    let graph = registry::read_code_graph(&root).await.expect("graph");
    Fixture {
        _directory: directory,
        graph,
        slugs,
    }
}

/// Helper: an indexed Python module with a linear call chain
/// func_a → func_b → func_c → func_d, plus an isolated function with no
/// incoming or outgoing calls.
async fn setup_linear_chain_db() -> Fixture {
    indexed(
        "def func_a():\n    func_b()\n\ndef func_b():\n    func_c()\n\n\
         def func_c():\n    func_d()\n\ndef func_d():\n    return 0\n\n\
         def isolated():\n    return 1\n",
    )
    .await
}

/// Helper: an indexed Python module with two paths from entry to target:
/// entry → short_path → target (2 hops) and
/// entry → long_path_a → long_path_b → target (3 hops).
async fn setup_multi_path_db() -> Fixture {
    indexed(
        "def entry():\n    short_path()\n    long_path_a()\n\n\
         def short_path():\n    target()\n\ndef long_path_a():\n    long_path_b()\n\n\
         def long_path_b():\n    target()\n\ndef target():\n    return 0\n",
    )
    .await
}

#[path = "ast_call_chain_test/direct_call_chain.rs"]
mod direct_call_chain;
#[path = "ast_call_chain_test/multiple_paths_ordered_by.rs"]
mod multiple_paths_ordered_by;
