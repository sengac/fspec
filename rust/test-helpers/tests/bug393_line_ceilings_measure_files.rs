//! BUG-393 — per-file line ceilings measure files, not module trees.
//!
//! Feature: spec/features/line-ceilings-measure-files.feature

use codelet_test_helpers::source_composition::tree_measured_line_ceilings;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TREE_CEILING: &str = r#"
fn read_tree(path: &std::path::Path) -> String {
    codelet_test_helpers::source_composition::read_module_tree(path).unwrap_or_default()
}

#[test]
fn module_stays_under_300_lines() {
    let lines = read_tree(std::path::Path::new("src/lib.rs")).lines().count();
    assert!(lines < 300);
}
"#;

#[test]
fn a_ceiling_on_module_tree_text_is_detected() -> TestResult {
    // @step Given a test source whose line ceiling counts read_module_tree text
    // @step When it is scanned for tree-measured line ceilings
    let found = tree_measured_line_ceilings(TREE_CEILING)?;
    // @step Then the ceiling is reported
    assert_eq!(found, vec!["module_stays_under_300_lines".to_string()]);
    Ok(())
}

#[test]
fn no_workspace_line_ceiling_measures_a_module_tree() -> TestResult {
    // @step Given the workspace test sources
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut offenders = Vec::new();
    // @step When they are scanned for tree-measured line ceilings
    for crate_dir in std::fs::read_dir(&root)? {
        let tests = crate_dir?.path().join("tests");
        if !tests.is_dir() {
            continue;
        }
        let mut stack = vec![tests];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs")
                    && path
                        .file_name()
                        .is_some_and(|name| name != "bug393_line_ceilings_measure_files.rs")
                {
                    let source = std::fs::read_to_string(&path)?;
                    let found = tree_measured_line_ceilings(&source)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    for function in found {
                        offenders.push(format!("{}::{function}", path.display()));
                    }
                }
            }
        }
    }
    // @step Then none is found
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}
