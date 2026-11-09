//! BUG-632: workspace examples compile in the quality check.
//!
//! Feature: spec/features/workspace-examples-compile-in-the-quality-check.feature
//!
//! TUI-130 added `RenderedChunk::row_ends`; the scrollback_perf example
//! kept building chunks without it and nothing noticed, because cargo test
//! and clippy --tests never build example targets. Every fspec-tui example
//! is compiled into this test binary as a module, so the fspec-tui suite
//! fails to build when an example does, and a test fails when an example
//! file is left out.

use codelet_test_helpers::{OrFail, TestResult};

// The examples, compiled into this test binary: a change that breaks one
// breaks this target too. Each one is listed in `COMPILED_IN`.
#[allow(dead_code)]
#[path = "../examples/scrollback_perf.rs"]
mod scrollback_perf;

/// The example files compiled in above.
const COMPILED_IN: &[&str] = &["scrollback_perf.rs", "probe_pause.rs", "repro_env_pause.rs"];

const EXAMPLE_SOURCE: &str = include_str!("../examples/scrollback_perf.rs");

#[test]
fn the_scrollback_perf_example_builds_its_chunks_with_the_row_ends_of_their_wrap() -> TestResult {
    // @step Given the fspec-tui example scrollback_perf
    let source = EXAMPLE_SOURCE;

    // @step When it pushes a wrapped chunk onto its ScrollbackList
    let push = source
        .split("fn push_src(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .or_fail("scrollback_perf.rs has a push_src function")?;

    // @step Then the RenderedChunk takes its lines and its row ends from one wrap_source_rows call
    assert_eq!(
        push.matches("wrap_source_rows(&source, ").count(),
        1,
        "push_src wraps its source with exactly one wrap_source_rows call:\n{push}"
    );
    assert!(
        push.contains("let (lines, row_ends) = wrap_source_rows("),
        "the lines and the row ends come from that one call:\n{push}"
    );
    assert!(
        push.contains("        lines,\n") && push.contains("        row_ends,\n"),
        "the RenderedChunk is built with those lines and row ends:\n{push}"
    );
    Ok(())
}

#[test]
fn every_fspec_tui_example_is_compiled_into_the_fspec_tui_test_suite() -> TestResult {
    // @step Given the examples directory of fspec-tui
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");

    // @step When the fspec-tui guard test target is built
    let mut examples: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&directory).or_fail("the fspec-tui examples directory reads")? {
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
        "these fspec-tui examples are not compiled into this guard test: {left_out:?}"
    );
    assert!(
        !examples.is_empty(),
        "the fspec-tui examples directory lists the examples"
    );
    Ok(())
}

#[allow(dead_code)]
#[path = "../examples/probe_pause.rs"]
mod probe_pause;

#[allow(dead_code)]
#[path = "../examples/repro_env_pause.rs"]
mod repro_env_pause;
