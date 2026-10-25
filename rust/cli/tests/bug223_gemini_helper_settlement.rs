//! Feature: spec/features/gemini-helper-completion-settlement.feature

#[path = "bug223/support.rs"]
mod support;

#[path = "bug223/accepted.rs"]
mod accepted;

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;

use codelet_cli::interactive::{output::ContinueStateReason, StreamEvent};
use serde_json::json;
use std::sync::atomic::Ordering;
use support::{enter_helper, measured_response, response, text, tool, Fixture, TestResult};
use wiremock::ResponseTemplate;

#[tokio::test]
#[serial_test::serial]
async fn clean_helper_completion_consumes_a_counted_nudge() -> TestResult {
    // @step Given an armed Gemini session enters empty-after-tool continuation
    for nested in [false, true] {
        let mut script = enter_helper();
        if nested {
            script.extend([tool("inspect"), text("")]);
        }
        script.extend([
            measured_response(
                json!([{"text":"helper progress"}]),
                "STOP",
                json!({
                    "promptTokenCount":130,"candidatesTokenCount":17,"totalTokenCount":152,
                    "cachedContentTokenCount":30,"thoughtsTokenCount":5
                }),
            ),
            text("final progress"),
        ]);
        let mut fixture = Fixture::new(script).await?;
        // @step When the helper completes with text and a clean stop without accepted done
        fixture.run().await?;
        // @step Then the common owner sends one counted nudge and eventually emits exactly one terminal event
        let requests = fixture.requests().await?;
        assert_eq!(requests.len(), if nested { 6 } else { 4 });
        assert!(requests[2]
            .to_string()
            .contains("Based on the information retrieved"));
        if nested {
            assert!(requests[4]
                .to_string()
                .contains("Based on the information retrieved"));
        }
        assert!(requests
            .last()
            .ok_or("nudge request missing")?
            .to_string()
            .contains("done"));
        assert_eq!(fixture.session.continue_nudges_used, 1);
        assert_eq!(fixture.output.terminals(), vec![Some("end_turn".into())]);
        // @step And helper text history and measured usage are preserved
        let history = serde_json::to_string(&fixture.session.messages)?;
        assert_eq!(history.matches("helper progress").count(), 1);
        assert!(history.contains("final progress"));
        assert_eq!(
            fixture.session.token_tracker.output_tokens,
            if nested { 67 } else { 47 }
        );
        assert_eq!(fixture.session.token_tracker.reasoning_tokens, 5);
        assert!(fixture.output.events().iter().any(|event| matches!(event,
            StreamEvent::Tokens(usage) if usage.input_tokens == 130
                && usage.cache_read_input_tokens == Some(30)
                && usage.reasoning_tokens == Some(5)
        )));
        let persisted = codelet_core::persistence::load_session(fixture.session_id)?;
        assert_eq!(
            persisted.token_usage.cumulative_billed_output,
            if nested { 72 } else { 52 }
        );
        assert_eq!(persisted.token_usage.reasoning_tokens, 5);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn helper_tool_activity_refunds_a_pending_nudge() -> TestResult {
    // @step Given an armed Gemini session has consumed a nudge before entering the helper
    let mut script = enter_helper();
    script.extend([
        text("first helper"),
        tool("inspect"),
        text(""),
        tool("inspect"),
        text("productive helper"),
        text("budget reached"),
    ]);
    let mut fixture = Fixture::new(script).await?;
    // @step When the helper executes a tool and completes with a clean text response
    fixture.run().await?;
    // @step Then the common owner refunds the pending nudge before deciding the next nudge
    assert_eq!(fixture.requests().await?.len(), 8);
    assert_eq!(
        fixture.calls.load(Ordering::SeqCst),
        3,
        "{:?}",
        fixture.output.events()
    );
    let events = fixture.output.events();
    assert!(events
        .iter()
        .any(|event| matches!(event, StreamEvent::ContinueState(state)
        if state.reason == ContinueStateReason::RefundSettled && state.nudges_used == 0)));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, StreamEvent::ContinueState(state)
        if state.reason == ContinueStateReason::NudgeConsumed))
            .count(),
        2
    );
    // @step And eventual budget exhaustion emits exactly one terminal event
    assert_eq!(fixture.session.continue_nudges_used, 1);
    assert!(fixture
        .output
        .statuses()
        .iter()
        .any(|status| status.contains("never called done()")));
    assert_eq!(fixture.output.terminals().len(), 1);
    assert_eq!(fixture.session.token_tracker.output_tokens, 80);
    let persisted = codelet_core::persistence::load_session(fixture.session_id)?;
    assert_eq!(persisted.token_usage.cumulative_billed_output, 80);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn exhausted_helper_completion_emits_the_normal_budget_warning() -> TestResult {
    // @step Given an armed Gemini session enters the helper with no remaining nudge budget
    let mut script = enter_helper();
    script.push(text("helper finished"));
    let mut fixture = Fixture::new(script).await?;
    fixture.session.continue_budget = 0;
    // @step When the helper completes with a clean text response
    fixture.run().await?;
    // @step Then the common owner warns about exhausted budget without another inference
    assert!(fixture
        .output
        .events()
        .iter()
        .any(|event| matches!(event, StreamEvent::ContinueState(state)
        if state.reason == ContinueStateReason::BudgetExhausted)));
    assert!(fixture
        .output
        .statuses()
        .iter()
        .any(|status| status.contains("never called done()")));
    assert_eq!(fixture.requests().await?.len(), 3);
    // @step And exactly one terminal event is emitted
    assert_eq!(fixture.output.terminals().len(), 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn exhausted_goal_helper_completion_escalates_for_human_review() -> TestResult {
    // @step Given a Gemini helper session has an active goal and an exhausted goal budget
    let mut script = enter_helper();
    script.push(text("helper finished"));
    let mut fixture = Fixture::new(script).await?;
    fixture.session.set_goal("finish verified task", None);
    fixture.session.continue_nudges_used = 15;
    // @step When the helper completes without accepted done
    fixture.run().await?;
    // @step Then the common owner raises goal escalation and retains the active goal without another inference
    assert!(fixture
        .output
        .statuses()
        .iter()
        .any(|status| status.contains("human review")));
    assert!(fixture.session.goal.is_some());
    assert_eq!(fixture.requests().await?.len(), 3);
    // @step And exactly one terminal event is emitted
    assert_eq!(fixture.output.terminals().len(), 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn nonclean_helper_stops_never_consume_a_nudge() -> TestResult {
    // @step Given armed Gemini helper sessions return token safety or unknown stop reasons
    for (wire, normalized) in [
        ("MAX_TOKENS", "max_tokens"),
        ("SAFETY", "content_filter"),
        ("OTHER", "unknown"),
    ] {
        let mut script = enter_helper();
        script.push(response(json!([{"text":"useful helper answer"}]), wire));
        let mut fixture = Fixture::new(script).await?;
        // @step When each helper completes with useful text
        fixture.run().await?;
        // @step Then no nudge is consumed and exactly one terminal event retains the actual helper stop reason
        assert_eq!(fixture.requests().await?.len(), 3);
        assert_eq!(fixture.session.continue_nudges_used, 0);
        assert_eq!(fixture.output.terminals(), vec![Some(normalized.into())]);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn helper_errors_do_not_settle_a_clean_completion() -> TestResult {
    // @step Given an armed Gemini session enters the helper
    let mut script = enter_helper();
    script.push(
        ResponseTemplate::new(400)
            .set_body_json(json!({"error":{"message":"terminal fixture error"}})),
    );
    let mut fixture = Fixture::new(script).await?;
    // @step When the helper receives a terminal provider error
    let result = fixture.run().await;
    // @step Then the error is surfaced without a nudge or a successful terminal event
    assert!(result.is_err());
    assert_eq!(fixture.requests().await?.len(), 3);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    assert!(fixture.output.terminals().is_empty());
    assert!(fixture
        .output
        .events()
        .iter()
        .any(|event| matches!(event, StreamEvent::Error(_))));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn helper_interruption_does_not_settle_a_clean_completion() -> TestResult {
    // @step Given an armed Gemini session enters the helper
    let mut script = enter_helper();
    script.push(text("partial helper"));
    let mut fixture = Fixture::new(script).await?;
    fixture.output.cancel_on_text = true;
    // @step When streamed helper text triggers user interruption
    fixture.run().await?;
    // @step Then interruption preserves partial history without a nudge and emits exactly one terminal event
    assert_eq!(fixture.requests().await?.len(), 3);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    assert!(serde_json::to_string(&fixture.session.messages)?.contains("partial helper"));
    assert_eq!(fixture.output.terminals().len(), 1);
    assert!(fixture
        .output
        .events()
        .iter()
        .any(|event| matches!(event, StreamEvent::Interrupted(_))));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn helper_compaction_hands_off_without_a_completion_nudge() -> TestResult {
    // @step Given an armed Gemini helper executes a tool whose result exceeds the context threshold
    let (mut fixture, compactions) = support::compaction_fixture().await?;
    // @step When the next helper request is cancelled for compaction
    let result = fixture.run().await;
    // @step Then the compaction owner receives the handoff without a nudge or premature terminal success
    support::assert_compaction_error(result)?;
    assert_eq!(compactions.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.requests().await?.len(), 3);
    assert_eq!(fixture.session.continue_nudges_used, 0);
    assert!(fixture.output.terminals().is_empty());
    assert!(fixture
        .output
        .events()
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionStarted)));
    Ok(())
}
