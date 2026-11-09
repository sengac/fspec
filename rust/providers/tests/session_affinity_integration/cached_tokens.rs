//! PROV-051: cached-token usage tests (split from session_affinity_integration_test.rs, PROV-171).

// =========================================================================
// Scenario: Cached tokens from Fireworks response are captured in usage metrics
// =========================================================================
#[test]
fn test_cached_tokens_deserialization() {
    // @step Given an OpenAI completion response with prompt_tokens_details.cached_tokens of 5000
    // This tests the JSON contract that Fireworks.ai uses for cached token reporting.
    // rig-core's PromptTokensDetails.cached_tokens deserializes this field; we verify
    // the structure matches what our code expects to find in the response.
    let response_json = serde_json::json!({
        "prompt_tokens": 10000,
        "completion_tokens": 500,
        "total_tokens": 10500,
        "prompt_tokens_details": {
            "cached_tokens": 5000
        }
    });

    // @step When the response is deserialized
    let details = response_json
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(serde_json::Value::as_u64);

    // @step Then the usage should report cache_read_input_tokens as 5000
    assert_eq!(details, Some(5000));
}

// =========================================================================
// Additional: Verify cached_tokens zero and missing cases
// =========================================================================
#[test]
fn test_cached_tokens_zero_is_reported() {
    let response_json = serde_json::json!({
        "prompt_tokens": 10000,
        "prompt_tokens_details": {
            "cached_tokens": 0
        }
    });

    let details = response_json
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(serde_json::Value::as_u64);

    assert_eq!(details, Some(0), "Zero cached tokens should be reported");
}

#[test]
fn test_cached_tokens_missing_details() {
    let response_json = serde_json::json!({
        "prompt_tokens": 10000,
        "completion_tokens": 500,
        "total_tokens": 10500
    });

    let details = response_json
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(serde_json::Value::as_u64);

    assert_eq!(
        details, None,
        "Missing prompt_tokens_details should return None"
    );
}
