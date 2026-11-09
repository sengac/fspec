// Feature: spec/features/source-code-metadata-storage.feature
//
// Source Code and Metadata Storage in Graph Nodes
// Tests that Function and Type nodes store source code, docstrings,
// parameters, decorators, and language as graph properties after indexing.
//
// Integration tests index a real TypeScript + Python fixture with
// ast_index, then verify the named AST queries return the metadata.
// The other tests index small fixtures with the fact pipeline and read the
// Function and Type nodes' properties.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry::{self, GraphRead};

pub mod graph_test_helpers;
use graph_test_helpers::{index_fixture, write_test_file};

/// The TypeScript file of the metadata fixture.
const HANDLER_TS: &str = r#"/** Configuration options. */
interface Options {
  verbose: boolean;
  timeout: number;
}

class Handler {
  /** Dispatch an action to the appropriate handler. */
  @Injectable
  async dispatch(action: string, options: Options) {
    if (action === 'create') {
      return create(options);
    }
    return update(options);
  }
}

function plainHelper() {
  return 42;
}
"#;

/// The Python file of the metadata fixture.
const SERVICE_PY: &str = r#"@staticmethod
def process(name: str, age: int):
    """Process a user record."""
    if age < 0:
        raise ValueError('negative age')
    return {'name': name, 'age': age}


@dataclass
class UserService:
    """Service for managing user operations."""
    pass
"#;

/// Helper: index a TypeScript + Python project with ast_index and open its
/// graph for reading.
async fn setup_metadata_db(temp_dir: &std::path::Path) -> GraphRead {
    let root = temp_dir.join("project");
    write_test_file(&root, "src/handler.ts", HANDLER_TS);
    write_test_file(&root, "src/service.py", SERVICE_PY);
    let output = dispatch_ast_index(&root, false, false).await;
    let result: serde_json::Value = serde_json::from_str(&output).expect("ast_index JSON");
    assert!(result.get("error").is_none(), "ast_index failed: {output}");
    registry::read_code_graph(&root).await.expect("open graph")
}

/// Index one file and return the properties of its one `node_type` node
/// named `name`.
fn node_props(
    rel: &str,
    source: &str,
    node_type: &str,
    name: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let (_dir, indexed) = index_fixture(&[(rel, source)]);
    let found: Vec<_> = indexed
        .entities
        .into_iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type: t,
                properties,
                ..
            } if t == node_type
                && properties.get("name").and_then(|v| v.as_str()) == Some(name) =>
            {
                Some(properties)
            }
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "one {node_type} {name} in {rel}: {found:?}");
    found.into_iter().next().expect("one node")
}

/// A string property of a node.
fn prop<'a>(props: &'a serde_json::Map<String, serde_json::Value>, key: &str) -> &'a str {
    props
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("string property {key} in {props:?}"))
}

/// Bundled AST query source for named queries.
const AST_QUERIES: &str = include_str!("../schemas/ast-queries.gq");

#[path = "ast_metadata_test/decorator_extraction_language_specific.rs"]
mod decorator_extraction_language_specific;
#[path = "ast_metadata_test/function_nodes_include_metadata.rs"]
mod function_nodes_include_metadata;
