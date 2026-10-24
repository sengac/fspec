// Feature: spec/features/explicit-request-admission-without-fallback.feature
use codelet_cli::compaction_threshold::calculate_usable_context;
use codelet_cli::interactive::is_context_overflow_error;
use codelet_providers::codex::diagnostics::{CodexRejection, EvidenceState, RejectionClass};
use codelet_providers::{resolve_context_window, resolve_max_output_tokens, CodexProvider};

#[test]
fn missing_model_limits_cannot_become_usable_defaults() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a Codex model without registry limits or explicit limits
    let provider = CodexProvider::from_api_key("sk-proj-test-admission", "unknown-model")?;
    // @step When the model limits are resolved for admission
    let context = resolve_context_window(None, None, &provider);
    let output = resolve_max_output_tokens(None, None, &provider);
    // @step Then context and output capacity remain unavailable
    assert_eq!((context, output), (0, 0));
    Ok(())
}

#[test]
fn unknown_output_capacity_cannot_become_an_output_reservation() {
    // @step Given an explicit context capacity and unknown output capacity
    let context = 200_000;
    // @step When the usable request budget is calculated
    let usable = calculate_usable_context(context, 0);
    // @step Then no positive usable budget is returned
    assert_eq!(usable, 0);
}

#[test]
fn unrelated_provider_text_does_not_request_compaction() {
    // @step Given an unknown error mentioning invalid_request_error tokens and maximum
    let error = anyhow::anyhow!("invalid_request_error: token maximum is misconfigured")
        .context("input is too long for the logging buffer");
    // @step When recovery classifies the error through its source chain
    let overflow = is_context_overflow_error(&error);
    // @step Then compaction is not requested
    assert!(!overflow);
}

#[test]
fn declared_typed_overflow_remains_detectable_through_wrappers() {
    // @step Given a declared Codex context length rejection
    let rejection = CodexRejection {
        status: Some(400),
        class: RejectionClass::ContextLengthExceeded,
        code: Some("context_length_exceeded".into()),
        message: Some("request rejected".into()),
        request_id: None,
        retry_after: None,
        attempts: Some(1),
        state: EvidenceState::Complete,
        body: String::new(),
    };
    // @step When recovery classifies the error through streaming and context wrappers
    let error = anyhow::Error::from(rig::completion::CompletionError::HttpError(
        rig::http_client::Error::Instance(Box::new(rejection)),
    ))
    .context("streaming failed");
    let overflow = is_context_overflow_error(&error);
    // @step Then compaction is requested
    assert!(overflow);
}
