// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
use super::*;

// ============================================================================
// Scenario: Extract Function nodes from a TypeScript file
// ============================================================================
#[tokio::test]
async fn test_extract_function_nodes_from_typescript_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a TypeScript file "src/auth/login.ts" with async and sync function declarations
    let ts_content = r#"
import { hash } from './utils';
import { Config } from '../config';

export async function login(username: string, password: string): Promise<boolean> {
    const hashed = hash(password);
    return verify(username, hashed);
}

function verify(username: string, hashedPassword: string): boolean {
    return username.length > 0 && hashedPassword.length > 0;
}

export const logout = async (): Promise<void> => {
    await clearSession();
};
"#;
    write_test_file(temp_dir.path(), "src/auth/login.ts", ts_content);

    // @step When the TypeScript extractor parses the file
    let entities = walk_and_extract(temp_dir.path(), false);

    // @step Then Function nodes should be created with correct name, qualifiedName, isAsync, isPublic properties
    assert!(
        count_nodes(&entities, "Function") >= 2,
        "Should extract at least 2 Function nodes, got {}",
        count_nodes(&entities, "Function")
    );

    let login_fn = node_named(&entities, "Function", "src/auth/login.ts::login");
    assert!(login_fn.is_some(), "Should find login function");

    // @step And each Function node should have lineStart and lineEnd positions
    if let Some(GraphEntity::Node { properties, .. }) = login_fn {
        assert!(
            properties.contains_key("lineStart"),
            "Function should have lineStart"
        );
        assert!(
            properties.contains_key("lineEnd"),
            "Function should have lineEnd"
        );
    }

    // @step And each Function node should have a paramCount matching the declaration
    if let Some(GraphEntity::Node { properties, .. }) = login_fn {
        let param_count = properties.get("paramCount").and_then(|v| v.as_i64());
        assert_eq!(param_count, Some(2), "login should have 2 params");
    }

    // @step And a File node should be created for "src/auth/login.ts"
    let source_file = node_named(&entities, "File", "src/auth/login.ts");
    assert!(
        source_file.is_some(),
        "Should create a File node for the source file"
    );

    // @step And Contains edges should link the File to each Function
    assert!(
        count_edges(&entities, "Contains") >= 2,
        "Should have at least 2 Contains edges"
    );
}

// ============================================================================
// Scenario: Extract Type nodes from a Rust file
// ============================================================================
#[tokio::test]
async fn test_extract_type_nodes_from_rust_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a Rust file "src/graph/database.rs" with struct, enum, and trait declarations
    let rs_content = r#"
use std::path::PathBuf;

pub struct GraphDatabase {
    path: PathBuf,
    name: String,
}

pub enum GraphType {
    Memory,
    AstCode,
    Learnings,
}

pub trait Queryable {
    fn query(&self, name: &str) -> Result<String, String>;
}

pub fn ensure_db() -> Result<(), String> {
    Ok(())
}

fn internal_helper(x: i32) -> i32 {
    x + 1
}
"#;
    write_test_file(temp_dir.path(), "src/graph/database.rs", rs_content);

    // @step When the Rust extractor parses the file
    let entities = walk_and_extract(temp_dir.path(), false);

    // @step Then Type nodes should be created for each struct with typeKind "struct_kind"
    let struct_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("struct_kind"))
        })
        .collect();
    assert!(
        !struct_nodes.is_empty(),
        "Should find at least 1 struct Type node"
    );

    // @step And Type nodes should be created for each enum with typeKind "enum_kind"
    let enum_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("enum_kind"))
        })
        .collect();
    assert!(
        !enum_nodes.is_empty(),
        "Should find at least 1 enum Type node"
    );

    // @step And Type nodes should be created for each trait with typeKind "trait_kind"
    let trait_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
                if node_type == "Type"
                    && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind"))
        })
        .collect();
    assert!(
        !trait_nodes.is_empty(),
        "Should find at least 1 trait Type node"
    );

    // @step And Function nodes should be created for each fn declaration
    assert!(
        count_nodes(&entities, "Function") >= 2,
        "Should find at least 2 Function nodes"
    );

    // @step And a File node should be created for "src/graph/database.rs"
    assert_eq!(
        count_nodes(&entities, "File"),
        1,
        "Should create 1 File node"
    );
}

// ============================================================================
// Scenario: Extract import edges from TypeScript files
// ============================================================================
#[tokio::test]
async fn test_extract_import_edges_from_typescript_files() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // @step Given a TypeScript file "src/auth/login.ts" that imports from "src/auth/utils.ts" and "src/config.ts"
    let login_content = r#"
import { hash, compare } from './utils';
import { Config } from '../config';

export function login(user: string): boolean {
    return true;
}
"#;
    let utils_content = r#"
export function hash(input: string): string {
    return input;
}
export function compare(a: string, b: string): boolean {
    return a === b;
}
"#;
    let config_content = r#"
export interface Config {
    dbUrl: string;
}
"#;
    write_test_file(temp_dir.path(), "src/auth/login.ts", login_content);
    write_test_file(temp_dir.path(), "src/auth/utils.ts", utils_content);
    write_test_file(temp_dir.path(), "src/config.ts", config_content);

    // @step When the TypeScript extractor parses the file
    let entities = walk_and_extract(temp_dir.path(), false);

    // @step Then File nodes should be created for all three files
    for path in ["src/auth/login.ts", "src/auth/utils.ts", "src/config.ts"] {
        assert!(
            node_named(&entities, "File", path).is_some(),
            "Should create a File node for {path}"
        );
    }
    let import_edges: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Imports"))
        .collect();

    // @step And Imports edges should link "src/auth/login.ts" to "src/auth/utils.ts"
    assert!(
        has_named_edge(
            &entities,
            "Imports",
            "src/auth/login.ts",
            "src/auth/utils.ts"
        ),
        "Should have Imports edge login -> utils, got {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And Imports edges should link "src/auth/login.ts" to "src/config.ts"
    assert!(
        has_named_edge(&entities, "Imports", "src/auth/login.ts", "src/config.ts"),
        "Should have Imports edge login -> config, got {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And each Imports edge should have the importPath property set
    for edge in &import_edges {
        if let GraphEntity::Edge { properties, .. } = edge {
            assert!(
                properties.contains_key("importPath"),
                "Imports edge should have importPath property"
            );
        }
    }
}
