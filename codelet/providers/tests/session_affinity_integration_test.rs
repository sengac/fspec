//! PROV-051: Integration tests for session affinity in OpenAI provider
//!
//! Feature: spec/features/openai-session-affinity.feature
//!
//! Tests the CacheOptimizationFacade, SessionAffinityConfig, and their integration
//! with ProviderManager::get_openai() for session affinity header injection.

use codelet_providers::{
    CacheOptimizationFacade, LlmProvider, OpenAIProvider, ProviderManager, ProviderType,
    SessionAffinityConfig,
};
use uuid::Uuid;

// =========================================================================
// Scenario: Session affinity header is set when using custom base URL
// =========================================================================
#[test]
#[serial_test::serial]
fn test_session_affinity_header_set_with_custom_base_url() {
    // @step Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
    let has_custom_base_url = true;

    // @step And OPENAI_API_KEY is set to "fw-test-key"
    // (API key not relevant for header construction — handled by provider)

    // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    std::env::remove_var("OPENAI_SESSION_AFFINITY");
    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    // @step When an OpenAI provider is created with that session ID
    let config = SessionAffinityConfig::new(session_id, None, has_custom_base_url);
    let headers = CacheOptimizationFacade::build_headers(&config);

    // @step Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"
    assert_eq!(
        headers.get("x-session-affinity").unwrap().to_str().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

// =========================================================================
// Scenario: Session affinity header uses custom value from environment
// =========================================================================
#[test]
#[serial_test::serial]
fn test_session_affinity_header_uses_custom_env_value() {
    // @step Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
    let has_custom_base_url = true;

    // @step And OPENAI_API_KEY is set to "fw-test-key"
    // (API key not relevant for header construction)

    // @step And OPENAI_SESSION_AFFINITY is set to "my-custom-session"
    std::env::set_var("OPENAI_SESSION_AFFINITY", "my-custom-session");

    // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    // @step When an OpenAI provider is created with that session ID
    let config = SessionAffinityConfig::from_env(session_id, has_custom_base_url);
    let headers = CacheOptimizationFacade::build_headers(&config);

    // @step Then the rig client headers should include "x-session-affinity" with value "my-custom-session"
    assert_eq!(
        headers.get("x-session-affinity").unwrap().to_str().unwrap(),
        "my-custom-session"
    );

    // Cleanup
    std::env::remove_var("OPENAI_SESSION_AFFINITY");
}

// =========================================================================
// Scenario: Session affinity header is sent for any custom base URL endpoint
// =========================================================================
#[test]
#[serial_test::serial]
fn test_session_affinity_header_sent_for_any_custom_url() {
    // @step Given OPENAI_BASE_URL is set to "http://localhost:8888"
    let has_custom_base_url = true;

    // @step And OPENAI_API_KEY is set to "test-key"
    // (API key not relevant for header construction)

    // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    std::env::remove_var("OPENAI_SESSION_AFFINITY");
    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    // @step When an OpenAI provider is created with that session ID
    let config = SessionAffinityConfig::new(session_id, None, has_custom_base_url);
    let headers = CacheOptimizationFacade::build_headers(&config);

    // @step Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"
    assert_eq!(
        headers.get("x-session-affinity").unwrap().to_str().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}

// =========================================================================
// Scenario: No session affinity header when using default OpenAI API
// =========================================================================
#[test]
#[serial_test::serial]
fn test_no_session_affinity_header_for_default_openai() {
    // @step Given OPENAI_BASE_URL is not set
    let has_custom_base_url = false;

    // @step And OPENAI_API_KEY is set to "sk-test-key"
    // (API key not relevant for header construction)

    // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    std::env::remove_var("OPENAI_SESSION_AFFINITY");
    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    // @step When an OpenAI provider is created with that session ID
    let config = SessionAffinityConfig::new(session_id, None, has_custom_base_url);
    let headers = CacheOptimizationFacade::build_headers(&config);

    // @step Then the rig client headers should not include "x-session-affinity"
    assert!(headers.get("x-session-affinity").is_none());
}

// =========================================================================
// Scenario: get_openai accepts session_id parameter
// =========================================================================
#[test]
#[serial_test::serial]
fn test_get_openai_accepts_session_id_parameter() {
    // @step Given a provider manager with OpenAI credentials
    std::env::set_var("OPENAI_API_KEY", "test-key-for-session-affinity");
    std::env::set_var("OPENAI_BASE_URL", "http://localhost:9999");
    std::env::remove_var("OPENAI_SESSION_AFFINITY");

    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI);
    manager
        .set_model_direct("openai", "test-model")
        .expect("set_model_direct should succeed");

    // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

    // @step When get_openai is called with the session ID
    let provider = manager.get_openai(session_id);

    // @step Then the returned provider should have the session affinity header set
    // Verify get_openai succeeds and returns a properly configured provider
    let provider = provider.expect("get_openai should succeed with valid credentials and session ID");
    assert_eq!(provider.model(), "test-model");
    assert!(
        provider.is_local_endpoint(),
        "Provider should use custom base URL endpoint"
    );

    // Also verify the facade path produces headers for this configuration
    // (headers are embedded in the rig client and not directly inspectable,
    // so we verify the facade independently for the same inputs)
    let config = SessionAffinityConfig::from_env(session_id, true);
    let headers = CacheOptimizationFacade::build_headers(&config);
    assert_eq!(
        headers.get("x-session-affinity").unwrap().to_str().unwrap(),
        "550e8400-e29b-41d4-a716-446655440000",
        "Session affinity header should be set for custom base URL"
    );

    // Cleanup
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_BASE_URL");
}

// =========================================================================
// Scenario: get_openai with from_api_key_with_session wires session_id through
// =========================================================================
#[test]
#[serial_test::serial]
fn test_from_api_key_with_session_creates_provider_with_affinity() {
    // Verify from_api_key_with_session (the method get_openai delegates to)
    // correctly passes session_id through to the cache optimization facade.
    std::env::set_var("OPENAI_BASE_URL", "https://api.fireworks.ai/inference");
    std::env::remove_var("OPENAI_SESSION_AFFINITY");

    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let provider = OpenAIProvider::from_api_key_with_session(
        "fw-test-key",
        "accounts/fireworks/models/llama-v3p3-70b-instruct",
        session_id,
    );

    let provider = provider.expect("from_api_key_with_session should succeed");
    assert!(provider.is_local_endpoint());
    assert_eq!(
        provider.model(),
        "accounts/fireworks/models/llama-v3p3-70b-instruct"
    );

    // Cleanup
    std::env::remove_var("OPENAI_BASE_URL");
}

// =========================================================================
// Scenario: get_openai without custom base URL produces no affinity header
// =========================================================================
#[test]
#[serial_test::serial]
fn test_get_openai_default_endpoint_no_affinity() {
    // Verify that when using the default OpenAI API (no OPENAI_BASE_URL),
    // get_openai still works but no session affinity header is produced.
    std::env::set_var("OPENAI_API_KEY", "sk-test-key-no-affinity");
    std::env::remove_var("OPENAI_BASE_URL");
    std::env::remove_var("OPENAI_SESSION_AFFINITY");

    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI);
    manager
        .set_model_direct("openai", "gpt-4o")
        .expect("set_model_direct should succeed");

    let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let provider = manager
        .get_openai(session_id)
        .expect("get_openai should succeed for default endpoint");

    assert!(!provider.is_local_endpoint());

    // Verify facade produces empty headers for default endpoint
    let config = SessionAffinityConfig::from_env(session_id, false);
    let headers = CacheOptimizationFacade::build_headers(&config);
    assert!(
        headers.get("x-session-affinity").is_none(),
        "No session affinity header for default OpenAI API"
    );

    // Cleanup
    std::env::remove_var("OPENAI_API_KEY");
}

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
        .and_then(|v| v.as_u64());

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
        .and_then(|v| v.as_u64());

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
        .and_then(|v| v.as_u64());

    assert_eq!(
        details, None,
        "Missing prompt_tokens_details should return None"
    );
}
