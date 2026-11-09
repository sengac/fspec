//! BUG-462 — Copilot limits and the context fill are never saturated.
//!
//! Feature: spec/features/copilot-limits-and-fill-are-never-saturated.feature
#![cfg(feature = "test-support")]

use codelet_providers::copilot::models::{build_catalog_from_response, CopilotModelsResponse};
use codelet_rpc_types::{ContextProvenance, RequestContextSnapshot};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn an_oversized_copilot_output_limit_fails_the_catalog_naming_the_model_and_value() -> TestResult {
    // @step Given the Copilot models endpoint advertises model "big-model" with max_output_tokens 5000000000
    let response: CopilotModelsResponse = serde_json::from_value(json!({"data": [{
        "id": "big-model", "name": "Big Model", "version": "big-model-2099-01-01",
        "model_picker_enabled": true,
        "capabilities": {"family": "any-family",
            "limits": {"max_context_window_tokens": 100, "max_output_tokens": 5_000_000_000u64,
                "max_prompt_tokens": 100},
            "supports": {"streaming": true, "tool_calls": true, "vision": false}}
    }]}))?;
    // @step When the Copilot catalog is built
    let built = build_catalog_from_response(&response);
    // @step Then building fails naming "big-model" and 5000000000
    let error = built.err().ok_or("the catalog was built")?;
    assert!(
        error.contains("big-model") && error.contains("5000000000"),
        "{error}"
    );
    Ok(())
}

#[test]
fn a_context_fill_percentage_that_does_not_fit_is_an_error() -> TestResult {
    // @step Given a measured request context with 5000000000 input tokens and a threshold of 1 token
    let snapshot = RequestContextSnapshot {
        model: "any-model".to_string(),
        generation: 0,
        request: 1,
        physical_request: None,
        input_tokens: Some(5_000_000_000),
        output_tokens: Some(0),
        capacity: Some(10),
        threshold: Some(1),
        provenance: ContextProvenance::Measured,
    };
    // @step When its context fill is computed
    let fill = snapshot.context_fill();
    // @step Then the computation fails naming the percentage 500000000000
    let error = fill.err().ok_or("the fill was computed")?;
    assert!(error.contains("500000000000"), "{error}");
    Ok(())
}
