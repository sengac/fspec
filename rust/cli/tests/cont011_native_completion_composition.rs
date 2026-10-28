//! Feature: spec/features/native-completion-loopback-composition.feature
#[path = "support/cont011_builders.rs"]
mod builders;
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/cont011_native.rs"]
mod native;
#[path = "support/prov161_runtime.rs"]
pub mod runtime;
#[path = "support/cont011_security.rs"]
mod security;
#[path = "support/cont011_wire.rs"]
mod wire;

use codelet_cli::interactive::{ContinueStateReason, StreamEvent};
use environment::{Environment, TestResult};
use native::Case;
use wire::{Kind, Scenario};

#[tokio::test]
#[serial_test::serial]
async fn native_clean_stops_compose_nudge_work_and_accepted_done() -> TestResult {
    // @step Given isolated native Claude API OAuth Gemini and ZAI sessions with continuation armed
    let _environment = Environment::new()?;
    builders::assert_rejected_endpoints()?;
    let mut failures = Vec::new();
    for kind in Kind::ALL {
        let mut fixture = Case::new(kind, Scenario::Clean).await?;
        // @step When clean stops are followed by one automatic nudge native file work and accepted Done
        fixture.run().await?;
        // @step Then exact physical requests preserve tool result history and the file effect without trailing work
        let requests = fixture.requests().await?;
        assert_eq!(requests.len(), 3, "{kind:?}");
        assert!(requests[1]
            .contains(codelet_cli::interactive::auto_continue::AUTO_CONTINUE_NUDGE_PROMPT));
        assert!(
            requests[2].contains("native-effect"),
            "{kind:?}: missing tool result"
        );
        if matches!(kind, Kind::Gemini) {
            assert!(requests[2].contains("native-owned-signature"));
        }
        let written = std::fs::read_to_string(&fixture.effect).map_err(|error| {
            format!(
                "{kind:?}: missing file effect: {error}; {:?}",
                fixture
                    .output
                    .events()
                    .iter()
                    .filter(|event| matches!(event, StreamEvent::ToolResult(_)))
                    .collect::<Vec<_>>()
            )
        })?;
        assert_eq!(written, "native-effect");
        assert!(!fixture.effect.with_file_name("trailing").exists());
        let history = serde_json::to_string(&fixture.session.messages)?;
        assert!(history.contains("native-effect"));
        let events = fixture.output.events();
        let results = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolResult(result) => Some(result),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(results.len(), 2, "{kind:?}: {results:?}");
        for result in results {
            assert!(events
                .iter()
                .any(|event| matches!(event, StreamEvent::ToolCall(call) if call.id == result.id)));
        }
        // @step And each session settles one terminal event and once only measured physical billing
        if let Err(error) = fixture.assert_settled(3, 1) {
            failures.push(error.to_string());
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn anthropic_client_calls_require_authoritative_terminal_settlement() -> TestResult {
    // @step Given local Claude API and OAuth streams contain completed client calls before terminal delivery
    let _environment = Environment::new()?;
    for kind in [Kind::Claude, Kind::OAuth] {
        for scenario in [Scenario::Unsettled, Scenario::MalformedAfterTool] {
            let mut fixture = Case::new(kind, scenario).await?;
            // @step When the terminal delta is missing or malformed
            let outcome = fixture.run().await;
            let events = fixture.output.events();
            // @step Then no client call or accepted Done can execute and no outer continuation occurs
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, StreamEvent::ToolResult(_))),
                "{kind:?}: unsettled tool executed"
            );
            assert!(!events.iter().any(|event| matches!(event, StreamEvent::ContinueState(state) if state.reason == ContinueStateReason::NudgeConsumed)));
            assert!(!fixture.effect.with_file_name("trailing").exists());
            // @step And the response never publishes fabricated completion or terminal billing
            assert!(
                outcome.is_err()
                    || events
                        .iter()
                        .any(|event| matches!(event, StreamEvent::Error(_)))
            );
            assert!(!events.iter().any(|event| matches!(event, StreamEvent::Status(text) if text.contains("native complete"))));
            assert_eq!(fixture.requests().await?.len(), 1);
            let manifest = codelet_core::persistence::load_session(fixture.session_id)?;
            assert_eq!(manifest.token_usage.cumulative_billed_input, 0);
            assert_eq!(manifest.token_usage.cumulative_billed_output, 0);
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn native_rejected_goal_completion_remains_a_rejection() -> TestResult {
    // @step Given isolated native provider sessions with an active goal and armed Done
    let _environment = Environment::new()?;
    let mut failures = Vec::new();
    for kind in Kind::ALL {
        let mut fixture = Case::new(kind, Scenario::Goal).await?;
        // @step When Done without evidence is followed by a genuinely valid completion
        fixture.run().await?;
        // @step Then the first completion records a rejection and only the valid completion tears down the goal
        let requests = fixture.requests().await?;
        assert_eq!(requests.len(), 2, "{kind:?}");
        assert!(
            requests[1].contains("evidence"),
            "missing rejected tool outcome"
        );
        assert_eq!(
            *fixture.rejections.lock().map_err(|_| "rejections lock")?,
            [0, 1]
        );
        assert!(fixture.session.goal.is_none());
        assert!(codelet_tools::get_session_goal(fixture.session_id).is_none());
        assert!(!serde_json::to_string(&fixture.session.messages)?.contains("Active goal:"));
        assert!(!fixture.effect.with_file_name("trailing").exists());
        // @step And no rejection is converted into an automatic nudge or unmeasured success
        if let Err(error) = fixture.assert_settled(2, 0) {
            failures.push(error.to_string());
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn gemini_client_calls_require_safe_authoritative_terminal_settlement() -> TestResult {
    // @step Given local Gemini streams contain signed client calls before terminal delivery
    let _environment = Environment::new()?;
    for scenario in [
        Scenario::Unsettled,
        Scenario::MalformedAfterTool,
        Scenario::Safety,
        Scenario::Budget,
    ] {
        let mut fixture = Case::new(Kind::Gemini, scenario).await?;
        // @step When terminal evidence is missing malformed safety filtered or budget limited
        let outcome = fixture.run().await;
        let events = fixture.output.events();
        // @step Then no client call accepted Done or outer continuation occurs
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, StreamEvent::ToolResult(_))),
            "unsettled Gemini call executed"
        );
        assert!(!events.iter().any(|event| matches!(event, StreamEvent::ContinueState(state) if state.reason == ContinueStateReason::NudgeConsumed)));
        assert!(!fixture.effect.with_file_name("trailing").exists());
        assert_eq!(fixture.requests().await?.len(), 1);
        // @step And only validated terminal usage is settled without fabricated completion
        assert!(
            outcome.is_err()
                || events
                    .iter()
                    .any(|event| matches!(event, StreamEvent::Error(_)))
        );
        assert!(!events.iter().any(
            |event| matches!(event, StreamEvent::Status(text) if text.contains("native complete"))
        ));
        let manifest = codelet_core::persistence::load_session(fixture.session_id)?;
        let measured = matches!(scenario, Scenario::Safety | Scenario::Budget);
        assert_eq!(
            manifest.token_usage.cumulative_billed_input,
            if measured { 12 } else { 0 }
        );
        assert_eq!(
            manifest.token_usage.cumulative_billed_output,
            if measured { 3 } else { 0 }
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn isolated_security_still_blocks_high_risk_native_file_work() -> TestResult {
    // @step Given real native provider builders use a fixture owned loopback security dependency at default gates
    let _environment = Environment::new()?;
    for kind in Kind::ALL {
        let mut fixture = Case::new(kind, Scenario::Denied).await?;
        // @step When the synthetic security response classifies native file work as high risk
        let outcome = fixture.run().await;
        let events = fixture.output.events();
        // @step Then the original native Write remains blocked and no file effect occurs
        assert!(!fixture.effect.exists());
        assert!(!fixture.effect.with_file_name("trailing").exists());
        assert_eq!(fixture.security_checks().await?, 1);
        assert!(events.iter().any(|event| matches!(event, StreamEvent::ToolResult(result) if result.content.contains("RLCD security: P(risk)=0.99"))));
        // @step And the completion fixture cannot count the blocked operation as successful work
        assert!(
            outcome.is_err()
                || events
                    .iter()
                    .any(|event| matches!(event, StreamEvent::Error(_)))
        );
        assert!(!events.iter().any(
            |event| matches!(event, StreamEvent::Status(text) if text.contains("native complete"))
        ));
        assert_eq!(fixture.requests().await?.len(), 2);
        assert!(codelet_tools::take_done_acceptance(fixture.session_id).is_none());
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn native_malformed_delivery_and_cancellation_do_not_complete_work() -> TestResult {
    // @step Given isolated native provider streams with malformed delivery or explicit caller interruption
    let _environment = Environment::new()?;
    for kind in Kind::ALL {
        for scenario in [Scenario::Malformed, Scenario::Cancel] {
            let mut fixture = Case::new(kind, scenario).await?;
            // @step When the real CLI stream driver consumes those responses
            let outcome = fixture.run().await;
            let events = fixture.output.events();
            if matches!(scenario, Scenario::Malformed) {
                assert!(
                    outcome.is_err()
                        || events
                            .iter()
                            .any(|event| matches!(event, StreamEvent::Error(_)))
                );
            } else {
                assert!(events
                    .iter()
                    .any(|event| matches!(event, StreamEvent::Interrupted(_))));
            }
            // @step Then no accepted Done automatic nudge or trailing file effect occurs
            assert!(!fixture.effect.exists());
            assert!(!fixture.effect.with_file_name("trailing").exists());
            assert!(!events
                .iter()
                .any(|event| matches!(event, StreamEvent::ToolResult(_))));
            assert!(!events.iter().any(|event| matches!(event, StreamEvent::Status(text) if text.contains("native complete"))));
            assert!(!events.iter().any(|event| matches!(event, StreamEvent::ContinueState(state) if state.reason == ContinueStateReason::NudgeConsumed)));
            // @step And physical request and terminal accounting remain bounded without fabricated completion
            assert_eq!(fixture.requests().await?.len(), 1);
            assert!(
                events
                    .iter()
                    .filter(|event| matches!(event, StreamEvent::Done(_)))
                    .count()
                    <= 1
            );
            let manifest = codelet_core::persistence::load_session(fixture.session_id)?;
            let billed = (
                manifest.token_usage.cumulative_billed_input,
                manifest.token_usage.cumulative_billed_output,
            );
            assert!(
                [(0, 0), (12, 0), (12, 3)].contains(&billed),
                "{kind:?}: {billed:?}"
            );
            assert!(codelet_tools::take_done_acceptance(fixture.session_id).is_none());
        }
    }
    Ok(())
}
