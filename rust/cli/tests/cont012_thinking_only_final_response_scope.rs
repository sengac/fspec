//! Feature: spec/features/thinking-only-exhaustion-final-response-scope.feature

#[path = "bug221/support.rs"]
mod support;

use codelet_cli::interactive::{StreamEvent, MAX_THINKING_EXHAUSTION_RETRIES};
use std::sync::{atomic::Ordering, PoisonError};
use support::{response, tool_response, Fixture, TestResult, REASONING};

const FINAL_REASONING: &str = "Weigh the two layouts before writing the stub.";

fn last_done(fixture: &Fixture) -> Option<Option<String>> {
    fixture
        .output
        .events
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .rev()
        .find_map(|event| match event {
            StreamEvent::Done(stop_reason) => Some(stop_reason.clone()),
            _ => None,
        })
}

#[tokio::test]
#[serial_test::serial]
async fn thinking_only_length_final_after_earlier_tool_progress_is_recovered_in_an_armed_session(
) -> TestResult {
    // @step Given an armed session whose first request completes a tool call
    // @step And the next response contains only thinking and stops for length
    let mut fixture = Fixture::new(
        vec![
            tool_response(),
            response(FINAL_REASONING, "", Some("length")),
            response("", "Task finished.", Some("length")),
        ],
        true,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then exactly one thinking exhaustion retry is sent
    let requests = fixture.requests().await?;
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 1);
    assert_eq!(requests.len(), 3);
    assert_eq!(fixture.retries(), 1);
    // @step And the retry prompt preserves only the final response's reasoning
    let retry: serde_json::Value = serde_json::from_slice(&requests[2].body)?;
    let retry_prompt = retry["messages"]
        .as_array()
        .and_then(|messages| messages.last())
        .map(|message| message["content"].to_string())
        .ok_or("retry prompt missing")?;
    assert!(retry_prompt.contains(FINAL_REASONING), "{retry_prompt}");
    assert!(!retry_prompt.contains(REASONING), "{retry_prompt}");
    // @step And the retry's answer finishes the turn without consuming an auto-continue nudge
    assert!(fixture
        .output
        .events
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .any(|event| matches!(event, StreamEvent::Text(text) if text == "Task finished.")));
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn thinking_only_length_final_after_earlier_tool_progress_is_recovered_with_auto_continue_disabled(
) -> TestResult {
    // @step Given an unarmed session whose first request completes a tool call
    // @step And the next response contains only thinking and stops for length
    let mut fixture = Fixture::new(
        vec![
            tool_response(),
            response(FINAL_REASONING, "", Some("length")),
            response("", "Task finished.", Some("length")),
        ],
        false,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then exactly one thinking exhaustion retry is sent
    assert_eq!(fixture.requests().await?.len(), 3);
    assert_eq!(fixture.retries(), 1);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn armed_session_nudges_after_thinking_exhaustion_recovery_is_exhausted() -> TestResult {
    // @step Given an armed session with an auto-continue budget of 1
    // @step And every response contains only thinking and stops for length
    let mut fixture = Fixture::new(
        vec![response(FINAL_REASONING, "", Some("length"))],
        true,
        false,
    )
    .await?;
    fixture.session.continue_budget = 1;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then two thinking exhaustion retries are followed by one auto-continue nudge
    assert_eq!(fixture.retries(), MAX_THINKING_EXHAUSTION_RETRIES as usize);
    assert_eq!(
        fixture.requests().await?.len(),
        MAX_THINKING_EXHAUSTION_RETRIES as usize + 2
    );
    assert_eq!(fixture.session.continue_nudges_used, 1);
    // @step And the auto-continue budget exhaustion warning is shown
    assert!(fixture
        .statuses()
        .iter()
        .any(|text| text.contains("auto-continue: model never called done()")));
    // @step And the turn's done event reports the normalized provider stop reason max_tokens
    assert_eq!(last_done(&fixture), Some(Some("max_tokens".to_string())));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unarmed_session_finishes_after_thinking_exhaustion_recovery_is_exhausted() -> TestResult {
    // @step Given an unarmed session where every response contains only thinking and stops for length
    let mut fixture = Fixture::new(
        vec![response(FINAL_REASONING, "", Some("length"))],
        false,
        false,
    )
    .await?;
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then exactly two thinking exhaustion retries occur before completion
    assert_eq!(
        fixture.requests().await?.len(),
        MAX_THINKING_EXHAUSTION_RETRIES as usize + 1
    );
    assert_eq!(fixture.retries(), MAX_THINKING_EXHAUSTION_RETRIES as usize);
    // @step And the thinking exhaustion warning is visible and no auto-continue nudge is consumed
    assert!(fixture
        .statuses()
        .iter()
        .any(|text| text.contains("retry budget exhausted")));
    assert_eq!(fixture.session.continue_nudges_used, 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn goal_mode_escalates_a_stalled_thinking_only_turn_instead_of_finishing_silently(
) -> TestResult {
    // @step Given a session with an active goal and auto-continue disabled
    // @step And every response contains only thinking and stops for length
    let mut fixture = Fixture::new(
        vec![response(FINAL_REASONING, "", Some("length"))],
        false,
        false,
    )
    .await?;
    fixture.session.set_goal("ship the stub layout", None);
    // @step When the turn runs to completion
    fixture.run().await?;
    // @step Then two auto-continue nudges are sent after the thinking exhaustion retries
    assert_eq!(fixture.retries(), MAX_THINKING_EXHAUSTION_RETRIES as usize);
    assert_eq!(
        fixture.requests().await?.len(),
        MAX_THINKING_EXHAUSTION_RETRIES as usize + 3
    );
    assert_eq!(fixture.session.continue_nudges_used, 2);
    // @step And the goal stall escalation message is shown
    assert!(fixture
        .statuses()
        .iter()
        .any(|text| text.contains("model stalled")));
    Ok(())
}
