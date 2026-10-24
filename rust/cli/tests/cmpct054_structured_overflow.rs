//! Feature: spec/features/structured-terminal-overflow-classification.feature
use codelet_cli::interactive::is_context_overflow_error;
use rig::completion::CompletionError;

fn http_error(body: serde_json::Value) -> anyhow::Error {
    let error = rig::http_client::Error::InvalidStatusCodeWithMessage(
        http::StatusCode::BAD_REQUEST,
        body.to_string(),
    );
    anyhow::Error::new(CompletionError::HttpError(error))
        .context("streaming request")
        .context("Agent error")
}

#[test]
fn structured_http_overflow_survives_streaming_wrappers() {
    // @step Given a provider HTTP rejection with a context overflow code
    for code in [
        "context_length_exceeded",
        "prompt_too_long",
        "context_window_exceeded",
    ] {
        let body = serde_json::json!({"error":{"code":code,"message":"rejected"}});
        // @step When streaming and agent context wrap the original rejection
        let error = http_error(body);
        // @step Then terminal recovery recognizes the overflow structurally
        assert!(is_context_overflow_error(&error), "{code}: {error:?}");
    }
}

#[test]
fn http_diagnostic_wording_does_not_authorize_compaction() {
    // @step Given a provider HTTP rejection without a context overflow code
    for code in [
        "rate_limit_exceeded",
        "invalid_api_key",
        "invalid_request_error",
    ] {
        // @step When its diagnostic text mentions context overflow
        let error = http_error(serde_json::json!({"error":{
            "code":code,"message":"context_length_exceeded: prompt is too long"
        }}));
        // @step Then terminal recovery does not classify it as overflow
        assert!(!is_context_overflow_error(&error), "{code}: {error:?}");
    }
}

#[test]
fn bare_text_cannot_impersonate_a_provider_rejection() {
    // @step Given an untyped error containing a serialized overflow rejection
    let error = anyhow::Error::msg(r#"{"error":{"code":"context_length_exceeded"}}"#);
    // @step When terminal recovery classifies the error
    let classified = is_context_overflow_error(&error);
    // @step Then terminal recovery does not classify it as overflow
    assert!(!classified);
}

#[test]
fn provider_overflow_code_survives_agent_context() {
    // @step Given a provider completion rejection with a context overflow code
    let error = CompletionError::ProviderError(
        serde_json::json!({"error":{"code":"context_length_exceeded"}}).to_string(),
    );
    // @step When agent context wraps the original rejection
    let error = anyhow::Error::new(error).context("Agent error");
    // @step Then terminal recovery recognizes the overflow structurally
    assert!(is_context_overflow_error(&error));
}
