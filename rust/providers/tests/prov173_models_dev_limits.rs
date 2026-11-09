//! Feature: spec/features/models-dev-limit-info.feature
//!
//! PROV-173 — models.dev limits have no guessed Default, and a catalog limit
//! of 0 reads as unavailable.

use codelet_providers::models::LimitInfo;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_zero_catalog_limit_is_unavailable() -> TestResult {
    // @step Given a models.dev model entry whose limit is context 32768 and output 0
    let limit: LimitInfo = serde_json::from_str(r#"{"context":32768,"output":0}"#)?;
    // @step When its limits are read
    let (context, output) = (limit.context_window(), limit.max_output_tokens());
    // @step Then its context window is 32768
    assert_eq!(context, Some(32_768));
    // @step And its max output tokens are unavailable
    assert_eq!(output, None);
    Ok(())
}

#[test]
fn model_limits_have_no_guessed_default() -> TestResult {
    // @step Given the models.dev LimitInfo type source
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/models/types.rs");
    let source = std::fs::read_to_string(path)?;
    // @step When its trait implementations are listed
    let compact: String = source.split_whitespace().collect();
    // @step Then it does not implement Default
    assert!(!compact.contains("implDefaultforLimitInfo"));
    let derive_line = source
        .lines()
        .take_while(|line| !line.contains("pub struct LimitInfo"))
        .last()
        .ok_or("LimitInfo is declared")?;
    assert!(!derive_line.contains("Default"), "{derive_line}");
    Ok(())
}
