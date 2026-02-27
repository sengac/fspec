//! Feature: spec/features/openai-compatible-local-model-support-vllm-ollama.feature
//!
//! Tests for NAPI binding: models_list_local_openai
//! This test file validates the NAPI integration layer for local model listing.
//!
//! NOTE: These tests require the real NAPI bindings (not noop stubs),
//! so they are gated behind `not(feature = "noop")`.

#[cfg(all(test, not(feature = "noop")))]
mod napi_binding_exposes_local_model_listing {
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};
    
    #[tokio::test]
    async fn test_models_list_local_openai_function() {
        // @step Given I have a local server running at "http://localhost:8888"
        let mock_server = MockServer::start().await;
        
        // @step And the server's /v1/models endpoint returns models "model-a" and "model-b"
        let models_response = serde_json::json!({
            "object": "list",
            "data": [
                {"id": "model-a", "object": "model", "created": 1234567890, "owned_by": "local"},
                {"id": "model-b", "object": "model", "created": 1234567891, "owned_by": "local"}
            ]
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&models_response))
            .expect(1)
            .mount(&mock_server)
            .await;
        
        // @step When I call the NAPI function models_list_local_openai("http://localhost:8888")
        let base_url = mock_server.uri();
        
        // NOTE: This test will fail until models_list_local_openai is implemented
        // The function should be exposed via NAPI and call OpenAIProvider::list_local_models internally
        let result = codelet_napi::models_list_local_openai(base_url).await;
        
        // @step Then the function should return an array of model IDs
        let models = result.expect("Should return model list");
        
        // @step And the array should contain "model-a" and "model-b"
        assert!(models.contains(&"model-a".to_string()));
        assert!(models.contains(&"model-b".to_string()));
        assert_eq!(models.len(), 2);
    }
}
