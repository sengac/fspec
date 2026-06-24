// Feature: spec/features/ast-dependency-graph-population.feature
//
// AST Dependency Graph Population
// Tests for extracting dependency information from package.json and Cargo.toml
// into Dependency graph nodes with DependsOn edges.
//
// Each test uses an isolated temp directory with synthetic manifest files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::cargo_dep_extractor;
use codelet_napi::graph::ast_pipeline::npm_dep_extractor;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{count_edges, count_nodes, find_node, write_test_file};

// ============================================================================
// Scenario: Parse package.json dependencies into Dependency nodes
// ============================================================================
#[test]
fn test_parse_package_json_dependencies() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a project directory with a package.json containing dependencies and devDependencies
    let pkg_json = r#"{
  "name": "my-project",
  "version": "1.0.0",
  "dependencies": {
    "express": "^4.18.0",
    "lodash": "~4.17.21"
  },
  "devDependencies": {
    "vitest": "^2.0.0",
    "typescript": "^5.3.0"
  }
}"#;
    write_test_file(temp_dir.path(), "package.json", pkg_json);

    // @step When the npm dependency extractor parses the package.json
    let entities = npm_dep_extractor::extract_npm_dependencies(temp_dir.path())
        .expect("npm extraction should succeed");

    // @step Then Dependency nodes should be created for each dependency with name, version, and isDev=false
    let express_node = find_node(&entities, "Dependency", "dep::express");
    assert!(
        express_node.is_some(),
        "Should find express Dependency node"
    );
    if let Some(GraphEntity::Node { properties, .. }) = express_node {
        assert_eq!(
            properties.get("isDev").and_then(|v| v.as_bool()),
            Some(false),
            "express should have isDev=false"
        );
        assert_eq!(
            properties.get("version").and_then(|v| v.as_str()),
            Some("^4.18.0"),
            "express should have correct version"
        );
    }

    let lodash_node = find_node(&entities, "Dependency", "dep::lodash");
    assert!(lodash_node.is_some(), "Should find lodash Dependency node");

    // @step And Dependency nodes should be created for each devDependency with name, version, and isDev=true
    let vitest_node = find_node(&entities, "Dependency", "dep::vitest");
    assert!(vitest_node.is_some(), "Should find vitest Dependency node");
    if let Some(GraphEntity::Node { properties, .. }) = vitest_node {
        assert_eq!(
            properties.get("isDev").and_then(|v| v.as_bool()),
            Some(true),
            "vitest should have isDev=true"
        );
    }

    let ts_node = find_node(&entities, "Dependency", "dep::typescript");
    assert!(ts_node.is_some(), "Should find typescript Dependency node");

    // @step And each Dependency node should have a slug in the format "dep::<package-name>"
    assert_eq!(
        count_nodes(&entities, "Dependency"),
        4,
        "Should have 4 Dependency nodes"
    );

    // @step And each Dependency node should have source "npm"
    for entity in &entities {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "Dependency" {
                assert_eq!(
                    properties.get("source").and_then(|v| v.as_str()),
                    Some("npm"),
                    "All npm deps should have source=npm"
                );
            }
        }
    }

    // @step And DependsOn edges should link the package.json File node to each Dependency
    assert_eq!(
        count_edges(&entities, "DependsOn"),
        4,
        "Should have 4 DependsOn edges"
    );
    assert_eq!(
        count_nodes(&entities, "File"),
        1,
        "Should have 1 File node for package.json"
    );
}

// ============================================================================
// Scenario: Parse Cargo.toml dependencies into Dependency nodes
// ============================================================================
#[test]
fn test_parse_cargo_toml_dependencies() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a project directory with a Cargo.toml containing dependencies and dev-dependencies
    let cargo_toml = r#"
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"

[dev-dependencies]
tempfile = "3"
"#;
    write_test_file(temp_dir.path(), "Cargo.toml", cargo_toml);

    // @step When the cargo dependency extractor parses the Cargo.toml
    let entities = cargo_dep_extractor::extract_cargo_dependencies(temp_dir.path())
        .expect("cargo extraction should succeed");

    // @step Then Dependency nodes should be created for each dependency with correct version constraints
    let serde_node = find_node(&entities, "Dependency", "dep::serde");
    assert!(serde_node.is_some(), "Should find serde Dependency node");
    if let Some(GraphEntity::Node { properties, .. }) = serde_node {
        assert_eq!(
            properties.get("isDev").and_then(|v| v.as_bool()),
            Some(false),
            "serde should have isDev=false"
        );
    }

    let tokio_node = find_node(&entities, "Dependency", "dep::tokio");
    assert!(tokio_node.is_some(), "Should find tokio Dependency node");

    let anyhow_node = find_node(&entities, "Dependency", "dep::anyhow");
    assert!(anyhow_node.is_some(), "Should find anyhow Dependency node");

    // @step And Dependency nodes should be created for each dev-dependency with isDev=true
    let tempfile_node = find_node(&entities, "Dependency", "dep::tempfile");
    assert!(
        tempfile_node.is_some(),
        "Should find tempfile Dependency node"
    );
    if let Some(GraphEntity::Node { properties, .. }) = tempfile_node {
        assert_eq!(
            properties.get("isDev").and_then(|v| v.as_bool()),
            Some(true),
            "tempfile should have isDev=true"
        );
    }

    // @step And each Dependency node should have source "crate"
    for entity in &entities {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "Dependency" {
                assert_eq!(
                    properties.get("source").and_then(|v| v.as_str()),
                    Some("crate"),
                    "All cargo deps should have source=crate"
                );
            }
        }
    }

    // @step And DependsOn edges should link the Cargo.toml File node to each Dependency
    assert_eq!(
        count_edges(&entities, "DependsOn"),
        4,
        "Should have 4 DependsOn edges"
    );
    assert_eq!(
        count_nodes(&entities, "Dependency"),
        4,
        "Should have 4 Dependency nodes"
    );
}

// ============================================================================
// Scenario: Parse Cargo workspace with multiple member crates
// ============================================================================
#[test]
fn test_parse_cargo_workspace_with_members() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // @step Given a Cargo workspace with a root Cargo.toml listing member crates
    let root_cargo = r#"
[workspace]
members = ["crate-a", "crate-b"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
"#;
    write_test_file(root, "Cargo.toml", root_cargo);

    // @step And each member crate has its own Cargo.toml with dependencies
    let crate_a = r#"
[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = "1"
serde.workspace = true

[dev-dependencies]
tempfile = "3"
"#;
    write_test_file(root, "crate-a/Cargo.toml", crate_a);

    let crate_b = r#"
[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
serde.workspace = true
"#;
    write_test_file(root, "crate-b/Cargo.toml", crate_b);

    // Also create src dirs so they look like real crates
    write_test_file(root, "crate-a/src/lib.rs", "// placeholder\n");
    write_test_file(root, "crate-b/src/lib.rs", "// placeholder\n");

    // @step When the cargo dependency extractor parses the workspace
    let entities = cargo_dep_extractor::extract_cargo_dependencies(root)
        .expect("workspace extraction should succeed");

    // @step Then Dependency nodes should be created from all member crate Cargo.toml files
    assert!(
        count_nodes(&entities, "Dependency") >= 3,
        "Should have at least 3 unique Dependency nodes (tokio, serde, anyhow), got {}",
        count_nodes(&entities, "Dependency")
    );

    let tokio_dep = find_node(&entities, "Dependency", "dep::tokio");
    assert!(tokio_dep.is_some(), "Should find tokio from crate-a");

    let anyhow_dep = find_node(&entities, "Dependency", "dep::anyhow");
    assert!(anyhow_dep.is_some(), "Should find anyhow from crate-b");

    // @step And DependsOn edges should link each member crate's Cargo.toml to its dependencies
    assert!(
        count_edges(&entities, "DependsOn") >= 5,
        "Should have at least 5 DependsOn edges (crate-a: tokio+serde+tempfile, crate-b: anyhow+serde), got {}",
        count_edges(&entities, "DependsOn")
    );

    // @step And workspace-level dependencies should be included
    let serde_dep = find_node(&entities, "Dependency", "dep::serde");
    assert!(
        serde_dep.is_some(),
        "Should find serde (workspace dep used by both crates)"
    );
}
