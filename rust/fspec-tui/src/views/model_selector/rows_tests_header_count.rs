//! PROV-127 — provider header count pluralization (TS parity).
//! A single model reads "(1 model)"; multiple models read "(N models)".
//!
//! Feature: spec/features/model-selector-header-count-label.feature
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rows_test_support::*;
use super::*;

/// Scenario: A provider with exactly one model is labelled with the singular noun
#[test]
fn single_model_header_uses_singular_noun() {
    // @step Given a provider header for a provider with exactly one model
    let providers = vec![provider(
        "moonshot",
        vec![model("kimi-k2", false, false, 200_000, false)],
    )];

    // @step When the header label is rendered
    let rows = build_view_rows(&providers, &expanded_set(&[]), "");
    let header = &rows[0];

    // @step Then the label ends with "(1 model)"
    assert!(
        header.label.ends_with("(1 model)"),
        "expected singular '(1 model)', got: {}",
        header.label
    );
}

/// Scenario: A provider with two models is labelled with the plural noun
#[test]
fn two_model_header_uses_plural_noun() {
    // @step Given a provider header for a provider with two models
    let providers = vec![provider(
        "openai",
        vec![
            model("gpt-5", false, false, 400_000, false),
            model("gpt-5-mini", false, false, 400_000, false),
        ],
    )];

    // @step When the header label is rendered
    let rows = build_view_rows(&providers, &expanded_set(&[]), "");
    let header = &rows[0];

    // @step Then the label ends with "(2 models)"
    assert!(
        header.label.ends_with("(2 models)"),
        "expected plural '(2 models)', got: {}",
        header.label
    );
}
