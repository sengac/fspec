//! Feature: spec/features/codex-metadata-safety-status.feature

use codelet_providers::codex::{
    metadata::CodexMetadataHandle, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[path = "fixtures/attempt_events.rs"]
mod attempt_events;
#[path = "fixtures/prov148.rs"]
pub mod fixtures;
use fixtures::{event, terminal, text, TestResult};

async fn model(
    body: String,
    headers: &[(&str, &str)],
    handle: CodexMetadataHandle,
) -> Result<(MockServer, CodexResponsesModel), Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    let mut response = ResponseTemplate::new(200)
        .insert_header("Content-Type", "text/event-stream")
        .set_body_string(body);
    for (name, value) in headers {
        response = response.insert_header(*name, *value);
    }
    Mock::given(method("POST"))
        .respond_with(response)
        .mount(&server)
        .await;
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("private-credential")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok((
        server,
        CodexResponsesModel::new(client, "selected-model")
            .with_request_admission(AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(1000),
            })
            .with_metadata_handle(handle),
    ))
}

#[tokio::test]
async fn http_metadata_reaches_session_diagnostics_before_text() -> TestResult {
    // @step Given a Codex response with mixed-case diagnostic rate-limit and opaque headers
    let handle = CodexMetadataHandle::default();
    let (_server, model) = model(
        [text(), terminal("completed", None, None)].concat(),
        &[
            ("X-ReQuEsT-Id", "req-test"),
            ("OpenAI-Model", "serving-model"),
            ("X-Models-Etag", "private-etag"),
            ("X-Reasoning-Included", "false"),
            ("X-Codex-Turn-State", "private-routing-state"),
            ("X-Codex-Primary-Used-Percent", "12.5"),
            ("X-Codex-Primary-Window-Minutes", "300"),
            ("X-Codex-Primary-Reset-At", "999"),
        ],
        handle.clone(),
    )
    .await?;
    // @step When the response streams text and completes
    let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
    let mut stream = model
        .stream(request)
        .await?
        .filter(attempt_events::is_content);
    assert!(matches!(
        stream.next().await.transpose()?,
        Some(StreamedAssistantContent::Text(_))
    ));
    // @step Then sanitized request and serving-model status reaches the observer before text
    let status = handle.status();
    assert_eq!(status.request_id.as_deref(), Some("req-test"));
    assert_eq!(status.server_model.as_deref(), Some("serving-model"));
    // @step And accounting etag rate limits and private turn state remain available
    assert!(status.reasoning_included);
    assert_eq!(
        status.rate_limits["codex"]
            .primary
            .as_ref()
            .map(|window| window.used_percent),
        Some(12.5)
    );
    assert_eq!(handle.models_etag().as_deref(), Some("private-etag"));
    assert_eq!(
        handle.turn_state().as_deref(),
        Some("private-routing-state")
    );
    assert!(!format!("{handle:?} {status:?}").contains("private-"));
    while stream.next().await.transpose()?.is_some() {}
    Ok(())
}

#[tokio::test]
async fn event_headers_override_top_level_metadata_without_duplicating_output() -> TestResult {
    // @step Given conflicting top-level and response headers and a rate-limit update
    let handle = CodexMetadataHandle::default();
    let metadata = event(
        json!({"type":"response.metadata", "headers":{"openai-model":"wrong-model","x-request-id":"wrong-id"}, "response":{"headers":{"X-OpenAI-Model":["right-model"],"X-Request-ID":"right-id","X-Codex-Turn-State":"private-event-state"}}}),
    );
    let rates = event(
        json!({"type":"codex.rate_limits","metered_limit_name":"codex_other","rate_limits":{"primary":{"used_percent":42,"window_minutes":60,"reset_at":123}}}),
    );
    let (_server, model) = model(
        [metadata, rates, text(), terminal("completed", None, None)].concat(),
        &[],
        handle.clone(),
    )
    .await?;
    // @step When the Codex stream consumes metadata and completion events
    let observed = fixtures::collect(model).await?;
    // @step Then response headers win and usage and text are delivered exactly once
    assert_eq!(handle.status().server_model.as_deref(), Some("right-model"));
    assert_eq!(handle.status().request_id.as_deref(), Some("right-id"));
    assert_eq!(handle.turn_state().as_deref(), Some("private-event-state"));
    assert_eq!(
        handle.status().rate_limits["codex_other"]
            .primary
            .as_ref()
            .map(|window| window.used_percent),
        Some(42.0)
    );
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.finals.len(), 1);
    assert_eq!(observed.usage.len(), 1);
    Ok(())
}

#[tokio::test]
async fn safety_buffering_updates_without_delaying_text() -> TestResult {
    // @step Given buffering notifications and advisory faster-model headers
    let handle = CodexMetadataHandle::default();
    let start = event(
        json!({"type":"response.metadata","metadata":{"type":"safety_buffering","use_cases":["private-case"],"reasons":["private-reason"]}}),
    );
    let update = event(
        json!({"type":"response.metadata","safety_buffering":{"use_cases":[],"reasons":[],"retry_model":"advisory-model"}}),
    );
    let clear = event(json!({"type":"response.metadata","safety_buffering":false}));
    let (server, model) = model(
        [
            start,
            text(),
            update,
            text(),
            clear,
            text(),
            terminal("completed", None, None),
        ]
        .concat(),
        &[("x-codex-safety-buffering-faster-model", "header-advice")],
        handle.clone(),
    )
    .await?;
    // @step When output arrives between buffering updates and an explicit false
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?
        .filter(attempt_events::is_content);
    for (active, suggestion) in [
        (false, "header-advice"),
        (false, "advisory-model"),
        (false, "advisory-model"),
    ] {
        assert!(matches!(
            stream.next().await.transpose()?,
            Some(StreamedAssistantContent::Text(_))
        ));
        // @step Then output dismisses buffering and the false is ignored
        assert_eq!(handle.status().buffering, active);
        assert_eq!(handle.status().faster_model.as_deref(), Some(suggestion));
    }
    while stream.next().await.transpose()?.is_some() {}
    // @step And the selected model never changes
    let requests = server.received_requests().await.ok_or("missing requests")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].body_json::<serde_json::Value>()?["model"],
        "selected-model"
    );
    Ok(())
}

#[tokio::test]
async fn verification_and_moderation_expose_only_sanitized_diagnostics() -> TestResult {
    // @step Given known and unknown verification names and private moderation payloads
    let handle = CodexMetadataHandle::default();
    let metadata = event(
        json!({"type":"response.metadata","headers":{"authorization":"private-credential","x-codex-turn-state":"private-routing","openai-model":"bad\u{001b}[31m model"},"metadata":{"openai_verification_recommendation":["trusted_access_for_cyber","private-verification"],"openai_chatgpt_moderation_metadata":{"reason":"private-moderation"}},"encrypted_content":"private-reasoning"}),
    );
    let (_server, model) = model(
        [metadata, text(), terminal("completed", None, None)].concat(),
        &[],
        handle.clone(),
    )
    .await?;
    // @step When metadata reaches the Codex status observer
    fixtures::collect(model).await?;
    // @step Then only recognized verification and moderation presence are retained in diagnostics
    let status = handle.status();
    assert!(status.verification_required);
    assert!(status.moderation_received);
    // @step And credentials routing payloads and opaque reasoning never appear in diagnostics
    let rendered = format!("{handle:?} {status:?}");
    assert!(!rendered.contains("private-"));
    assert!(!rendered.contains('\u{001b}'));
    Ok(())
}

#[tokio::test]
async fn missing_unknown_and_malformed_optional_metadata_remain_compatible() -> TestResult {
    // @step Given Codex and ordinary Responses streams with absent or unknown optional metadata
    let metadata = event(
        json!({"type":"response.metadata","headers":27,"safety_buffering":null,"metadata":{"openai_verification_recommendation":false}}),
    );
    let body = [metadata, text(), terminal("completed", None, None)].concat();
    let server = fixtures::Server::start(vec![body.clone(), body], false).await?;
    let (codex, ordinary) = server.models()?;
    let handle = CodexMetadataHandle::default();
    // @step When each stream completes
    let codex = fixtures::collect(codex.with_metadata_handle(handle.clone())).await?;
    let ordinary = fixtures::collect(ordinary).await?;
    // @step Then normal text and existing usage remain unchanged
    assert_eq!(codex.text, ordinary.text);
    assert_eq!(codex.finals, ordinary.finals);
    // @step And only the Codex stream can update Codex status
    assert_eq!(handle.status(), Default::default());
    Ok(())
}

#[tokio::test]
async fn ending_or_cancelling_a_stream_clears_transient_buffering() -> TestResult {
    // @step Given a Codex stream with active safety buffering
    for ending in [Some("completed"), Some("failed"), None] {
        let handle = CodexMetadataHandle::default();
        let body = [
            event(json!({"type":"response.metadata","safety_buffering":{"use_cases":[],"reasons":[]}})),
            event(json!({"type":"response.reasoning_summary_text.delta","item_id":"reason-153","output_index":1,"summary_index":0,"delta":"thinking"})),
            ending
                .map(|status| terminal(status, None, None))
                .unwrap_or_default(),
        ]
        .concat();
        let (_server, model) =
            model(body, &[("x-request-id", "req-retained")], handle.clone()).await?;
        let mut stream = model
            .stream(CompletionRequestBuilder::new(model.clone(), "Hello").build())
            .await?
            .filter(attempt_events::is_content);
        assert!(stream.next().await.transpose()?.is_some());
        assert!(handle.status().buffering);
        // @step When the stream completes fails or is dropped
        if ending.is_some() {
            while stream.next().await.is_some() {}
        }
        drop(stream);
        // @step Then buffering clears while safe diagnostic metadata remains available
        assert!(!handle.status().buffering);
        assert_eq!(handle.status().request_id.as_deref(), Some("req-retained"));
    }
    Ok(())
}

proptest::proptest! {
    #[test]
    fn arbitrary_optional_payloads_cannot_leak_through_status_rendering(secret in "[a-zA-Z0-9]{1,100}") {
        // @step Given arbitrary private optional metadata strings
        let secret = format!("PRIVATE-{secret}");
        let handle = CodexMetadataHandle::default();
        // @step When Codex metadata is parsed and rendered
        handle.apply_event(&json!({"type":"response.metadata","headers":{"authorization":secret,"x-codex-turn-state":secret},"metadata":{"openai_chatgpt_moderation_metadata":secret},"safety_buffering":{"use_cases":[secret],"reasons":[secret]}}));
        // @step Then private strings are absent from status and debug output
        let status = handle.status();
        let rendered = format!("{handle:?} {status:?}");
        proptest::prop_assert!(!rendered.contains(&secret));
    }
}
