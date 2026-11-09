//! BUG-359 — paint_spinner computes no discarded spinner glyph.
//!
//! Feature: spec/features/paint-spinner-computes-no-discarded-glyph.feature
//!
//! Source-shape test: paint_spinner painted the row through
//! paint_spinner_line and then evaluated a #[must_use]
//! `current_frame_glyph(elapsed_ms)` only to drop it.

use codelet_test_helpers::{OrFail, TestResult};
mod common;

/// The text of `fn paint_spinner` up to its closing brace at column 0.
fn paint_spinner_body(source: &str) -> Result<&str, codelet_test_helpers::TestError> {
    let start = source
        .find("fn paint_spinner(")
        .or_fail("input_transition.rs defines fn paint_spinner")?;
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .or_fail("fn paint_spinner closes at column 0")?;
    Ok(&rest[..end])
}

/// Scenario: paint_spinner does not compute and discard a spinner glyph
#[test]
fn paint_spinner_does_not_compute_and_discard_a_spinner_glyph() -> TestResult {
    // @step Given the rust/fspec-tui/src/views/agent/input_transition.rs source
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("views")
        .join("agent")
        .join("input_transition.rs");
    let source = common::strip_rust_comments(&common::read_to_string_or_fail(&path)?);

    // @step When a developer reads the body of paint_spinner
    let body = paint_spinner_body(&source)?;

    // @step Then it paints the spinner row through paint_spinner_line
    assert!(
        body.contains("paint_spinner_line("),
        "paint_spinner must paint through paint_spinner_line, got:\n{body}"
    );

    // @step And it contains no discarded current_frame_glyph call
    assert!(
        !body.contains("current_frame_glyph("),
        "paint_spinner must not compute a glyph it never paints, got:\n{body}"
    );
    Ok(())
}
