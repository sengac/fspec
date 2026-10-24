//! Feature: spec/features/model-discovery-http-errors.feature

use codelet_providers::{OpenAIProvider, ProviderError};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn scenario_discovery_preserves_proxy_prefixes_and_composes_one_version_suffix(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a local models endpoint behind a proxy prefix with bearer authentication
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/proxy/deployment/v1/models"))
        .and(header("Authorization", "Bearer fixture-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{"id": "configured-model"}]
        })))
        .expect(4)
        .mount(&server)
        .await;

    // @step When discovery uses bases with and without a trailing v1 and slash
    for suffix in ["", "/", "/v1", "/v1/"] {
        let base = format!("{}/proxy/deployment{suffix}", server.uri());
        let models =
            OpenAIProvider::list_local_models_with_auth(&base, Some("fixture-key")).await?;

        // @step Then every request preserves the prefix and uses exactly one v1 models path
        assert_eq!(models, ["configured-model"]);
    }
    server.verify().await;
    Ok(())
}

#[tokio::test]
async fn scenario_unsupported_model_listing_has_a_typed_nonretryable_error(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given local models endpoints return HTTP 404 and 405
    let server = MockServer::start().await;
    for status in [404, 405] {
        Mock::given(method("GET"))
            .and(path(format!("/status-{status}/v1/models")))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;

        // @step When authenticated model discovery requests each endpoint
        let error = OpenAIProvider::list_local_models_with_auth(
            &format!("{}/status-{status}", server.uri()),
            Some("fixture-key"),
        )
        .await
        .err()
        .ok_or("unsupported listing must return an error, not an empty successful response")?;

        // @step Then each result identifies unsupported discovery with its HTTP status
        assert!(matches!(
            error,
            ProviderError::ModelDiscoveryUnsupported { status: actual, .. } if actual == status
        ));

        // @step And the errors retain the provider identity and are not retryable
        assert_eq!(error.provider_name(), "openai");
        assert!(!error.is_retryable());
        assert_eq!(error.retry_after(), None);
    }
    Ok(())
}

#[tokio::test]
async fn scenario_other_discovery_errors_are_not_unsupported_listing(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given local models endpoints return authentication server and malformed response failures
    let server = MockServer::start().await;
    for (index, response) in [
        ResponseTemplate::new(401),
        ResponseTemplate::new(403),
        ResponseTemplate::new(500),
        ResponseTemplate::new(200).set_body_string("not-json"),
        ResponseTemplate::new(200).set_body_json(json!({"error": "missing data"})),
    ]
    .into_iter()
    .enumerate()
    {
        Mock::given(method("GET"))
            .and(path(format!("/case-{index}/v1/models")))
            .respond_with(response)
            .mount(&server)
            .await;

        // @step When authenticated model discovery requests each endpoint
        let error = OpenAIProvider::list_local_models_with_auth(
            &format!("{}/case-{index}", server.uri()),
            Some("fixture-key"),
        )
        .await
        .err()
        .ok_or("failed model listing must remain an error")?;

        // @step Then none of the errors is classified as unsupported discovery
        assert!(!matches!(
            error,
            ProviderError::ModelDiscoveryUnsupported { .. }
        ));
    }
    Ok(())
}
