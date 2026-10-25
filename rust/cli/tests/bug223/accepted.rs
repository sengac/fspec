//! Feature: spec/features/gemini-helper-accepted-completion.feature

use super::support::{enter_helper, text, tool, Fixture, TestResult};
use codelet_cli::interactive::{ContinueStateReason, StreamEvent};

#[tokio::test]
#[serial_test::serial]
async fn accepted_helper_done_settles_summary_and_goal_teardown_once() -> TestResult {
    // @step Given Gemini helper sessions have an armed contract with and without an active goal
    for goal in [false, true] {
        let mut script = enter_helper();
        script.extend([tool("done"), text("must not infer after accepted done")]);
        let mut fixture = Fixture::new(script).await?;
        let verified = fixture.directory.path().join("verified");
        if goal {
            let verify = format!("printf 'verified\\n' >> '{}'", verified.display());
            fixture
                .session
                .set_goal("finish verified task", Some(&verify));
            codelet_tools::set_session_goal(
                fixture.session_id,
                Some(codelet_tools::GoalSpec {
                    text: "finish verified task".into(),
                    verify: Some(verify),
                }),
            );
            fixture.session.done_rejections = 2;
        }
        fixture.session.continue_nudges_used = 3;
        // @step When the helper executes an accepted done tool call
        fixture.run().await?;
        // @step Then the common owner announces the accepted summary and resets the nudge counter once
        assert_eq!(
            fixture
                .output
                .statuses()
                .iter()
                .filter(|status| status.contains("Task verified"))
                .count(),
            1,
            "{:?}",
            fixture.output.events()
        );
        assert_eq!(fixture.session.continue_nudges_used, 0);
        assert!(codelet_tools::take_done_acceptance(fixture.session_id).is_none());
        let expected = if goal {
            ContinueStateReason::GoalSatisfied
        } else {
            ContinueStateReason::DoneAccepted
        };
        assert_eq!(fixture.output.events().iter().filter(|event| matches!(event,
            StreamEvent::ContinueState(state) if state.reason == expected && state.nudges_used == 0
        )).count(), 1);
        // @step And an active goal and its reminder and registry state are cleared without another inference
        assert_eq!(fixture.requests().await?.len(), 3);
        assert!(fixture.session.goal.is_none());
        assert!(codelet_tools::get_session_goal(fixture.session_id).is_none());
        assert_eq!(fixture.session.done_rejections, 0);
        assert!(!serde_json::to_string(&fixture.session.messages)?.contains("Active goal:"));
        if goal {
            assert_eq!(std::fs::read_to_string(verified)?, "verified\n");
        }
        // @step And exactly one terminal event records done acceptance
        assert_eq!(fixture.output.terminals(), vec![Some("done".into())]);
        let events = fixture.output.events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, StreamEvent::ToolCall(_)))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, StreamEvent::ToolResult(_)))
                .count(),
            2
        );
    }
    Ok(())
}
