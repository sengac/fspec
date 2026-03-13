use super::*;
use rig::agent::MultiTurnStreamItem;

#[derive(Clone)]
struct TestStreamResponse;

impl rig::completion::GetTokenUsage for TestStreamResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        None
    }
}

#[test]
fn codex_requires_streaming_execution_path() {
    assert!(provider_uses_streaming_execution("codex"));
}

#[test]
fn non_codex_providers_remain_non_streaming() {
    for provider in ["claude", "openai", "gemini", "zai"] {
        assert!(!provider_uses_streaming_execution(provider));
    }
}

#[tokio::test]
async fn collect_final_response_from_stream_uses_final_response_only() {
    let stream = futures::stream::iter(vec![
        Ok(MultiTurnStreamItem::StreamAssistantItem(
            rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text("intermediate chunk"),
        )),
        Ok(MultiTurnStreamItem::final_response(
            "final synthesized answer",
            rig::completion::Usage::new(),
        )),
    ]);

    let collected = collect_final_response_from_stream(stream)
        .await
        .expect("stream should produce final response");

    assert_eq!(collected, "final synthesized answer");
}

#[tokio::test]
async fn collect_final_response_from_stream_errors_when_final_missing() {
    let stream = futures::stream::iter(vec![Ok(MultiTurnStreamItem::StreamAssistantItem(
        rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text("chunk without final"),
    ))]);

    let result = collect_final_response_from_stream(stream).await;
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected missing final response error")
            .contains("missing final response")
    );
}

#[tokio::test]
async fn collect_final_response_from_stream_propagates_stream_errors() {
    let stream = futures::stream::iter(vec![Err::<MultiTurnStreamItem<TestStreamResponse>, anyhow::Error>(
        anyhow::anyhow!("stream blew up"),
    )]);

    let result = collect_final_response_from_stream(stream).await;
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected stream error")
            .contains("stream blew up")
    );
}
