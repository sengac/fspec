//! Feature: spec/features/goal-state-back-sync.feature
use super::*;
use codelet_test_helpers::{OrFail, TestResult};

#[test]
fn escalation_keeps_the_goal_indicator_on_the_bar() -> TestResult {
    // @step Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/goal ship it".to_string()));
    }
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: false,
            budget: 10,
            nudges_used: 5,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 4,
        }),
    ));

    // @step When the engine raises a goal escalation through a registered pause handler
    // Drive the REAL escalation path the Escalate arm runs (status line
    // first, then the per-session HITL pause) with a stub pause handler
    // standing in for the TUI/NAPI surface (tool_pause registry).
    let engine_session_id = uuid::Uuid::new_v4();
    let mut engine_session = fresh_cli_session()?;
    engine_session.set_goal("ship it", None);
    codelet_tools::set_session_goal(
        engine_session_id,
        Some(codelet_tools::GoalSpec {
            text: "ship it".to_string(),
            verify: None,
        }),
    );
    let sink = RecordingOutput::new();
    let handler_invoked = Arc::new(AtomicBool::new(false));
    let handler_flag = Arc::clone(&handler_invoked);
    let handler: PauseHandler = Arc::new(move |request| {
        assert_eq!(
            request.tool_name, "goal",
            "escalation must pause under the goal tool name"
        );
        handler_flag.store(true, Ordering::SeqCst);
        PauseResponse::Resumed
    });
    set_pause_handler(engine_session_id, Some(handler));
    let message = build_goal_blocked_message();
    sink.emit_status(&message);
    let resolution = raise_goal_escalation(engine_session_id, &message);
    set_pause_handler(engine_session_id, None);

    // @step Then the painted footer still shows the goal indicator
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{1F3AF} goal (5/15)"),
        "escalation leaves the goal active — the bar must keep showing it; painted: {text:?}"
    );

    // @step And the escalation path emits no goal-cleared signal in the engine sources
    let stream_loop = strip_line_comments(&read_source("cli/src/interactive/stream_loop.rs")?);
    let arm_start = stream_loop
        .find("ContinueDecision::Escalate")
        .or_fail("stream_loop.rs must have the Escalate arm")?;
    let arm_end = stream_loop[arm_start..]
        .find("ContinueDecision::Nudge")
        .map(|i| arm_start + i)
        .or_fail("the Nudge arm must follow the Escalate arm")?;
    let escalate_arm = &stream_loop[arm_start..arm_end];
    assert!(
        !escalate_arm.contains("emit_continue_state") && !escalate_arm.contains("GoalSatisfied"),
        "the Escalate arm must not emit a goal-cleared (or any counter) snapshot — \
         the goal stays active by design"
    );

    // @step And the pause handler is invoked and no continue-state event reaches the output sink
    assert!(
        handler_invoked.load(Ordering::SeqCst),
        "raise_goal_escalation must dispatch through the registered pause handler"
    );
    assert_eq!(
        resolution,
        PauseResponse::Resumed,
        "the stub handler resumes the escalation"
    );
    let sink_events = sink.events()?;
    assert!(
        sink_events
            .iter()
            .any(|e| matches!(e, StreamEvent::Status(s) if s == &message)),
        "the blocked status line stays in chat (terminal event)"
    );
    assert!(
        !sink_events
            .iter()
            .any(|e| matches!(e, StreamEvent::ContinueState(_))),
        "escalation must push no ContinueState (or goal-cleared) event to the sink"
    );

    // @step And the inner session goal remains set after the escalation
    assert!(
        engine_session.goal.is_some(),
        "escalation leaves the goal armed on the inner session"
    );
    assert_eq!(
        codelet_tools::get_session_goal(engine_session_id).map(|g| g.text),
        Some("ship it".to_string()),
        "escalation leaves the goal armed in the done() registry"
    );
    codelet_tools::set_session_goal(engine_session_id, None);
    Ok(())
}

#[test]
fn wire_chunk_and_twin_serializers_carry_the_goal_cleared_flag_and_rejection_count() -> TestResult {
    // @step Given a ContinueStateUpdate chunk with goalCleared true and doneRejections 2
    let chunk = StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 0,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: true,
        done_rejections: 2,
    });

    // @step When the chunk is serialized to JSON and deserialized back
    let json = serde_json::to_string(&chunk).or_fail("serialize ContinueStateUpdate")?;
    let back: StreamChunk =
        serde_json::from_str(&json).or_fail("deserialize ContinueStateUpdate")?;

    // @step Then the JSON payload uses the camelCase field names goalCleared and doneRejections
    assert!(
        json.contains("goalCleared") && json.contains("doneRejections"),
        "wire JSON must be camelCase; got: {json}"
    );
    match back {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(continue_state.goal_cleared);
            assert_eq!(continue_state.done_rejections, 2);
        }
        other => {
            return Err(
                format!("expected ContinueStateUpdate after round-trip, got {other:?}").into(),
            )
        }
    }

    // @step And a payload without the new fields still deserializes with false and zero defaults
    let mut value: serde_json::Value = serde_json::from_str(&json).or_fail("parse to Value")?;
    let info = value
        .pointer_mut("/ContinueStateUpdate/continueState")
        .or_fail("externally-tagged payload must carry the continueState object")?
        .as_object_mut()
        .or_fail("continueState must be an object")?;
    info.remove("goalCleared");
    info.remove("doneRejections");
    let legacy_json = serde_json::to_string(&value).or_fail("re-serialize legacy payload")?;
    let legacy: StreamChunk =
        serde_json::from_str(&legacy_json).or_fail("legacy payload must still deserialize")?;
    match legacy {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(
                !continue_state.goal_cleared,
                "missing goalCleared must default to false"
            );
            assert_eq!(
                continue_state.done_rejections, 0,
                "missing doneRejections must default to 0"
            );
        }
        other => {
            return Err(
                format!("expected ContinueStateUpdate from legacy payload, got {other:?}").into(),
            )
        }
    }

    // @step And both background twins and both twin JSON serializers carry the new fields and the shared write-back call
    let agent_loop_twin = strip_line_comments(&read_source("agent-loop/src/background_output.rs")?);
    assert!(
        agent_loop_twin.contains("clear_goal_state_if_unchanged_since_sync"),
        "agent-loop twin must call the SHARED BackgroundSession write-back helper \
         (CONT-009 twin-parity rule)"
    );
    assert!(
        agent_loop_twin.contains("goal_cleared") && agent_loop_twin.contains("done_rejections"),
        "agent-loop twin must map the new fields into ContinueStateInfo"
    );
    let agent_loop_json = strip_line_comments(&read_source("agent-loop/src/stream_chunk_json.rs")?);
    assert!(
        agent_loop_json.contains("goalCleared") && agent_loop_json.contains("doneRejections"),
        "agent-loop stream_chunk_to_json_value must serialize the new fields"
    );
    Ok(())
}
