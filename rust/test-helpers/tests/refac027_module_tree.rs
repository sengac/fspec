//! REFAC-027: split test files are read as one source.
//!
//! Feature: spec/features/split-test-source-composition.feature

use codelet_test_helpers::source_composition::{read_module_tree, read_tree_or_file};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn source_shape_tests_read_a_split_test_file_as_one_source() -> TestResult {
    // @step Given a test crate root that declares a sibling module and a path-attributed part module
    let dir = tempfile::tempdir()?;
    let tests = dir.path().join("tests");
    std::fs::create_dir_all(tests.join("helpers"))?;
    std::fs::create_dir_all(tests.join("shape"))?;
    std::fs::write(
        tests.join("shape.rs"),
        "mod helpers;\n#[path = \"shape/part_a.rs\"]\nmod part_a;\nconst ROOT: &str = \"root-marker\";\n",
    )?;
    std::fs::write(
        tests.join("helpers/mod.rs"),
        "pub const H: &str = \"helper-marker\";\n",
    )?;
    std::fs::write(
        tests.join("shape/part_a.rs"),
        "const P: &str = \"part-marker\";\n",
    )?;

    // @step When the module tree is read through the shared source composition helper
    let text = read_module_tree(&tests.join("shape.rs"))?;

    // @step Then the text contains the root, the sibling module and the part module
    for marker in ["root-marker", "helper-marker", "part-marker"] {
        assert!(text.contains(marker), "missing {marker} in:\n{text}");
    }
    Ok(())
}

#[test]
fn a_rust_file_whose_module_tree_is_broken_reports_the_error() -> TestResult {
    // @step Given a test crate root that declares a sibling module whose file is missing
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("shape.rs");
    std::fs::write(&root, "mod helpers;\nconst ROOT: &str = \"root-marker\";\n")?;

    // @step When the root is read through read_tree_or_file
    let result = read_tree_or_file(&root);

    // @step Then the read fails and names the missing module
    // @step And the root file is not returned on its own
    match result {
        Ok(text) => Err(format!("broken tree read as:\n{text}").into()),
        Err(error) => {
            assert!(error.to_string().contains("helpers"), "{error}");
            Ok(())
        }
    }
}

#[test]
fn a_rust_file_that_does_not_parse_reports_the_error() -> TestResult {
    // @step Given a Rust file that does not parse
    let dir = tempfile::tempdir()?;
    let broken = dir.path().join("broken.rs");
    std::fs::write(&broken, "fn (\n")?;

    // @step When the file is read through read_tree_or_file
    let result = read_tree_or_file(&broken);

    // @step Then the read fails instead of returning the raw text
    match result {
        Ok(text) => Err(format!("unparsable file read as:\n{text}").into()),
        Err(_) => Ok(()),
    }
}
