use super::{environment::TestResult, support};
use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::{completion::CompletionModel, streaming::StreamedAssistantContent};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn assert_in_flight_reset_bills_without_publication() -> TestResult {
    let server = MockServer::start().await;
    let history = CodexHistory::default();
    let owner = history.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            assert!(owner.transition_generation(1).is_ok());
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(support::terminal(
                    support::output(),
                    "completed",
                    false,
                    None,
                ))
        })
        .expect(1)
        .mount(&server)
        .await;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("synthetic-loopback-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model")
        .with_history(history.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100000),
            output_reservation: Some(1000),
        });
    let mut stream = model.stream(support::request(&model)).await?;
    let mut usage = Vec::new();
    let mut errors = Vec::new();
    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamedAssistantContent::Usage(tokens)) => usage.push(tokens.total_tokens),
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_)) => {
                return Err("reset response published stale completion".into());
            }
            _ => {}
        }
    }
    assert_eq!(usage, [20]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("generation"));
    let current = history.snapshot()?.ok_or("reset checkpoint missing")?;
    assert_eq!(current.generation, 1);
    assert!(current.items.is_empty());
    server.verify().await;
    Ok(())
}
