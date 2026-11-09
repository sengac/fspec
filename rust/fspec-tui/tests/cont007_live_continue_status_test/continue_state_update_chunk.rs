//! Feature: spec/features/live-continue-status-indicator.feature
use super::*;
use codelet_test_helpers::{OrFail, TestResult};

#[test]
fn continue_state_update_chunk_round_trips_and_stays_out_of_the_transcript() -> TestResult {
    // @step Given a StreamChunk::ContinueStateUpdate with enabled true, budget 10, nudgesUsed 3, goalActive false, effectiveBudget 10
    let chunk = StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 3,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: false,
        done_rejections: 0,
    });

    // @step When the chunk is serialized to JSON and deserialized back
    let json = serde_json::to_string(&chunk).or_fail("serialize ContinueStateUpdate")?;
    let back: StreamChunk =
        serde_json::from_str(&json).or_fail("deserialize ContinueStateUpdate")?;

    // @step Then the JSON payload uses the camelCase field names nudgesUsed, goalActive and effectiveBudget
    assert!(
        json.contains("nudgesUsed")
            && json.contains("goalActive")
            && json.contains("effectiveBudget"),
        "wire JSON must be camelCase; got: {json}"
    );

    // @step And the deserialized chunk carries the same field values
    match back {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(continue_state.enabled);
            assert_eq!(continue_state.budget, 10);
            assert_eq!(continue_state.nudges_used, 3);
            assert!(!continue_state.goal_active);
            assert_eq!(continue_state.effective_budget, 10);
        }
        other => {
            return Err(
                format!("expected ContinueStateUpdate after round-trip, got {other:?}").into(),
            )
        }
    }

    // @step And recording the chunk into a SessionContext adds zero scrollback chunks
    let mut ctx = SessionContext::new(sid("s-1"));
    ctx.record_chunk(&StreamChunk::continue_state_update(ContinueStateInfo {
        enabled: true,
        budget: 10,
        nudges_used: 3,
        goal_active: false,
        effective_budget: 10,
        goal_cleared: false,
        done_rejections: 0,
    }));
    assert_eq!(
        ctx.scrollback.chunk_count(),
        0,
        "ContinueStateUpdate is state-only — it must NEVER land in the transcript"
    );
    Ok(())
}

#[test]
fn engine_snapshot_reports_the_effective_goal_budget() -> TestResult {
    // @step Given a session with auto-continue enabled and budget 10 and 2 nudges used
    let mut session = fresh_session()?;
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 2;

    // @step When the continue state event is built without an active goal
    let event = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports enabled true, budget 10, nudgesUsed 2, goalActive false and effectiveBudget 10
    assert!(event.enabled);
    assert_eq!(event.budget, 10);
    assert_eq!(event.nudges_used, 2);
    assert!(!event.goal_active);
    assert_eq!(event.effective_budget, 10);
    assert_eq!(event.reason, ContinueStateReason::TurnStart);

    // @step When a goal is set on the session and the event is rebuilt
    session.set_goal("ship the feature", None);
    let goal_event = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports goalActive true and effectiveBudget 15
    assert!(goal_event.goal_active);
    assert_eq!(
        goal_event.effective_budget, 15,
        "goal mode effective budget must be max(explicit, 15)"
    );

    // @step When the budget is raised to 20 and the event is rebuilt
    session.continue_budget = 20;
    let raised = continue_state_event(&session, ContinueStateReason::TurnStart);

    // @step Then the event reports effectiveBudget 20 and goalActive true
    assert_eq!(raised.effective_budget, 20);
    assert!(raised.goal_active);
    Ok(())
}

#[test]
fn accepted_done_teardown_emits_a_reset_counter_state_from_the_shared_teardown() -> TestResult {
    // @step Given a session with 4 nudges used and an active goal
    let session_id = uuid::Uuid::new_v4();
    let mut session = fresh_session()?;
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 4;
    session.set_goal("make all tests pass", None);
    let output = RecordingOutput::new();

    // @step When apply_finish_with_summary runs for an accepted done() summary
    apply_finish_with_summary(&mut session, session_id, "all tests pass", &output);

    // @step Then a ContinueState event is emitted after the announcement with nudgesUsed 0 and goalActive false
    let events = output.events()?;
    let status_pos = events
        .iter()
        .position(
            |e| matches!(e, StreamEvent::Status(s) if s.starts_with("\u{1F3AF} goal satisfied:")),
        )
        .or_fail("goal teardown must announce the satisfied goal")?;
    let state_pos = events
        .iter()
        .position(|e| matches!(e, StreamEvent::ContinueState(_)))
        .or_fail("teardown must emit a ContinueState event")?;
    assert!(
        state_pos > status_pos,
        "ContinueState must be emitted AFTER the announcement"
    );
    match &events[state_pos] {
        StreamEvent::ContinueState(cs) => {
            assert_eq!(cs.nudges_used, 0, "teardown must reset the nudge counter");
            assert!(!cs.goal_active, "goal must be cleared before the emission");
            // [UPDATED BY CONT-008] the goal-clearing teardown now marks
            // its snapshot with the DEDICATED GoalSatisfied reason (wire
            // goalCleared: true) so the twins can write the chrome goal
            // state back; DoneAccepted remains for the non-goal case below.
            assert_eq!(cs.reason, ContinueStateReason::GoalSatisfied);
        }
        _ => unreachable!(),
    }

    // @step When the teardown runs again for a session without an active goal
    let mut plain = fresh_session()?;
    plain.continue_enabled = true;
    plain.continue_nudges_used = 2;
    let plain_output = RecordingOutput::new();
    apply_finish_with_summary(&mut plain, uuid::Uuid::new_v4(), "done", &plain_output);
    let plain_events = plain_output.events()?;

    // @step Then a ContinueState event is emitted with nudgesUsed 0
    assert!(
        plain_events
            .iter()
            .any(|e| matches!(e, StreamEvent::Status(s) if s == "\u{2713} done: done")),
        "non-goal teardown must keep the closing line"
    );
    assert!(
        plain_events.iter().any(|e| matches!(
            e,
            StreamEvent::ContinueState(cs)
                if cs.nudges_used == 0 && cs.reason == ContinueStateReason::DoneAccepted
        )),
        "non-goal teardown must emit the reset ContinueState event"
    );
    Ok(())
}

#[test]
fn footer_paints_the_real_nudge_counter_from_a_live_update() -> TestResult {
    // @step Given an App with an open session whose cached continue state is enabled with budget 10
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/continue 10".to_string()));
    }
    let before = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .or_fail("s-1 context")?
        .scrollback
        .chunk_count();

    // @step When a ContinueStateUpdate chunk with nudgesUsed 2 and effectiveBudget 10 is dispatched for the session
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 2,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));

    // @step Then the session scrollback gains no transcript chunk
    let after = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .or_fail("s-1 context")?
        .scrollback
        .chunk_count();
    assert_eq!(
        after, before,
        "state-only chunk must not touch the transcript"
    );

    // @step And the painted footer shows "⏩ auto-continue (2/10)"
    let text = painted_footer(&app, &sid("s-1"));
    assert!(
        text.contains("\u{23E9} auto-continue (2/10)"),
        "footer must paint the live counter; painted: {text:?}"
    );

    // @step When a ContinueStateUpdate chunk with goalActive true, nudgesUsed 1 and effectiveBudget 15 is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 1,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 0,
        }),
    ));

    // @step Then the painted footer shows "🎯 goal (1/15)"
    let goal_text = painted_footer(&app, &sid("s-1"));
    assert!(
        goal_text.contains("\u{1F3AF} goal (1/15)"),
        "footer must paint the goal indicator with the effective budget; painted: {goal_text:?}"
    );
    Ok(())
}
