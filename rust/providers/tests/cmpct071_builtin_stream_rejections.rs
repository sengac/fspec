use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::http_client::Error as HttpError;
use rig::providers::{anthropic, gemini, openai};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Feature: spec/features/builtin-provider-structured-stream-rejection.feature
#[tokio::test]
async fn openai_chat_preserves_physical_http_rejection_evidence() -> TestResult {
    // @step Given a selected OpenAI Chat model whose physical endpoint rejects requests
    for (status, body) in [
        (
            400,
            r#"{"error":{"code":"context_length_exceeded","message":"input is too long"}}"#,
        ),
        (
            401,
            r#"{"error":{"code":"invalid_api_key","message":"context_length_exceeded is not the code"}}"#,
        ),
        (400, "context_length_exceeded is only unstructured text"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let model = openai::CompletionsClient::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("selected-model");
        let request = CompletionRequestBuilder::new(model.clone(), "retained suffix")
            .max_tokens(200)
            .build();
        // @step When requests receive structured overflow, authentication, and plain text rejection bodies
        let mut response = model.stream(request).await?;
        let events = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut events = Vec::new();
            while let Some(event) = response.next().await {
                events.push(event);
            }
            events
        })
        .await?;
        // @step Then each stream exposes the original typed HTTP status and exact body
        assert!(
            matches!(events.first(), Some(Err(CompletionError::HttpError(
                HttpError::InvalidStatusCodeWithMessage(actual_status, actual_body)
            ))) if actual_status.as_u16() == status && actual_body == body),
            "physical rejection lost its typed status or body"
        );
        // @step And no rejected stream emits a successful final response
        assert_eq!(
            events.len(),
            1,
            "a rejected stream must stop after its error"
        );
        server.verify().await;
    }
    Ok(())
}

async fn assert_rejection<M: CompletionModel>(
    model: M,
    server: &MockServer,
    endpoint: &str,
) -> TestResult {
    for (status, body) in [
        (
            400,
            r#"{"error":{"code":"context_length_exceeded","message":"input is too long"}}"#,
        ),
        (
            401,
            r#"{"error":{"code":"invalid_api_key","message":"context_length_exceeded is not the code"}}"#,
        ),
        (400, "context_length_exceeded is only unstructured text"),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(1)
            .mount(server)
            .await;
        let request = CompletionRequestBuilder::new(model.clone(), "retained suffix")
            .max_tokens(200)
            .build();
        let mut response = model.stream(request).await?;
        let events = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut events = Vec::new();
            while let Some(event) = response.next().await {
                events.push(event);
            }
            events
        })
        .await?;
        assert!(
            matches!(events.first(), Some(Err(CompletionError::HttpError(
                HttpError::InvalidStatusCodeWithMessage(actual_status, actual_body)
            ))) if actual_status.as_u16() == status && actual_body == body),
            "physical rejection lost its typed status or body"
        );
        assert_eq!(
            events.len(),
            1,
            "a rejected stream must stop after its error"
        );
        server.verify().await;
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-structured-stream-rejection.feature
#[tokio::test]
async fn gemini_preserves_physical_http_rejection_evidence() -> TestResult {
    // @step Given a selected Gemini model whose physical endpoint rejects requests
    let server = MockServer::start().await;
    let model = gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("selected-model");
    // @step When requests receive structured overflow, authentication, and plain text rejection bodies
    // @step Then each stream exposes the original typed HTTP status and exact body
    // @step And no rejected stream emits a successful final response
    assert_rejection(
        model,
        &server,
        "/v1beta/models/selected-model:streamGenerateContent",
    )
    .await
}

/// Feature: spec/features/builtin-provider-structured-stream-rejection.feature
#[tokio::test]
async fn openai_responses_preserves_physical_http_rejection_evidence() -> TestResult {
    // @step Given a selected OpenAI Responses model whose physical endpoint rejects requests
    let server = MockServer::start().await;
    let model = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("selected-model");
    // @step When requests receive structured overflow, authentication, and plain text rejection bodies
    // @step Then each stream exposes the original typed HTTP status and exact body
    // @step And no rejected stream emits a successful final response
    assert_rejection(model, &server, "/responses").await
}

/// Feature: spec/features/builtin-provider-structured-stream-rejection.feature
#[tokio::test]
async fn anthropic_preserves_physical_http_rejection_evidence() -> TestResult {
    // @step Given a selected Anthropic model whose physical endpoint rejects requests
    let server = MockServer::start().await;
    let model = anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("selected-model");
    // @step When requests receive structured overflow, authentication, and plain text rejection bodies
    // @step Then each stream exposes the original typed HTTP status and exact body
    // @step And no rejected stream emits a successful final response
    assert_rejection(model, &server, "/v1/messages").await
}
