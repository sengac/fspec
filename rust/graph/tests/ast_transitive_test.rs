// Feature: spec/features/transitive-callers-callees-multi-hop-traversal.feature
//
// Transitive Callers / Callees (Multi-Hop Traversal)
// Tests for ast_callers and ast_callees actions that find all direct and
// transitive callers or callees of a function via multi-hop CALLS edge
// traversal using BFS.
//
// Each test indexes an isolated Python fixture with ast_index (KGRAPH-078),
// then exercises the dispatch functions directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_transitive;
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

/// Helper: an indexed Python module with a depth-4 chain:
/// entry → step1 → step2 → step3 → leaf.
async fn setup_deep_chain_db() -> Fixture {
    indexed(
        "def entry():\n    step1()\n\ndef step1():\n    step2()\n\n\
         def step2():\n    step3()\n\ndef step3():\n    leaf()\n\ndef leaf():\n    return 0\n",
    )
    .await
}

#[path = "ast_transitive_test/find_all_transitive_callers.rs"]
mod find_all_transitive_callers;
