//! Feature: spec/features/goal-state-back-sync.feature
use super::*;
use codelet_test_helpers::{OrFail, TestResult};

#[test]
fn goal_satisfied_teardown_emits_the_dedicated_goal_cleared_snapshot() -> TestResult {
    // @step Given a session with an active goal and 4 nudges used
    let session_id = uuid::Uuid::new_v4();
    let mut session = fresh_cli_session()?;
    session.continue_enabled = true;
    session.continue_budget = 10;
    session.continue_nudges_used = 4;
    session.set_goal("make tests pass", None);
    let output = RecordingOutput::new();

    // @step When the shared teardown runs for an accepted done() summary
    apply_finish_with_summary(&mut session, session_id, "all tests pass", &output);

    // @step Then the satisfied goal is announced before the counter snapshot
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

    // @step And the snapshot carries the goal-satisfied reason with goalActive false and nudgesUsed 0
    match &events[state_pos] {
        StreamEvent::ContinueState(cs) => {
            assert_eq!(
                cs.reason,
                ContinueStateReason::GoalSatisfied,
                "goal teardown must mark the snapshot with the DEDICATED goal-cleared reason"
            );
            assert!(!cs.goal_active, "goal must be cleared before the emission");
            assert_eq!(cs.nudges_used, 0, "teardown must reset the nudge counter");
        }
        _ => unreachable!(),
    }

    // @step And running the teardown without a goal emits the done-accepted reason instead
    let mut plain = fresh_cli_session()?;
    plain.continue_enabled = true;
    plain.continue_nudges_used = 2;
    let plain_output = RecordingOutput::new();
    apply_finish_with_summary(&mut plain, uuid::Uuid::new_v4(), "done", &plain_output);
    assert!(
        plain_output.events()?.iter().any(|e| matches!(
            e,
            StreamEvent::ContinueState(cs)
                if cs.reason == ContinueStateReason::DoneAccepted && cs.nudges_used == 0
        )),
        "non-goal teardown must keep the DoneAccepted reason (goalCleared false on the wire)"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn goal_satisfied_snapshot_writes_the_chrome_goal_state_back() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await?;
    session.set_goal_state(Some(("ship the fix".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string(), None);
    let mut rx = session.subscribe_to_stream();

    // @step When the background output maps a goal-satisfied counter snapshot
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::GoalSatisfied,
        false,
    )));

    // @step Then the chrome goal state reads as no goal
    assert!(
        session.get_goal_state().is_none(),
        "GoalSatisfied must write the chrome goal state back to None"
    );

    // @step And the pushed chunk carries goalCleared true
    let chunk = rx.try_recv().or_fail("a chunk must have been pushed")?;
    match chunk {
        StreamChunk::ContinueStateUpdate { continue_state } => {
            assert!(
                continue_state.goal_cleared,
                "the wire chunk must carry goalCleared true for the TUI cache drop"
            );
            assert!(!continue_state.goal_active);
        }
        other => return Err(format!("expected ContinueStateUpdate, got {other:?}").into()),
    }

    // @step And mapping a non-goal done-accepted snapshot leaves a synced chrome goal untouched
    session.set_goal_state(Some(("second goal".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::DoneAccepted,
        false,
    )));
    assert_eq!(
        session.get_goal_state(),
        Some(("second goal".to_string(), None)),
        "a DoneAccepted (goalCleared false) snapshot must NEVER wipe the chrome goal"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_goal_replaced_mid_turn_survives_the_goal_satisfied_write_back() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await?;
    session.set_goal_state(Some(("goal A".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step And the user has since replaced the chrome goal mid-turn
    session.set_goal_state(Some(("goal B".to_string(), Some("true".to_string()))));

    // @step When the background output maps a goal-satisfied counter snapshot
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string(), None);
    output.emit(StreamEvent::ContinueState(snapshot(
        ContinueStateReason::GoalSatisfied,
        false,
    )));

    // @step Then the chrome goal state still holds the replacement goal
    assert_eq!(
        session.get_goal_state(),
        Some(("goal B".to_string(), Some("true".to_string()))),
        "a mid-turn replacement (generation moved) must WIN over the write-back"
    );

    // @step And the next dispatch sync applies the replacement goal to the inner session
    let mut inner = session.inner.lock().await;
    session.sync_completion_contract_for_user_turn(&mut inner);
    let goal = inner.goal.as_ref().or_fail("inner goal must be set")?;
    assert_eq!(goal.text, "goal B");
    assert_eq!(goal.verify.as_deref(), Some("true"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_satisfied_goal_is_not_resurrected_on_the_next_dispatched_user_message() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await?;
    session.set_goal_state(Some(("make tests pass".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }

    // @step And the engine accepted done() for the goal and the background output performed the write-back
    let output = BackgroundOutput::with_provider(session.clone(), "test".to_string(), None);
    {
        let mut inner = session.inner.lock().await;
        // The REAL teardown against the REAL twin: apply_finish_with_summary
        // emits the GoalSatisfied snapshot through BackgroundOutput, which
        // performs the chrome write-back.
        apply_finish_with_summary(&mut inner, session.id, "all tests pass", &output);
        assert!(inner.goal.is_none(), "teardown must clear the inner goal");
    }
    assert!(
        session.get_goal_state().is_none(),
        "write-back must clear the chrome goal state"
    );

    // @step When the dispatch-site sync helper runs for the next real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session has no goal
    let inner = session.inner.lock().await;
    assert!(
        inner.goal.is_none(),
        "RESURRECTION: the dispatch sync must not re-apply the satisfied goal"
    );

    // @step And the done() registry reports the session as disarmed with no goal spec
    assert!(
        !codelet_tools::is_continue_armed(session.id),
        "registry must stay disarmed after the satisfied goal"
    );
    assert!(
        codelet_tools::get_session_goal(session.id).is_none(),
        "registry must hold no goal spec after the satisfied goal"
    );

    // @step And no CompletionContract reminder is re-injected into the inner session messages
    assert_eq!(
        count_system_reminders_by_type(&inner.messages, SystemReminderType::CompletionContract),
        0,
        "no CompletionContract reminder may be re-injected for a satisfied goal"
    );
    Ok(())
}
