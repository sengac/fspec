//! Feature: spec/features/codex-rejection-diagnostics.feature
#[path = "fixtures/cmpct076.rs"]
pub mod fixture;

use codelet_providers::codex::{
    diagnostics::{EvidenceState, RejectionClass, MAX_DIAGNOSTIC_BYTES},
    refreshing_client::RefreshingCodexClient,
    CodexResponsesModel,
};
use fixture::{model_diagnostic, TestResult};
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use serde_json::json;
use std::time::Duration;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn direct_error(url: &str) -> Result<CompletionError, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(
            RefreshingCodexClient::new_api_key().with_idle_timeout(Duration::from_millis(100)),
        )
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model");
    let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
    model
        .completion(request)
        .await
        .err()
        .ok_or_else(|| "rejection completed".into())
}

#[tokio::test]
async fn scenario_direct_completion_rejections_retain_bounded_private_typed_evidence() -> TestResult
{
    // @step Given direct Codex HTTP rejections with context unrelated malformed and oversized evidence
    for (status, body, class, state) in [
        (
            400,
            json!({"error":{"code":"context_length_exceeded","message":"test-key"}}).to_string(),
            RejectionClass::ContextLengthExceeded,
            EvidenceState::Complete,
        ),
        (
            413,
            json!({"error":{"code":"bad_request","message":"test-key"}}).to_string(),
            RejectionClass::Other,
            EvidenceState::Complete,
        ),
        (
            429,
            "not JSON test-key".into(),
            RejectionClass::Unknown,
            EvidenceState::Complete,
        ),
        (
            503,
            format!("test-key{}", "x".repeat(20_000)),
            RejectionClass::Unknown,
            EvidenceState::Truncated,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("x-request-id", "direct-request")
                    .insert_header("retry-after", "0")
                    .set_body_string(body),
            )
            .expect(1)
            .mount(&server)
            .await;
        // @step When the real model requests a nonstreaming completion
        let error = direct_error(&server.uri()).await?;
        // @step Then typed bounded diagnostics preserve status identity and declared classification without credential disclosure
        let rejection = model_diagnostic(&error)?;
        assert_eq!(rejection.status, Some(status));
        assert_eq!(rejection.class, class);
        assert_eq!(rejection.state, state);
        assert_eq!(rejection.attempts, Some(1));
        assert_eq!(rejection.request_id.as_deref(), Some("direct-request"));
        assert_eq!(rejection.retry_after.as_deref(), Some("0"));
        assert!(rejection.body.len() <= MAX_DIAGNOSTIC_BYTES);
        assert!(!format!("{error} {error:?}").contains("test-key"));
        // @step And the rejected request is not replayed
        server.verify().await;
    }
    Ok(())
}

#[tokio::test]
async fn scenario_direct_completion_rejection_capture_obeys_its_deadline() -> TestResult {
    // @step Given a direct completion rejection whose body never finishes
    let server = fixture::StalledRejection::start().await?;
    // @step When the real model requests a nonstreaming completion
    let error = tokio::time::timeout(Duration::from_secs(2), direct_error(&server.url)).await??;
    // @step Then typed incomplete evidence returns before the harness deadline without replay
    let rejection = model_diagnostic(&error)?;
    assert_eq!(rejection.status, Some(400));
    assert_eq!(rejection.state, EvidenceState::DeadlineExceeded);
    assert_eq!(rejection.class, RejectionClass::Unknown);
    assert_eq!(rejection.attempts, Some(1));
    Ok(())
}
