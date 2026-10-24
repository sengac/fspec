//! Feature: spec/features/complete-token-event-forwarding.feature

use codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value;
use codelet_rpc_types::{StreamChunk, TokenTracker};
use serde_json::json;

#[test]
fn measured_token_fields_survive_json_forwarding() {
    // @step Given a typed token event with cache reasoning and cumulative billing measurements
    let tokens = TokenTracker {
        input_tokens: 1000,
        output_tokens: 100,
        cache_read_input_tokens: Some(400),
        cache_creation_input_tokens: Some(0),
        reasoning_tokens: Some(30),
        cumulative_billed_input: Some(4000),
        cumulative_billed_output: Some(500),
        tokens_per_second: Some(12.5),
    };
    // @step When the bridge projects the event to JSON
    let forwarded = stream_chunk_to_json_value(&StreamChunk::TokenUpdate { tokens });
    // @step Then every supported measurement survives unchanged
    assert_eq!(
        forwarded,
        json!({
            "type": "tokenUpdate",
            "tokens": {
                "inputTokens": 1000, "outputTokens": 100,
                "cacheReadInputTokens": 400, "cacheCreationInputTokens": 0,
                "reasoningTokens": 30, "cumulativeBilledInput": 4000,
                "cumulativeBilledOutput": 500, "tokensPerSecond": 12.5
            }
        })
    );
}

#[test]
fn unknown_token_fields_remain_unknown_in_json() {
    // @step Given a typed token event with absent cache reasoning and cumulative billing measurements
    let tokens = TokenTracker {
        input_tokens: 1000,
        output_tokens: 100,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        reasoning_tokens: None,
        cumulative_billed_input: None,
        cumulative_billed_output: None,
        tokens_per_second: None,
    };
    // @step When the bridge projects the event to JSON
    let forwarded = stream_chunk_to_json_value(&StreamChunk::TokenUpdate { tokens });
    // @step Then absent measurements remain null rather than fabricated zero
    let measured = forwarded["tokens"]
        .as_object()
        .expect("token projection object");
    for key in [
        "cacheReadInputTokens",
        "cacheCreationInputTokens",
        "reasoningTokens",
        "cumulativeBilledInput",
        "cumulativeBilledOutput",
        "tokensPerSecond",
    ] {
        assert_eq!(measured.get(key), Some(&serde_json::Value::Null), "{key}");
    }
}
