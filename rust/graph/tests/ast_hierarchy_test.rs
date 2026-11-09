// Feature: spec/features/class-hierarchy-and-inheritance-traversal.feature
//
// Class Hierarchy and Inheritance Traversal
// Tests for ast_hierarchy action that returns the full inheritance tree
// (parents, children, interfaces, methods) for a type.
//
// Each test indexes an isolated temp project with ast_index (the Extends,
// Implements and HasMethod edges come from the resolve pass), then
// exercises the dispatch function directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_hierarchy;
use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry::{self, GraphRead};
use serde_json::Value;

/// A class hierarchy in TypeScript:
///
/// ```text
/// Animal (parent)
///   └── Dog (child, implements Trainable)
///         └── GuideDog (grandchild)
/// ```
///
/// Each class has its own methods.
const ANIMALS_TS: &str = r#"export interface Trainable {
  train(): void;
}

export class Animal {
  speak(sound: string): string {
    return sound;
  }
}

export class Dog extends Animal implements Trainable {
  train(): void {}

  fetch(item: string): string {
    return item;
  }
}

export class GuideDog extends Dog {
  guide(person: string, route: string): void {}
}
"#;

/// A standalone Python class next to a module-level function.
const UTIL_PY: &str = r#"class Formatter:
    def format_text(self, text):
        return text.strip()


def module_helper(value):
    return value
"#;

/// An indexed temp project: the temp directory (keep it alive), the graph
/// opened for reading, and every entity of the graph.
struct Indexed {
    _dir: tempfile::TempDir,
    graph: GraphRead,
    entities: Vec<GraphEntity>,
}

impl Indexed {
    /// The slug of the Type whose qualifiedName is `name`.
    fn type_slug(&self, name: &str) -> String {
        let found: Vec<&String> = self
            .entities
            .iter()
            .filter_map(|e| match e {
                GraphEntity::Node {
                    node_type,
                    slug,
                    properties,
                } if node_type == "Type"
                    && properties.get("qualifiedName").and_then(|v| v.as_str()) == Some(name) =>
                {
                    Some(slug)
                }
                _ => None,
            })
            .collect();
        assert_eq!(found.len(), 1, "one Type {name}: {found:?}");
        found[0].clone()
    }
}

/// Index a temp project holding `files` with ast_index.
async fn index_project(files: &[(&str, &str)]) -> Indexed {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("project");
    for (path, text) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("create dir");
        std::fs::write(full, text).expect("write file");
    }
    let output = dispatch_ast_index(&root, false, false).await;
    let result: Value = serde_json::from_str(&output).expect("ast_index JSON");
    assert!(result.get("error").is_none(), "ast_index failed: {output}");
    let graph = registry::read_code_graph(&root).await.expect("open graph");
    let entities = graph.export_all_entities().await.expect("export");
    Indexed {
        _dir: dir,
        graph,
        entities,
    }
}

/// Helper: index the class hierarchy fixture.
async fn setup_hierarchy_db() -> Indexed {
    index_project(&[("src/animals.ts", ANIMALS_TS)]).await
}

/// Helper: index a standalone class (no parents, no children).
async fn setup_standalone_db() -> Indexed {
    index_project(&[("src/util.py", UTIL_PY)]).await
}

/// The `name` of every element of a result array, sorted.
fn names(items: &Value) -> Vec<String> {
    let mut found: Vec<String> = items
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|i| i["name"].as_str().map(str::to_owned))
        .collect();
    found.sort();
    found
}

// ============================================================================
// Scenario: Find class hierarchy with parents and children
// ============================================================================
#[tokio::test]
async fn test_hierarchy_parents_children_interfaces() {
    let indexed = setup_hierarchy_db().await;

    // @step Given I have a codebase indexed with class inheritance relationships
    // (src/animals.ts: Animal → Dog → GuideDog, Dog implements Trainable)
    let dog = indexed.type_slug("src/animals.ts::Dog");

    // @step When I request ast_hierarchy for a class with parent and child classes
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &indexed.graph,
        &dog,
        None, // default depth
        None, // include_methods default true
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");

    // Verify no error
    assert!(
        result.get("error").is_none(),
        "Expected no error, got: {result_json}"
    );

    // @step Then I should receive the parent classes via Extends edges
    assert_eq!(names(&result["parents"]), ["Animal"], "{result_json}");
    assert_eq!(
        result["parents"][0]["slug"],
        indexed.type_slug("src/animals.ts::Animal").as_str()
    );

    // @step And I should receive the child classes via reverse Extends edges
    assert_eq!(names(&result["children"]), ["GuideDog"], "{result_json}");

    // @step And I should receive implemented interfaces via Implements edges
    assert_eq!(names(&result["interfaces"]), ["Trainable"], "{result_json}");

    // @step And the response should include methods from the type's containing file
    // The methods are Dog's own (HasMethod edges), not every function of
    // the file.
    assert_eq!(
        names(&result["methods"]),
        ["fetch", "train"],
        "{result_json}"
    );
}

// ============================================================================
// Scenario: Multi-level hierarchy traversal
// ============================================================================
#[tokio::test]
async fn test_multi_level_hierarchy() {
    let indexed = setup_hierarchy_db().await;

    // @step Given I have a codebase indexed with a 3-level class hierarchy
    // (Animal → Dog → GuideDog)
    let dog = indexed.type_slug("src/animals.ts::Dog");

    // @step When I request ast_hierarchy for the middle class
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &indexed.graph,
        &dog,
        Some(3), // depth 3 to ensure multi-level
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");
    assert!(
        result.get("error").is_none(),
        "Expected no error, got: {result_json}"
    );

    // @step Then I should receive grandparent classes in the parents array
    // Dog → Animal is direct parent at depth 1
    let parents = result["parents"].as_array().expect("parents array");
    assert!(
        parents.iter().any(|p| p["name"] == "Animal"),
        "Should find Animal (grandparent accessible via Dog)",
    );

    // @step And I should receive grandchild classes in the children array
    let children = result["children"].as_array().expect("children array");
    assert!(
        children.iter().any(|c| c["name"] == "GuideDog"),
        "Should find GuideDog (child of Dog)",
    );

    // From the root of the hierarchy, the grandchild is at depth 2.
    let animal = indexed.type_slug("src/animals.ts::Animal");
    let from_root: Value = serde_json::from_str(
        &ast_hierarchy::dispatch_ast_hierarchy(&indexed.graph, &animal, Some(3), None).await,
    )
    .expect("parse JSON");
    let depths: Vec<(String, u64)> = from_root["children"]
        .as_array()
        .expect("children array")
        .iter()
        .map(|c| {
            (
                c["name"].as_str().expect("name").to_owned(),
                c["depth"].as_u64().expect("depth"),
            )
        })
        .collect();
    assert_eq!(
        depths,
        [("Dog".to_string(), 1), ("GuideDog".to_string(), 2)],
        "{from_root}"
    );
}

// ============================================================================
// Scenario: Standalone type with no inheritance
// ============================================================================
#[tokio::test]
async fn test_standalone_type_no_inheritance() {
    let indexed = setup_standalone_db().await;

    // @step Given I have a codebase indexed with a class that has no parents or children
    // (Formatter class with no extends/implements edges)
    let formatter = indexed.type_slug("src/util.py::Formatter");

    // @step When I request ast_hierarchy for that class
    let result_json =
        ast_hierarchy::dispatch_ast_hierarchy(&indexed.graph, &formatter, None, None).await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");
    assert!(
        result.get("error").is_none(),
        "Expected no error, got: {result_json}"
    );

    // @step Then I should receive the type itself with methods from its containing file
    assert_eq!(result["type"]["name"], "Formatter");
    // Its method is format_text (HasMethod); the module-level function of
    // the same file is not a method.
    assert_eq!(names(&result["methods"]), ["format_text"], "{result_json}");

    // @step And the parents array should be empty
    let parents = result["parents"].as_array().expect("parents array");
    assert!(
        parents.is_empty(),
        "Parents should be empty for standalone type"
    );

    // @step And the children array should be empty
    let children = result["children"].as_array().expect("children array");
    assert!(
        children.is_empty(),
        "Children should be empty for standalone type"
    );
}

// ============================================================================
// Scenario: Non-existent type returns error
// ============================================================================
#[tokio::test]
async fn test_nonexistent_type_returns_error() {
    let indexed = setup_standalone_db().await;

    // @step Given I have a codebase indexed in the AST graph
    // (any indexed graph — we use standalone_db)

    // @step When I request ast_hierarchy for a non-existent type slug
    let result_json = ast_hierarchy::dispatch_ast_hierarchy(
        &indexed.graph,
        "nonexistent.py::FakeType@0",
        None,
        None,
    )
    .await;
    let result: Value = serde_json::from_str(&result_json).expect("parse JSON");

    // @step Then I should receive an error indicating the type was not found
    let error = result["error"].as_str().expect("error field");
    assert!(
        error.contains("not found") || error.contains("Not found"),
        "Error should indicate type not found, got: {error}"
    );
}
