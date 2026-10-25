use super::*;

#[test]
fn test_partial_usage_token_calculation() {
    let usage = PartialUsage {
        total_token_count: 80,
        cached_content_token_count: Some(20),
        candidates_token_count: Some(30),
        thoughts_token_count: Some(10),
        prompt_token_count: 40,
    };
    let token_usage = usage.token_usage().expect("consistent Gemini measurements");
    assert_eq!(token_usage.input_tokens, 20);
    assert_eq!(token_usage.output_tokens, 30);
    assert_eq!(token_usage.total_tokens, 80);
    assert_eq!(token_usage.cache_read_input_tokens, Some(20));
    assert_eq!(token_usage.reasoning_tokens, Some(10));
}

#[test]
fn test_partial_usage_with_missing_counts() {
    let usage = PartialUsage {
        total_token_count: 50,
        cached_content_token_count: None,
        candidates_token_count: Some(30),
        thoughts_token_count: None,
        prompt_token_count: 20,
    };
    let token_usage = usage
        .token_usage()
        .expect("consistent Gemini measurements without optional counts");
    assert_eq!(token_usage.input_tokens, 20);
    assert_eq!(token_usage.output_tokens, 30);
    assert_eq!(token_usage.total_tokens, 50);
}

#[test]
fn test_streaming_completion_response_token_usage() {
    let response = StreamingCompletionResponse {
        usage_metadata: PartialUsage {
            total_token_count: 150,
            cached_content_token_count: None,
            candidates_token_count: Some(75),
            thoughts_token_count: None,
            prompt_token_count: 75,
        },
        stop_reason: None,
    };
    let token_usage = response
        .token_usage()
        .expect("consistent streaming Gemini measurements");
    assert_eq!(token_usage.input_tokens, 75);
    assert_eq!(token_usage.output_tokens, 75);
    assert_eq!(token_usage.total_tokens, 150);
}

#[test]
fn test_streaming_completion_response_stop_reason_max_tokens() {
    let response = StreamingCompletionResponse {
        usage_metadata: PartialUsage {
            total_token_count: 100,
            cached_content_token_count: None,
            candidates_token_count: Some(50),
            thoughts_token_count: None,
            prompt_token_count: 50,
        },
        stop_reason: Some("max_tokens".to_string()),
    };
    assert_eq!(response.stop_reason(), Some("max_tokens"));
}

#[test]
fn test_streaming_completion_response_stop_reason_end_turn() {
    let response = StreamingCompletionResponse {
        usage_metadata: PartialUsage {
            total_token_count: 100,
            cached_content_token_count: None,
            candidates_token_count: Some(50),
            thoughts_token_count: None,
            prompt_token_count: 50,
        },
        stop_reason: Some("end_turn".to_string()),
    };
    assert_eq!(response.stop_reason(), Some("end_turn"));
}

#[test]
fn test_streaming_completion_response_stop_reason_none() {
    let response = StreamingCompletionResponse {
        usage_metadata: PartialUsage::default(),
        stop_reason: None,
    };
    assert_eq!(response.stop_reason(), None);
}

#[test]
fn test_finish_reason_to_stop_reason_mapping() {
    let cases = [
        (FinishReason::Stop, "end_turn"),
        (FinishReason::MaxTokens, "max_tokens"),
        (FinishReason::Safety, "content_filter"),
        (FinishReason::Recitation, "content_filter"),
        (FinishReason::Language, "content_filter"),
        (FinishReason::Blocklist, "content_filter"),
        (FinishReason::ProhibitedContent, "content_filter"),
        (FinishReason::Spii, "content_filter"),
        (FinishReason::MalformedFunctionCall, "end_turn"),
        (FinishReason::Other, "unknown"),
        (FinishReason::FinishReasonUnspecified, "unknown"),
    ];
    for (finish_reason, expected_stop_reason) in cases {
        let mapped = match &finish_reason {
            FinishReason::Stop => "end_turn",
            FinishReason::MaxTokens => "max_tokens",
            FinishReason::Safety
            | FinishReason::Recitation
            | FinishReason::Language
            | FinishReason::Blocklist
            | FinishReason::ProhibitedContent
            | FinishReason::Spii => "content_filter",
            FinishReason::MalformedFunctionCall => "end_turn",
            FinishReason::Other | FinishReason::FinishReasonUnspecified => "unknown",
        };
        assert_eq!(
            mapped, expected_stop_reason,
            "FinishReason::{finish_reason:?} should map to '{expected_stop_reason}'"
        );
    }
}
