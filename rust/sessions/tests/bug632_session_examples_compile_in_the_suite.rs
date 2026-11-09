//! BUG-632: every codelet-sessions example is compiled into the
//! codelet-sessions test suite.
//!
//! Feature: spec/features/workspace-examples-compile-in-the-quality-check.feature
//!
//! cargo test and clippy --tests never build example targets, so an API
//! change can break an example unnoticed. Every example is compiled into
//! this test binary as a module, so the suite fails to build when one
//! does, and the test fails when an example file is left out.

use codelet_test_helpers::{OrFail, TestResult};

/// The example files compiled into this test binary.
const COMPILED_IN: &[&str] = &["repro_paused_chunk_delay.rs"];

#[test]
fn every_codelet_sessions_example_is_compiled_into_the_codelet_sessions_test_suite() -> TestResult {
    // @step Given the examples directory of codelet-sessions
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");

    // @step When the codelet-sessions guard test target is built
    let mut examples: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&directory).or_fail("the sessions examples directory reads")? {
        let name = entry.or_fail("an examples entry reads")?.file_name();
        let name = name.to_string_lossy().into_owned();
        if name.ends_with(".rs") {
            examples.push(name);
        }
    }
    examples.sort();

    // @step Then each example file is compiled into it as a module
    // @step And an example file that is not compiled in fails the guard test, naming the file
    let left_out: Vec<&String> = examples
        .iter()
        .filter(|name| !COMPILED_IN.contains(&name.as_str()))
        .collect();
    assert!(
        left_out.is_empty(),
        "these codelet-sessions examples are not compiled into this guard test: {left_out:?}"
    );
    assert!(
        !examples.is_empty(),
        "the codelet-sessions examples directory lists the examples"
    );
    Ok(())
}

// The examples, compiled into this test binary: a change that breaks one
// breaks this target too. Each one is listed in `COMPILED_IN`.
#[allow(dead_code)]
#[path = "../examples/repro_paused_chunk_delay.rs"]
mod repro_paused_chunk_delay;
