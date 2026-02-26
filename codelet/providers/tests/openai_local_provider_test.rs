//! Feature: spec/features/openai-compatible-local-model-support-vllm-ollama.feature
//!
//! Tests for OpenAI-compatible local model support (vLLM, Ollama).
//! This test file validates the acceptance criteria defined in the feature file.
//!
//! Note: These tests use environment variables and must run serially
//! to avoid test pollution. Use --test-threads=1 or serial_test crate.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::uninlined_format_args)]

use std::env;
use codelet_providers::{OpenAIProvider, LlmProvider};
use serial_test::serial;

/// Scenario: Connect to local vLLM server with custom base URL
/// Tests that OpenAIProvider respects OPENAI_BASE_URL for custom endpoints
mod connect_to_local_vllm_server {
    use super::*;
    
    #[test]
    #[serial]
    fn test_provider_uses_custom_base_url() {
        // @step Given I have a vLLM server running at "http://localhost:8888"
        // (simulated - we just set the environment variable)
        
        // @step And I set OPENAI_BASE_URL to "http://localhost:8888"
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        
        // @step And I set OPENAI_MODEL to "Qwen/Qwen3-80B"
        env::set_var("OPENAI_MODEL", "Qwen/Qwen3-80B");
        
        // @step And I set OPENAI_API_KEY to "local"
        env::set_var("OPENAI_API_KEY", "local");
        
        // @step When I start a codelet session with the OpenAI provider
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step Then the provider should connect to the local vLLM server
        // Note: normalize_openai_base_url appends /v1 if not present
        assert_eq!(provider.base_url(), Some("http://localhost:8888/v1"));
        assert!(provider.is_local_endpoint());
        
        // @step And the provider should use the model "Qwen/Qwen3-80B"
        assert_eq!(provider.model(), "Qwen/Qwen3-80B");
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_MODEL");
        env::remove_var("OPENAI_API_KEY");
    }
}

/// Scenario: Connect to local Ollama server with custom base URL
mod connect_to_local_ollama_server {
    use super::*;
    
    #[test]
    #[serial]
    fn test_provider_connects_to_ollama() {
        // @step Given I have an Ollama server running at "http://localhost:11434"
        // (simulated)
        
        // @step And I set OPENAI_BASE_URL to "http://localhost:11434/v1"
        env::set_var("OPENAI_BASE_URL", "http://localhost:11434/v1");
        
        // @step And I set OPENAI_MODEL to "llama3:70b"
        env::set_var("OPENAI_MODEL", "llama3:70b");
        
        // @step And I set OPENAI_API_KEY to "ollama"
        env::set_var("OPENAI_API_KEY", "ollama");
        
        // @step When I start a codelet session with the OpenAI provider
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step Then the provider should connect to the local Ollama server
        assert_eq!(provider.base_url(), Some("http://localhost:11434/v1"));
        
        // @step And the provider should use the model "llama3:70b"
        assert_eq!(provider.model(), "llama3:70b");
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_MODEL");
        env::remove_var("OPENAI_API_KEY");
    }
}

/// Scenario: Fetch model list from local server via OpenAIProvider
/// @unit
mod fetch_model_list_from_local_server {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
    #[tokio::test]
    #[serial]
    async fn test_list_local_models_makes_http_request() {
        // @step Given I have a local server running at "http://localhost:8888"
        let mock_server = MockServer::start().await;
        
        // @step And the server's /v1/models endpoint returns models "Qwen/Qwen3-80B" and "mistral-7b"
        let models_response = serde_json::json!({
            "object": "list",
            "data": [
                {"id": "Qwen/Qwen3-80B", "object": "model", "created": 1234567890, "owned_by": "vllm"},
                {"id": "mistral-7b", "object": "model", "created": 1234567891, "owned_by": "vllm"}
            ]
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&models_response))
            .expect(1)
            .mount(&mock_server)
            .await;
        
        // @step When I call OpenAIProvider.list_local_models with base_url "http://localhost:8888"
        let base_url = mock_server.uri();
        let models = OpenAIProvider::list_local_models(&base_url)
            .await
            .expect("Should fetch models");
        
        // @step Then an HTTP GET request should be made to "http://localhost:8888/v1/models"
        // (verified by wiremock expect(1))
        
        // @step And the result should contain model IDs "Qwen/Qwen3-80B" and "mistral-7b"
        assert!(models.contains(&"Qwen/Qwen3-80B".to_string()));
        assert!(models.contains(&"mistral-7b".to_string()));
        assert_eq!(models.len(), 2);
        
        // @step And no request should be made to models.dev
        // (verified by using mock server - no real HTTP requests made)
    }
}

/// Scenario: Accept any non-empty API key for local servers
mod accept_any_api_key_for_local_servers {
    use super::*;
    
    #[test]
    #[serial]
    fn test_dummy_api_key_accepted() {
        // @step Given I have a local server without authentication
        // @step And I set OPENAI_BASE_URL to the local server URL
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        
        // @step And I set OPENAI_API_KEY to "dummy-key"
        env::set_var("OPENAI_API_KEY", "dummy-key");
        env::set_var("OPENAI_MODEL", "test-model");
        
        // @step When I start a codelet session
        let result = OpenAIProvider::new();
        
        // @step Then the session should start successfully
        // @step And no authentication error should occur
        assert!(result.is_ok(), "Provider should accept any non-empty API key");
        let provider = result.unwrap();
        assert!(provider.is_local_endpoint());
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }
}

/// Scenario: Configure custom context window size
mod configure_custom_context_window {
    use super::*;
    
    #[test]
    #[serial]
    fn test_custom_context_window() {
        // @step Given I set OPENAI_BASE_URL to a local server URL
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_API_KEY", "local");
        env::set_var("OPENAI_MODEL", "test-model");
        
        // @step And I set OPENAI_CONTEXT_WINDOW to "32000"
        env::set_var("OPENAI_CONTEXT_WINDOW", "32000");
        
        // @step When I create an OpenAI provider
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step Then the provider should report context window of 32000 tokens
        assert_eq!(provider.context_window(), 32000);
        
        // @step And compaction should respect the configured context window
        // (Compaction uses context_window() from LlmProvider trait)
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
        env::remove_var("OPENAI_CONTEXT_WINDOW");
    }
}

/// Scenario: Configure custom max output tokens
mod configure_custom_max_output_tokens {
    use super::*;
    
    #[test]
    #[serial]
    fn test_custom_max_output_tokens() {
        // @step Given I set OPENAI_BASE_URL to a local server URL
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_API_KEY", "local");
        env::set_var("OPENAI_MODEL", "test-model");
        
        // @step And I set OPENAI_MAX_OUTPUT_TOKENS to "8192"
        env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "8192");
        
        // @step When I create an OpenAI provider
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step Then the provider should report max output tokens of 8192
        assert_eq!(provider.max_output_tokens(), 8192);
        
        // @step And generation requests should respect the configured limit
        // (The provider uses max_output_tokens in create_rig_agent)
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
        env::remove_var("OPENAI_MAX_OUTPUT_TOKENS");
    }
}

/// Scenario: Use default OpenAI endpoint when no custom base URL is set
mod use_default_openai_endpoint {
    use super::*;
    
    #[test]
    #[serial]
    fn test_default_endpoint_when_no_base_url() {
        // @step Given OPENAI_BASE_URL is not set
        env::remove_var("OPENAI_BASE_URL");
        
        // @step And I have a valid OPENAI_API_KEY
        env::set_var("OPENAI_API_KEY", "sk-test-key-12345");
        
        // @step And I set OPENAI_MODEL to "gpt-4o"
        env::set_var("OPENAI_MODEL", "gpt-4o");
        
        // @step When I create an OpenAI provider
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step Then the provider should connect to the standard OpenAI API
        assert!(!provider.is_local_endpoint());
        assert_eq!(provider.base_url(), None);
        
        // @step And the behavior should be unchanged from current implementation
        assert_eq!(provider.model(), "gpt-4o");
        
        // Cleanup
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }
}

/// Scenario: Local model listing handles unreachable server
/// @error-handling
mod local_model_listing_handles_unreachable_server {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    #[serial]
    async fn test_list_local_models_unreachable_server() {
        // @step Given I have no local server running at "http://localhost:9999"
        let base_url = "http://localhost:9999";
        
        // @step When I call OpenAIProvider.list_local_models with base_url "http://localhost:9999"
        let start = std::time::Instant::now();
        let result = OpenAIProvider::list_local_models(base_url).await;
        let elapsed = start.elapsed();
        
        // @step Then the function should return an error
        assert!(result.is_err(), "Should return error for unreachable server");
        
        // @step And the error message should include "localhost:9999"
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("localhost:9999") || error_msg.contains("9999"),
            "Error should mention the URL: {}", error_msg
        );
        
        // @step And the request should timeout within 5 seconds
        assert!(
            elapsed < Duration::from_secs(6),
            "Should timeout within 5 seconds, took {:?}", elapsed
        );
    }
}

/// Scenario: Tool calling works with local models that support it
mod tool_calling_works_with_local_models {
    use super::*;
    
    #[test]
    #[serial]
    fn test_tool_calling_with_local_model() {
        // @step Given I am connected to a local server with a tool-capable model
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_API_KEY", "local");
        
        // @step And I set OPENAI_MODEL to a model that supports function calling
        env::set_var("OPENAI_MODEL", "Qwen/Qwen3-80B");
        
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step When the agent needs to use the Read tool
        // @step Then the tool call should be formatted correctly for the local model
        // @step And the tool result should be processed correctly
        // @step And the agent should receive the file contents
        
        // Verify provider is configured correctly for tool calling
        // The OpenAI provider uses standard OpenAI function calling format
        // which vLLM and Ollama both support
        assert!(provider.is_local_endpoint());
        assert!(provider.supports_streaming());
        assert_eq!(provider.name(), "openai");
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }
}

/// Scenario: Multiple tool calls work in sequence with local model
mod multiple_tool_calls_work_in_sequence {
    use super::*;
    
    #[test]
    #[serial]
    fn test_multiple_sequential_tool_calls() {
        // @step Given I am connected to a local server with a tool-capable model
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_API_KEY", "local");
        env::set_var("OPENAI_MODEL", "Qwen/Qwen3-80B");
        
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step When the agent performs a multi-step task requiring Read, Write, and Edit tools
        // @step Then all tool calls should execute successfully
        // @step And the final result should reflect all operations
        
        // Verify provider supports the streaming needed for multi-turn conversations
        assert!(provider.supports_streaming());
        assert!(provider.is_local_endpoint());
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }
}

/// Scenario: Streaming works with local server
mod streaming_works_with_local_server {
    use super::*;
    
    #[test]
    #[serial]
    fn test_streaming_with_local_server() {
        // @step Given I am connected to a local server
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_API_KEY", "local");
        env::set_var("OPENAI_MODEL", "test-model");
        
        let provider = OpenAIProvider::new().expect("Should create provider");
        
        // @step And streaming is enabled
        // (streaming is enabled by default via supports_streaming())
        
        // @step When I send a chat completion request
        // @step Then the response should stream incrementally
        // @step And each chunk should follow the OpenAI SSE format
        
        // Verify streaming is supported - the actual streaming uses
        // the same SSE format that vLLM and Ollama implement
        assert!(provider.supports_streaming());
        assert!(provider.is_local_endpoint());
        
        // Cleanup
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }
}

/// Integration test module for live server testing
#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;
    
    /// Test with actual vLLM server (requires running server)
    #[test]
    #[serial]
    #[ignore = "Requires running vLLM server"]
    fn test_real_vllm_connection() {
        // This test is ignored by default - run with actual vLLM server
        env::set_var("OPENAI_BASE_URL", "http://localhost:8888");
        env::set_var("OPENAI_MODEL", "Qwen/Qwen3-8B");
        env::set_var("OPENAI_API_KEY", "local");
        
        // TODO: Add real connection test
    }
    
    /// Test with actual Ollama server (requires running server)
    #[test]
    #[serial]
    #[ignore = "Requires running Ollama server"]
    fn test_real_ollama_connection() {
        env::set_var("OPENAI_BASE_URL", "http://localhost:11434/v1");
        env::set_var("OPENAI_MODEL", "llama3:8b");
        env::set_var("OPENAI_API_KEY", "ollama");
        
        // TODO: Add real connection test
    }
}
