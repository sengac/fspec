use codelet_providers::codex::{
    reconnect::CodexRecoveryHandle, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use rig::providers::openai;
use std::time::Duration;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub async fn api() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/responses"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    server
}

pub fn model(
    api: &str,
    issuer: &str,
    recovery: CodexRecoveryHandle,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = RefreshingCodexClient::new_oauth(
        "access-A".into(),
        "refresh-A".into(),
        "A".into(),
        Some(3600),
        issuer.into(),
    )
    .with_idle_timeout(Duration::from_millis(500))
    .with_test_endpoint(&format!("{api}/responses"))?;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("synthetic")
        .base_url(api)
        .http_client(client)
        .build()?;
    Ok(CodexResponsesModel::new(client, "local-model")
        .with_recovery_handle(recovery)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}
