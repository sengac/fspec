//! Feature: spec/features/thinking-only-exhaustion-recovery.feature

#[path = "bug221/support.rs"]
mod support;

use codelet_cli::interactive::StreamEvent;
use support::{response, Fixture, TestResult, REASONING};
use wiremock::ResponseTemplate;

#[tokio::test]
#[serial_test::serial]
async fn thinking_only_length_stop_recovers_without_numerical_reasoning_usage() -> TestResult {
    // @step Given a streamed thinking-only response with absent reasoning usage and 4096 output tokens
    let mut fixture = Fixture::new(
        vec![
            response(REASONING, "", Some("length")),
            response("", "Task finished.", Some("length")),
        ],
        true,
        false,
    )
    .await?;
    // @step When the response stops for length and the next response provides an answer
    fixture.run().await?;
    // @step Then exactly one thinking exhaustion retry preserves the captured reasoning
    let requests = fixture.requests().await?;
    assert_eq!(requests.len(), 2);
    assert_eq!(fixture.retries(), 1);
    let retry: serde_json::Value = serde_json::from_slice(&requests[1].body)?;
    let retry_prompt = retry["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .map(|message| message["content"].to_string())
        .ok_or("retry prompt missing")?;
    assert!(retry_prompt.contains(REASONING), "{retry_prompt}");
    assert!(fixture
        .output
        .events
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| matches!(event, StreamEvent::Text(text) if text == "Task finished.")));
    // @step And provider usage is unchanged and no auto-continue nudge is consumed
    assert_eq!(fixture.session.token_tracker.output_tokens, 8192);
    assert_eq!(fixture.session.token_tracker.reasoning_tokens, 0);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn useful_answer_progress_and_ordinary_truncation_do_not_trigger_thinking_only_recovery(
) -> TestResult {
    // @step Given length-limited responses with useful mixed output or ordinary text without thinking
    for (reasoning, answer) in [
        (REASONING, "The guard is fixed; run the scoped tests next."),
        ("", "The truncated answer contains useful progress."),
    ] {
        let mut fixture = Fixture::new(
            vec![response(reasoning, answer, Some("length"))],
            true,
            false,
        )
        .await?;
        // @step When the responses finish streaming
        fixture.run().await?;
        // @step Then each response completes without a thinking exhaustion retry or auto-continue nudge
        assert_eq!(fixture.requests().await?.len(), 1);
        assert_eq!(fixture.retries(), 0);
        assert_eq!(fixture.session.continue_nudges_used, 0);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn non_length_thinking_only_completions_do_not_enter_exhaustion_recovery() -> TestResult {
    // @step Given thinking-only responses with clean or unsafe reported stop reasons
    // BUG-314: a stream without a finish reason is a provider protocol error.
    for stop in [
        Some("stop"),
        Some("content_filter"),
        Some("error"),
        Some("tool_calls"),
    ] {
        let mut fixture = Fixture::new(vec![response(REASONING, "", stop)], false, false).await?;
        // @step When the responses finish with auto-continue disabled
        fixture.run().await?;
        // @step Then no thinking exhaustion retry occurs
        assert_eq!(fixture.requests().await?.len(), 1);
        assert_eq!(fixture.retries(), 0);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn terminal_provider_errors_do_not_trigger_thinking_only_recovery() -> TestResult {
    // @step Given a provider rejects the streaming request with a terminal error
    let terminal = ResponseTemplate::new(400).set_body_json(serde_json::json!({
        "error":{"message":"unsupported request","type":"invalid_request_error"}
    }));
    let mut fixture = Fixture::new(vec![terminal], true, false).await?;
    // @step When the stream loop handles the error with auto-continue armed
    let result = fixture.run().await;
    // @step Then no recovery request or auto-continue nudge is sent
    assert!(result.is_err());
    assert_eq!(fixture.requests().await?.len(), 1);
    assert_eq!(fixture.retries(), 0);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    assert!(fixture
        .output
        .events
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| matches!(event, StreamEvent::Error(_))));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn user_cancellation_prevents_a_thinking_only_recovery_request() -> TestResult {
    // @step Given a thinking-only length response whose thinking event triggers user cancellation
    let mut fixture =
        Fixture::new(vec![response(REASONING, "", Some("length"))], true, true).await?;
    // @step When the stream loop observes the cancellation
    fixture.run().await?;
    // @step Then no recovery request or auto-continue nudge is sent
    assert_eq!(fixture.requests().await?.len(), 1);
    assert_eq!(fixture.retries(), 0);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}
