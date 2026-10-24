//! Feature: spec/features/thinking-only-exhaustion-recovery.feature

#[path = "bug221/support.rs"]
mod support;

use codelet_cli::interactive::{StreamEvent, MAX_THINKING_EXHAUSTION_RETRIES};
use std::sync::atomic::Ordering;
use support::{response, tool_response, Fixture, TestResult, REASONING};
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
async fn repeated_thinking_only_length_stops_exhaust_the_bounded_recovery_budget() -> TestResult {
    // @step Given every streamed response contains only thinking with absent reasoning usage and 4096 output tokens
    let mut fixture =
        Fixture::new(vec![response(REASONING, "", Some("length"))], true, false).await?;
    // @step When every response stops for length
    fixture.run().await?;
    // @step Then exactly two thinking exhaustion retries occur before completion
    assert_eq!(
        fixture.requests().await?.len(),
        MAX_THINKING_EXHAUSTION_RETRIES as usize + 1
    );
    assert_eq!(fixture.retries(), MAX_THINKING_EXHAUSTION_RETRIES as usize);
    assert_eq!(fixture.session.thinking_exhaustion_cross_turn_count, 1);
    // @step And the exhaustion warning is visible and no auto-continue nudge is consumed
    assert!(fixture
        .statuses()
        .iter()
        .any(|text| text.contains("retry budget exhausted")));
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
    // @step Given thinking-only responses with clean absent or unsafe stop reasons
    for stop in [
        Some("stop"),
        None,
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

#[tokio::test]
#[serial_test::serial]
async fn tool_progress_prevents_thinking_only_recovery() -> TestResult {
    // @step Given a length-limited response containing thinking and a completed tool call
    let mut fixture = Fixture::new(
        vec![tool_response(), response(REASONING, "", Some("length"))],
        true,
        false,
    )
    .await?;
    // @step When the model stops without an answer after that tool progress
    fixture.run().await?;
    // @step Then no thinking exhaustion retry or auto-continue nudge is sent
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 1);
    assert_eq!(fixture.requests().await?.len(), 2);
    assert_eq!(fixture.retries(), 0);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}
