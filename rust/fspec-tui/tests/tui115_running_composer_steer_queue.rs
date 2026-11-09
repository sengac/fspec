//! TUI-115 — the composer stays visible while the focused session is busy,
//! with steer and queue hints, tagged transcript lines and a queued list.
//!
//! Feature: spec/features/running-composer-steer-and-queue-hints.feature
//!
//! Harness: App + MockBackend, as `rig016_steer_queue_keys.rs`. The
//! session's echo of a submitted input (`StreamChunk::UserInput`) is pushed
//! on the mock's chunk broadcast the way `BackgroundSession` emits it: at
//! once for a steer, and when the queued input's turn starts for a queue.

use codelet_rpc_types::{InputMode, SessionStatus};
use codelet_test_helpers::TestResult;
use crossterm::event::{KeyCode, KeyModifiers};

mod common;
mod tui115_support;
use tui115_support::{
    agent_view_with, drain, key, queued_rows, row_of, rows_containing, screen, session_echoes,
    type_text, COMPACTING_HINT, RUNNING_HINT,
};

#[tokio::test(flavor = "current_thread")]
async fn the_composer_stays_visible_under_the_activity_row_while_a_turn_runs() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, _mock, _session) = agent_view_with(SessionStatus::Running).await?;

    // @step When the user types "also check the logs"
    type_text(&mut app, "also check the logs").await?;
    let rows = screen(&mut app)?;

    // @step Then the activity row is painted above the composer
    let activity = row_of(&rows, "Thinking")?;
    let composer = row_of(&rows, "also check the logs")?;
    assert!(
        activity < composer,
        "activity row {activity} must sit above composer row {composer}"
    );

    // @step And the composer shows "also check the logs"
    assert_eq!(app.navigator().agent.input.value(), "also check the logs");

    // @step And the hint line under the composer reads "Enter steer · Alt+Enter queue · Esc interrupt"
    assert_eq!(row_of(&rows, RUNNING_HINT)?, composer + 1);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn enter_while_a_turn_runs_steers_the_input_and_tags_it_as_a_steer() -> TestResult {
    // @step Given the focused session's turn is running and the composer holds "also check the logs"
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    type_text(&mut app, "also check the logs").await?;

    // @step When the user presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then the input is submitted to the session with mode Steer
    assert_eq!(
        mock.submitted_inputs(),
        [(
            session.clone(),
            "also check the logs".to_string(),
            InputMode::Steer
        )]
    );
    assert_eq!(
        mock.send_input_calls(),
        0,
        "a busy session is not sent a new turn"
    );

    // @step And the transcript shows "You (steer): also check the logs"
    session_echoes(&mut app, &mock, &session, "also check the logs").await?;
    let rows = screen(&mut app)?;
    assert_eq!(
        rows_containing(&rows, "You (steer): also check the logs"),
        1
    );
    assert_eq!(
        rows_containing(&rows, "You: also check the logs"),
        0,
        "the steer is shown once"
    );

    // @step And the composer is empty and still visible
    assert_eq!(app.navigator().agent.input.value(), "");
    let hint = row_of(&rows, RUNNING_HINT)?;
    assert!(
        row_of(&rows, "Thinking")? < hint - 1,
        "the composer row sits between activity and hint"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn alt_enter_while_a_turn_runs_queues_the_input_and_lists_it_until_it_starts() -> TestResult {
    // @step Given the focused session's turn is running and the composer holds "then run the tests"
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    type_text(&mut app, "then run the tests").await?;

    // @step When the user presses Alt+Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::ALT).await?;

    // @step Then the input is submitted to the session with mode Queue
    assert_eq!(
        mock.submitted_inputs(),
        [(
            session.clone(),
            "then run the tests".to_string(),
            InputMode::Queue
        )]
    );

    // @step And the transcript shows "Queued #1: then run the tests"
    let rows = screen(&mut app)?;
    let composer = row_of(&rows, RUNNING_HINT)? - 1;
    let transcript_rows = &rows[..row_of(&rows, "Thinking")?];
    assert_eq!(
        rows_containing(transcript_rows, "Queued #1: then run the tests"),
        1
    );

    // @step And the queued list under the activity row shows "Queued #1: then run the tests"
    let listed = queued_rows(&rows, "Thinking", composer)?;
    assert_eq!(
        rows_containing(&listed, "Queued #1: then run the tests"),
        1,
        "{listed:?}"
    );

    // @step When the session starts a turn for "then run the tests"
    session_echoes(&mut app, &mock, &session, "then run the tests").await?;
    mock.push_status_change(session.clone(), SessionStatus::Running);
    drain(&mut app).await?;

    // @step Then the queued list no longer shows "then run the tests"
    let rows = screen(&mut app)?;
    let composer = row_of(&rows, RUNNING_HINT)? - 1;
    let listed = queued_rows(&rows, "Thinking", composer)?;
    assert_eq!(
        rows_containing(&listed, "then run the tests"),
        0,
        "{listed:?}"
    );
    assert_eq!(
        rows_containing(&rows, "You: then run the tests"),
        0,
        "the start echo is not shown twice"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn queued_inputs_are_numbered_in_submission_order() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, _session) = agent_view_with(SessionStatus::Running).await?;

    // @step When the user queues "first follow-up" and then "second follow-up" with Alt+Enter
    for text in ["first follow-up", "second follow-up"] {
        type_text(&mut app, text).await?;
        key(&mut app, KeyCode::Enter, KeyModifiers::ALT).await?;
    }
    assert_eq!(mock.submitted_inputs().len(), 2);

    // @step Then the queued list shows "Queued #1: first follow-up" above "Queued #2: second follow-up"
    let rows = screen(&mut app)?;
    let composer = row_of(&rows, RUNNING_HINT)? - 1;
    let listed = queued_rows(&rows, "Thinking", composer)?;
    let first = row_of(&listed, "Queued #1: first follow-up")?;
    let second = row_of(&listed, "Queued #2: second follow-up")?;
    assert!(first < second, "{listed:?}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_composer_stays_editable_during_compaction_with_a_queue_only_hint() -> TestResult {
    // @step Given the focused session is compacting
    let (mut app, _mock, _session) = agent_view_with(SessionStatus::Compacting).await?;

    // @step When the user types "then update the changelog"
    type_text(&mut app, "then update the changelog").await?;
    let rows = screen(&mut app)?;

    // @step Then the activity row is painted above the composer
    let activity = row_of(&rows, "Compacting")?;
    let composer = row_of(&rows, "then update the changelog")?;
    assert!(
        activity < composer,
        "activity row {activity} must sit above composer row {composer}"
    );

    // @step And the composer shows "then update the changelog"
    assert_eq!(
        app.navigator().agent.input.value(),
        "then update the changelog"
    );

    // @step And the hint line under the composer reads "Enter or Alt+Enter queue · runs after compaction"
    assert_eq!(row_of(&rows, COMPACTING_HINT)?, composer + 1);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn enter_during_compaction_queues_the_input() -> TestResult {
    // @step Given the focused session is compacting and the composer holds "then update the changelog"
    let (mut app, mock, session) = agent_view_with(SessionStatus::Compacting).await?;
    type_text(&mut app, "then update the changelog").await?;

    // @step When the user presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then the input is submitted to the session with mode Queue
    assert_eq!(
        mock.submitted_inputs(),
        [(
            session,
            "then update the changelog".to_string(),
            InputMode::Queue
        )]
    );
    assert_eq!(
        mock.send_input_calls(),
        0,
        "a compacting session is not sent a new turn"
    );

    // @step And the transcript shows "Queued #1: then update the changelog"
    let rows = screen(&mut app)?;
    let transcript_rows = &rows[..row_of(&rows, "Compacting")?];
    assert_eq!(
        rows_containing(transcript_rows, "Queued #1: then update the changelog"),
        1
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn an_idle_session_shows_the_plain_composer_without_a_busy_hint() -> TestResult {
    // @step Given the focused session is idle
    let (mut app, mock, session) = agent_view_with(SessionStatus::Idle).await?;

    // @step When the user types "hello" and presses Enter
    type_text(&mut app, "hello").await?;
    let typed = screen(&mut app)?;
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;

    // @step Then no activity row or hint line was painted around the composer
    row_of(&typed, "hello")?;
    for busy in ["Thinking", "Compacting", "Alt+Enter queue", "Esc interrupt"] {
        assert_eq!(
            rows_containing(&typed, busy),
            0,
            "{busy:?} painted while idle"
        );
    }

    // @step And "hello" is sent as a new turn and shown without a steer or queue tag
    assert_eq!(
        mock.last_send_input(),
        Some((session.clone(), "hello".to_string()))
    );
    assert!(
        mock.submitted_inputs().is_empty(),
        "an idle session is not steered or queued"
    );
    session_echoes(&mut app, &mock, &session, "hello").await?;
    let rows = screen(&mut app)?;
    assert_eq!(rows_containing(&rows, "You: hello"), 1);
    assert_eq!(rows_containing(&rows, "You (steer)"), 0);
    assert_eq!(rows_containing(&rows, "Queued #"), 0);
    Ok(())
}
