//! Feature: spec/features/goal-state-back-sync.feature
use super::*;
use codelet_test_helpers::TestResult;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_dispatch_sync_never_reapplies_a_chrome_goal_the_engine_already_consumed() -> TestResult
{
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a BackgroundSession whose chrome goal was synced into the inner session at dispatch
    let (_dir, _manager, session) = fresh_background_session().await?;
    session.set_goal_state(Some(("stale goal".to_string(), None)));
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
        assert!(inner.goal.is_some(), "precondition: inner goal synced");
    }

    // @step And the engine cleared the inner goal without the chrome write-back landing
    {
        let mut inner = session.inner.lock().await;
        inner.clear_goal();
        codelet_tools::set_session_goal(session.id, None);
    }

    // @step When the dispatch-site sync helper runs for the next real user message
    {
        let mut inner = session.inner.lock().await;
        session.sync_completion_contract_for_user_turn(&mut inner);
    }

    // @step Then the inner session still has no goal
    {
        let inner = session.inner.lock().await;
        assert!(
            inner.goal.is_none(),
            "defense-in-depth: the guard must hold even when the push channel is lost \
             (chrome generation unchanged since the last sync)"
        );
        assert!(
            !codelet_tools::is_continue_armed(session.id),
            "registry must not be re-armed by the stale chrome goal"
        );
    }

    // @step And setting a new chrome goal afterwards is applied by the following dispatch sync
    session.set_goal_state(Some(("fresh goal".to_string(), None)));
    let mut inner = session.inner.lock().await;
    session.sync_completion_contract_for_user_turn(&mut inner);
    assert_eq!(
        inner.goal.as_ref().map(|g| g.text.as_str()),
        Some("fresh goal"),
        "a genuinely NEW user /goal (generation bumped) must be applied"
    );
    assert!(
        codelet_tools::is_continue_armed(session.id),
        "the new goal must arm the registry"
    );
    Ok(())
}

#[test]
fn the_tui_drops_the_goal_indicator_when_the_engine_clears_the_goal() -> TestResult {
    // @step Given a TUI session whose footer shows the goal indicator from a live counter snapshot
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/continue 10".to_string()));
    }
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/goal ship it".to_string()));
    }
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
    let before = painted_footer(&app, &sid("s-1"));
    assert!(
        before.contains("\u{1F3AF} goal (1/15)"),
        "precondition: footer must show the goal indicator; painted: {before:?}"
    );

    // @step When a counter snapshot chunk with goalCleared true is dispatched for the session
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::continue_state_update(ContinueStateInfo {
            enabled: true,
            budget: 10,
            nudges_used: 0,
            goal_active: false,
            effective_budget: 10,
            goal_cleared: true,
            done_rejections: 0,
        }),
    ));

    // @step Then the cached goal state for the session is cleared
    assert!(
        app.agent_view_store().goal_state_for(&sid("s-1")).is_none(),
        "goalCleared must clear the TUI goal cache"
    );

    // @step And the painted footer no longer shows the goal indicator
    let after = painted_footer(&app, &sid("s-1"));
    assert!(
        !after.contains("\u{1F3AF}"),
        "footer must drop the goal indicator; painted: {after:?}"
    );
    assert!(
        after.contains("\u{23E9} auto-continue (0/10)"),
        "with the toggle on the footer falls back to the auto-continue indicator; painted: {after:?}"
    );

    // @step And bare /goal reports that no goal is set
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/goal".to_string()));
    }
    let text = last_scrollback_text(&app, &sid("s-1"))?;
    assert!(
        text.contains("no goal set"),
        "bare /goal after the engine clear must report no goal; got: {text:?}"
    );
    Ok(())
}

#[test]
fn bare_goal_reports_the_live_nudge_and_rejection_counters() -> TestResult {
    // @step Given a TUI session with an active goal whose live snapshot reports 3 nudges used and 2 rejections
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
            nudges_used: 3,
            goal_active: true,
            effective_budget: 15,
            goal_cleared: false,
            done_rejections: 2,
        }),
    ));

    // @step When the user enters "/goal"
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "/goal".to_string()));
    }

    // @step Then the state output shows nudges used 3 and rejections 2
    let text = last_scrollback_text(&app, &sid("s-1"))?;
    assert!(
        text.contains("nudges used: 3, rejections: 2"),
        "bare /goal must show the LIVE counters; got: {text:?}"
    );
    assert!(
        text.contains("budget: 15"),
        "bare /goal must show the effective goal budget; got: {text:?}"
    );

    // @step And the state output no longer hard-codes zero counters
    assert!(
        !text.contains("nudges used: 0, rejections: 0"),
        "the hard-coded zeros must be gone from the Show display; got: {text:?}"
    );
    Ok(())
}
