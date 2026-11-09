//! BUG-681: an index lists each dependency manifest that failed.
//!
//! Feature: spec/features/an-index-reports-each-dependency-manifest-that-failed.feature
//!
//! Each scenario indexes a temp project with `ast_index` and reads the
//! result's `dependencies_failed` and the graph's entities. The expected
//! errors are the parsers' own messages for the same text (TEST-042: no
//! timing, no randomness).

use codelet_test_helpers::{TestError, TestResult};
use serde_json::{json, Value};

pub mod code_graph_fixture;
use code_graph_fixture::{entities, index, nodes, project};

const LIB_ALPHA: (&str, &str) = ("src/lib.rs", "pub fn alpha() -> u32 {\n    1\n}\n");
const BAD_JSON: &str = "{\n  \"dependencies\": {\n    \"left-pad\": \n}\n";
const BAD_TOML: &str = "[package\nname = \"a\"\n";

/// The `name`s of the graph's nodes of `node_type`, sorted.
fn names(all: &[codelet_graph::graph_entities::GraphEntity], node_type: &str) -> Vec<String> {
    let mut found: Vec<String> = nodes(all, node_type)
        .into_iter()
        .filter_map(|(_, p)| p.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect();
    found.sort();
    found
}

fn json_error(text: &str) -> Result<String, TestError> {
    match serde_json::from_str::<Value>(text) {
        Ok(_) => Err("the bad JSON fixture parsed".into()),
        Err(e) => Ok(format!("invalid JSON: {e}")),
    }
}

fn toml_error(text: &str) -> Result<String, TestError> {
    match text.parse::<toml::Value>() {
        Ok(_) => Err("the bad TOML fixture parsed".into()),
        Err(e) => Ok(format!("invalid TOML: {e}")),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_package_json_is_listed_and_the_index_succeeds() -> TestResult {
    // @step Given a project holding src/lib.rs with fn alpha and a package.json that is not valid JSON
    let (_dir, root) = project(&[LIB_ALPHA, ("package.json", BAD_JSON)])?;

    // @step When ast_index runs for the project
    let result = index(&root).await;

    // @step Then the index succeeds
    let result = result?;

    // @step And the graph holds the Function alpha
    let all = entities(&root).await?;
    assert_eq!(names(&all, "Function"), vec!["alpha"], "{result}");

    // @step And dependencies_failed is one entry with file package.json, language javascript and the JSON parse error
    assert_eq!(
        result["dependencies_failed"],
        json!([{
            "file": "package.json",
            "language": "javascript",
            "error": json_error(BAD_JSON)?,
        }]),
        "{result}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_workspace_member_is_listed_and_the_other_cargo_manifests_are_indexed(
) -> TestResult {
    // @step Given a Cargo workspace whose root Cargo.toml names the dependency serde and lists member crates/a
    let workspace =
        "[workspace]\nmembers = [\"crates/a\"]\n\n[workspace.dependencies]\nserde = \"1\"\n";
    // @step And crates/a/Cargo.toml is not valid TOML
    let (_dir, root) = project(&[
        ("Cargo.toml", workspace),
        ("crates/a/Cargo.toml", BAD_TOML),
        ("crates/a/src/lib.rs", LIB_ALPHA.1),
    ])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then dependencies_failed is one entry with file crates/a/Cargo.toml, language rust and the TOML parse error
    assert_eq!(
        result["dependencies_failed"],
        json!([{
            "file": "crates/a/Cargo.toml",
            "language": "rust",
            "error": toml_error(BAD_TOML)?,
        }]),
        "{result}"
    );

    // @step And the graph holds the Dependency serde
    let all = entities(&root).await?;
    assert_eq!(names(&all, "Dependency"), vec!["serde"], "{result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_index_whose_manifests_all_parse_reports_no_dependency_failure() -> TestResult {
    // @step Given a project holding src/lib.rs and a valid package.json that names left-pad
    let package = "{\n  \"dependencies\": {\n    \"left-pad\": \"1.3.0\"\n  }\n}\n";
    let (_dir, root) = project(&[LIB_ALPHA, ("package.json", package)])?;

    // @step When ast_index runs for the project
    let result = index(&root).await?;

    // @step Then dependencies_failed is an empty list
    assert_eq!(result["dependencies_failed"], json!([]), "{result}");

    // @step And the graph holds the Dependency left-pad
    let all = entities(&root).await?;
    assert_eq!(names(&all, "Dependency"), vec!["left-pad"], "{result}");
    Ok(())
}
