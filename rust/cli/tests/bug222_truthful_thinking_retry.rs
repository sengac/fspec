//! Feature: spec/features/truthful-thinking-retry-policy.feature

#[path = "bug221/support.rs"]
mod support;

use codelet_cli::interactive::{
    StreamEvent, MAX_THINKING_EXHAUSTION_RETRIES, THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
};
use codelet_tools::facade::ThinkingLevel;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use support::{response, tool_response, Fixture, TestResult, REASONING};
use wiremock::ResponseTemplate;

async fn captured(fixture: &Fixture) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    fixture
        .requests()
        .await?
        .iter()
        .map(|request| serde_json::from_slice(&request.body).map_err(Into::into))
        .collect()
}

fn prompt_only_notices(fixture: &Fixture) {
    let statuses = fixture.statuses();
    let notice = statuses
        .iter()
        .find(|text| text.starts_with("Thinking exhaustion detected"));
    assert!(
        notice.is_some_and(|text| text.contains("prompt-only") && text.contains("unchanged")),
        "{statuses:?}"
    );
    assert!(!statuses
        .iter()
        .any(|text| text.contains("reduced reasoning budget")
            || text.contains("disabled for this turn")));
}

#[tokio::test]
#[serial_test::serial]
async fn prompt_only_retries_preserve_effective_reasoning_settings_and_output_caps() -> TestResult {
    // @step Given exhaustion responses under high low minimal or absent reasoning controls and a fixed output cap
    for effort in [Some("high"), Some("low"), Some("minimal"), None] {
        let mut fixture = Fixture::new(
            vec![
                response(REASONING, "", Some("length")),
                response("", "Useful answer", Some("length")),
            ],
            false,
            false,
        )
        .await?;
        fixture.request_settings = effort.map(|effort| json!({"reasoning_effort":effort}));
        // @step When an exhaustion response is followed by a useful answer
        fixture.run().await?;
        // @step Then the first and retry requests have identical reasoning controls and output limits
        let requests = captured(&fixture).await?;
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(request["reasoning_effort"].as_str(), effort);
            assert_eq!(request["max_completion_tokens"], 4096);
        }
        // @step And the retry prompt and status explicitly describe prompt-only recovery with unchanged settings
        let prompt = requests[1]["messages"]
            .as_array()
            .and_then(|messages| messages.last())
            .ok_or("retry prompt missing")?["content"]
            .to_string();
        assert!(
            prompt.contains("prompt-only") && prompt.contains("unchanged"),
            "{prompt}"
        );
        assert!(!prompt.contains("has been reduced"));
        prompt_only_notices(&fixture);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn exhausted_retries_stop_without_falsely_claiming_thinking_was_disabled() -> TestResult {
    // @step Given every response exhausts thinking and the future-turn preference is already Off
    let mut fixture =
        Fixture::new(vec![response(REASONING, "", Some("length"))], false, false).await?;
    fixture.session.session_thinking_level = ThinkingLevel::Off;
    fixture.session.thinking_exhaustion_cross_turn_count =
        THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD - 1;
    // @step When the bounded recovery budget is exhausted
    fixture.run().await?;
    // @step Then exactly two retry requests preserve the original settings and output cap
    let requests = captured(&fixture).await?;
    assert_eq!(requests.len(), MAX_THINKING_EXHAUSTION_RETRIES as usize + 1);
    assert_eq!(fixture.retries(), MAX_THINKING_EXHAUSTION_RETRIES as usize);
    for request in requests {
        assert!(request["reasoning_effort"].is_null());
        assert_eq!(request["max_completion_tokens"], 4096);
    }
    // @step And the final warning says no further recovery requests without claiming thinking was disabled or reduced
    let notices = fixture.statuses().join("\n");
    assert!(
        notices.contains("retry budget exhausted")
            && notices.contains("No further recovery requests"),
        "{notices}"
    );
    assert!(
        !notices.contains("disabled") && !notices.contains("reduced"),
        "{notices}"
    );
    assert_eq!(fixture.session.session_thinking_level, ThinkingLevel::Off);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cross_turn_downgrade_affects_the_future_preference_rather_than_the_active_retry(
) -> TestResult {
    // @step Given a High preference one exhaustion away from the cross-turn downgrade threshold
    let mut fixture = Fixture::new(
        vec![
            response(REASONING, "", Some("length")),
            response("", "Useful answer", Some("length")),
        ],
        false,
        false,
    )
    .await?;
    fixture.session.session_thinking_level = ThinkingLevel::High;
    fixture.session.thinking_exhaustion_cross_turn_count =
        THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD - 1;
    fixture.request_settings = Some(json!({"reasoning_effort":"high"}));
    // @step When another exhaustion response triggers a retry
    fixture.run().await?;
    // @step Then the future preference becomes Medium while both captured requests retain high reasoning effort
    assert_eq!(
        fixture.session.session_thinking_level,
        ThinkingLevel::Medium
    );
    let requests = captured(&fixture).await?;
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| request["reasoning_effort"] == "high"));
    // @step And the notification distinguishes subsequent turns from unchanged current retry settings
    let notices = fixture.statuses().join("\n");
    assert!(
        notices.contains("subsequent turns")
            && notices.contains("current retry settings")
            && notices.contains("unchanged"),
        "{notices}"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn prompt_only_recovery_retains_conversation_history_reasoning_and_completed_tool_pairs(
) -> TestResult {
    // @step Given a conversation with completed tool progress and an earlier answer
    let mut fixture = Fixture::new(
        vec![
            tool_response(),
            response("", "Earlier answer", Some("stop")),
            response(REASONING, "", Some("length")),
            response("", "Useful answer", Some("length")),
        ],
        false,
        false,
    )
    .await?;
    fixture.run().await?;
    assert_eq!(fixture.tool_calls.load(Ordering::Acquire), 1);
    // @step When the next user turn exhausts thinking and recovers with a prompt-only retry
    fixture.run().await?;
    // @step Then the retry retains the original history and matched tool pairs without duplicate recovery prompts
    let requests = captured(&fixture).await?;
    assert_eq!(requests.len(), 4);
    let initial = requests[2]["messages"]
        .as_array()
        .ok_or("initial history missing")?;
    let retry = requests[3]["messages"]
        .as_array()
        .ok_or("retry history missing")?;
    assert!(retry.starts_with(initial));
    assert_eq!(
        retry
            .iter()
            .filter(|message| message["tool_calls"]
                .as_array()
                .is_some_and(|calls| calls.iter().any(|call| call["id"] == "inspect-1")))
            .count(),
        1
    );
    assert_eq!(
        retry
            .iter()
            .filter(|message| message["tool_call_id"] == "inspect-1")
            .count(),
        1
    );
    assert_eq!(
        retry
            .iter()
            .filter(|message| message["content"]
                .to_string()
                .contains("Your previous response"))
            .count(),
        1
    );
    // @step And captured reasoning is present in the retry prompt while request caps remain unchanged
    assert!(retry.last().ok_or("retry prompt missing")?["content"]
        .to_string()
        .contains(REASONING));
    assert!(requests
        .iter()
        .all(|request| request["max_completion_tokens"] == 4096));
    prompt_only_notices(&fixture);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cancellation_during_recovery_prevents_an_unused_prompt_and_extra_request() -> TestResult {
    // @step Given user cancellation on a thinking event or recovery notification
    for cancel_on_thinking in [true, false] {
        let mut fixture = Fixture::new(
            vec![response(REASONING, "", Some("length"))],
            false,
            cancel_on_thinking,
        )
        .await?;
        fixture.output.cancel_on_retry = !cancel_on_thinking;
        // @step When the stream loop handles thinking exhaustion
        fixture.run().await?;
        // @step Then only the initial request is sent and no unused recovery prompt is appended
        assert_eq!(fixture.requests().await?.len(), 1);
        let history = serde_json::to_string(&fixture.session.messages)?;
        assert!(!history.contains("Your previous response"), "{history}");
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_terminal_retry_error_stops_prompt_only_recovery_immediately() -> TestResult {
    // @step Given thinking exhaustion followed by a terminal provider rejection on retry
    let rejected = ResponseTemplate::new(400).set_body_json(
        json!({"error":{"message":"unsupported request","type":"invalid_request_error"}}),
    );
    let mut fixture = Fixture::new(
        vec![response(REASONING, "", Some("length")), rejected],
        false,
        false,
    )
    .await?;
    // @step When the retry fails
    let result = fixture.run().await;
    // @step Then exactly two requests occur and the terminal error is surfaced without another recovery attempt
    assert!(result.is_err());
    assert_eq!(fixture.requests().await?.len(), 2);
    assert_eq!(fixture.retries(), 1);
    assert!(fixture
        .output
        .events
        .lock()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|event| matches!(event, StreamEvent::Error(_))));
    prompt_only_notices(&fixture);
    Ok(())
}
