//! Feature: spec/features/gherkin-description-formatting.feature
//!
//! RPC-330 formatter tests, moved out of gherkin_format.rs's inline tests
//! (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use gherkin::GherkinEnv;

fn parse(src: &str) -> Feature {
    Feature::parse(src, GherkinEnv::default()).expect("parse")
}

// ====================================================================
// RPC-330: Gherkin Description Blank-Line Preservation in Formatter
//
// Feature: spec/features/gherkin-description-formatting.feature
//
// These tests assert that `fspec format` (exercised here through the
// formatter entry point `format_feature` over a parsed source) preserves
// blank lines between paragraphs inside feature/scenario descriptions.
// They are RED until the formatter re-extracts description text from the
// raw source instead of the lossy parsed `description` field.
// ====================================================================

#[test]
fn rpc330_preserves_blank_line_between_feature_description_paragraphs() {
    // @step Given a feature file whose Feature header is followed by this description:
    let src = "Feature: Multi paragraph\n\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the feature description retains exactly one blank line between the two paragraphs:
    assert_eq!(
        out,
        "Feature: Multi paragraph\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n",
        "full formatted output must match TS-parity layout (no leading blank after Feature header, inter-paragraph blank preserved), got:\n{out}"
    );
}

#[test]
fn rpc330_preserves_blank_line_between_scenario_description_paragraphs() {
    // @step Given a feature file with a scenario whose header is followed by this description:
    let src = "Feature: Scenario desc\n\n  Scenario: Has prose\n    First paragraph of the scenario description.\n\n    Second paragraph of the scenario description.\n\n    Given x\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the scenario description retains exactly one blank line between the two paragraphs:
    assert!(
        out.contains(
            "    First paragraph of the scenario description.\n\n    Second paragraph of the scenario description.\n"
        ),
        "blank line between scenario description paragraphs was dropped, got:\n{out}"
    );
}

#[test]
fn rpc330_single_paragraph_description_is_unchanged() {
    // @step Given a feature file with a single-line feature description and no internal blank lines:
    let src =
        "Feature: One line desc\n\n  Only one paragraph here.\n\n  Scenario: A\n    Given x\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the description is emitted with no leading or internal blank line and is byte-identical to the input layout:
    assert_eq!(
        out, "Feature: One line desc\n  Only one paragraph here.\n\n  Scenario: A\n    Given x\n",
        "single-paragraph description must have no leading/internal blank line, got:\n{out}"
    );
}

#[test]
fn rpc330_collapses_more_than_two_blank_lines_to_two() {
    // @step Given a feature file whose feature description separates "Paragraph one." and "Paragraph two." by four consecutive blank lines
    let src = "Feature: Excessive blanks\n\n  Paragraph one.\n\n\n\n\n  Paragraph two.\n\n  Scenario: A\n    Given x\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the two paragraphs are separated by exactly two blank lines:
    assert!(
        out.contains("  Paragraph one.\n\n\n  Paragraph two.\n"),
        "four blank lines must collapse to exactly two, got:\n{out}"
    );
    assert!(
        !out.contains("  Paragraph one.\n\n\n\n  Paragraph two.\n"),
        "must not emit three or more blank lines between paragraphs, got:\n{out}"
    );
}

#[test]
fn rpc330_step_docstring_with_internal_blank_line_is_not_regressed() {
    // @step Given a feature file with a step doc string whose body is "line one", a blank line, then "line three"
    let src = "Feature: Docstring intact\n\n  Scenario: A\n    Given step:\n      \"\"\"\n      line one\n\n      line three\n      \"\"\"\n    Then ok\n";

    // @step When the formatter formats the feature file
    let out = format_feature(&parse(src), src);

    // @step Then the step doc string body is emitted unchanged with no spurious blanks added or removed
    assert!(
        out.contains("      \"\"\"\n      line one\n\n      line three\n      \"\"\"\n"),
        "step doc string body must be preserved exactly, got:\n{out}"
    );
}

#[test]
fn rpc330_multi_paragraph_description_is_idempotent() {
    // @step Given a feature file with a two-paragraph feature description that has already been formatted once
    let src = "Feature: Multi paragraph\n\n  First paragraph of the feature description.\n\n  Second paragraph of the feature description.\n\n  Scenario: A\n    Given x\n";
    let once = format_feature(&parse(src), src);

    // @step When the formatter formats the feature file a second time
    let twice = format_feature(&parse(&once), &once);

    // @step Then the output of the second run is byte-identical to the output of the first run
    assert_eq!(
        once, twice,
        "multi-paragraph description formatting must be idempotent"
    );
}
