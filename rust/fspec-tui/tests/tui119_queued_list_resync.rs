//! TUI-119 — the agent view's queued list resyncs from the session's
//! `queued_inputs` on an interrupt and empties on a /clear.
//!
//! Feature: spec/features/queued-list-resyncs-from-interrupts-and-clears-on-reset.feature
//!
//! Harness (App + MockBackend, key/screen helpers): `tui115_support/mod.rs`.
//! The queued list is read from the focused session's store
//! (`busy_inputs.queued()`, the rows the busy composer paints) and, where
//! the session is still busy, from the rendered rows under the activity row.

#![allow(dead_code)]

mod common;
mod tui115_support;

use codelet_test_helpers::TestResult;
use std::error::Error;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{InterruptionReason, SessionId, SessionStatus, StreamChunk};
use crossterm::event::{KeyCode, KeyModifiers};
use tui115_support::*;

/// Queue each of `texts` with Alt+Enter.
async fn queue(app: &mut App, texts: &[&str]) -> Result<(), codelet_test_helpers::TestError> {
    for text in texts {
        type_text(app, text).await?;
        key(app, KeyCode::Enter, KeyModifiers::ALT).await?;
    }
    Ok(())
}

/// The focused session's queued list as its `Queued #n: text` rows.
fn queued_list(app: &App, session: &SessionId) -> Result<Vec<String>, Box<dyn Error>> {
    let context = app
        .agent_view_store()
        .session_context_for(session)
        .ok_or("the focused session has no agent-view context")?;
    Ok(context
        .busy_inputs
        .queued()
        .iter()
        .map(|queued| format!("Queued #{}: {}", queued.n, queued.text))
        .collect())
}

/// Scenario: Interrupted resyncs the queued list from the session's queued inputs
#[tokio::test(flavor = "current_thread")]
async fn interrupted_resyncs_the_queued_list() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "first", "second" and "third" with Alt+Enter
    queue(&mut app, &["first", "second", "third"]).await?;

    // @step When the session sends Interrupted with queued inputs "second" and "third"
    let queued_inputs = vec!["second".to_string(), "third".to_string()];
    mock.push_chunk(session.clone(), StreamChunk::interrupted(queued_inputs));
    drain(&mut app).await?;

    // @step Then the queued list shows "Queued #2: second" and "Queued #3: third"
    let expected = ["Queued #2: second", "Queued #3: third"];
    assert_eq!(queued_list(&app, &session)?, expected);
    let rows = screen(&mut app)?;
    let listed = queued_rows(&rows, "Thinking", row_of(&rows, RUNNING_HINT)? - 1)?;
    assert_eq!(listed.len(), expected.len(), "{listed:?}");
    for (row, text) in listed.iter().zip(expected) {
        assert!(row.contains(text), "{row:?} must show {text:?}");
    }
    // @step And the queued list no longer shows "first"
    assert_eq!(rows_containing(&listed, "first"), 0, "{listed:?}");
    Ok(())
}

/// Scenario: InterruptedWithReason appends a queued input the agent view never listed
#[tokio::test(flavor = "current_thread")]
async fn interrupted_with_reason_appends_an_unlisted_input() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "first" with Alt+Enter
    queue(&mut app, &["first"]).await?;

    // @step When the session sends InterruptedWithReason User with queued inputs "first" and "from another client"
    let chunk = StreamChunk::InterruptedWithReason {
        queued_inputs: vec!["first".to_string(), "from another client".to_string()],
        reason: InterruptionReason::User,
    };
    mock.push_chunk(session.clone(), chunk);
    drain(&mut app).await?;

    // @step Then the queued list shows "Queued #1: first" and "Queued #2: from another client"
    let expected = ["Queued #1: first", "Queued #2: from another client"];
    assert_eq!(queued_list(&app, &session)?, expected);

    // @step When the user queues "next" with Alt+Enter
    queue(&mut app, &["next"]).await?;
    // @step Then the transcript shows "Queued #3: next"
    let rows = screen(&mut app)?;
    let transcript = &rows[..row_of(&rows, "Thinking")?];
    assert_eq!(rows_containing(transcript, "Queued #3: next"), 1);
    Ok(())
}

/// Scenario: A local /clear empties the queued list and restarts its numbering
#[tokio::test(flavor = "current_thread")]
async fn local_clear_empties_the_queued_list() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, _mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "stale follow-up" with Alt+Enter
    queue(&mut app, &["stale follow-up"]).await?;
    assert_eq!(queued_list(&app, &session)?, ["Queued #1: stale follow-up"]);

    // @step When the user runs /clear
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Clear,
    ));
    drain(&mut app).await?;

    // @step Then the queued list is empty
    assert!(queued_list(&app, &session)?.is_empty());

    // @step When the user queues "fresh follow-up" with Alt+Enter
    app.agent_view_store_mut()
        .set_session_status(session.clone(), SessionStatus::Running);
    queue(&mut app, &["fresh follow-up"]).await?;
    // @step Then the queued list shows "Queued #1: fresh follow-up"
    assert_eq!(queued_list(&app, &session)?, ["Queued #1: fresh follow-up"]);
    Ok(())
}

/// Scenario: The session_cleared event empties the queued list
#[tokio::test(flavor = "current_thread")]
async fn session_cleared_event_empties_the_queued_list() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "stale follow-up" with Alt+Enter
    queue(&mut app, &["stale follow-up"]).await?;
    assert_eq!(queued_list(&app, &session)?, ["Queued #1: stale follow-up"]);

    // @step When the session_cleared event arrives for the focused session
    mock.push_session_cleared(session.clone());
    drain(&mut app).await?;

    // @step Then the queued list is empty
    assert!(queued_list(&app, &session)?.is_empty());
    Ok(())
}

/// Scenario: A queued input after a resync is numbered past the highest listed number
#[tokio::test(flavor = "current_thread")]
async fn queued_input_after_resync_is_numbered_past_the_highest() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "first" with Alt+Enter
    queue(&mut app, &["first"]).await?;

    // @step When the session sends Interrupted with queued inputs "from another client" and "first"
    let queued_inputs = vec!["from another client".to_string(), "first".to_string()];
    mock.push_chunk(session.clone(), StreamChunk::interrupted(queued_inputs));
    drain(&mut app).await?;

    // @step Then the queued list shows "Queued #2: from another client" and "Queued #1: first"
    let expected = ["Queued #2: from another client", "Queued #1: first"];
    assert_eq!(queued_list(&app, &session)?, expected);

    // @step When the user queues "next" with Alt+Enter
    queue(&mut app, &["next"]).await?;
    // @step Then the queued list shows "Queued #3: next" last
    let listed = queued_list(&app, &session)?;
    assert_eq!(listed.last().map(String::as_str), Some("Queued #3: next"));
    Ok(())
}

/// Scenario: A transcript-replayed interrupt leaves the queued list alone
#[tokio::test(flavor = "current_thread")]
async fn transcript_replayed_interrupt_leaves_the_queued_list_alone() -> TestResult {
    // @step Given the focused session's turn is running
    let (mut app, mock, session) = agent_view_with(SessionStatus::Running).await?;
    // @step And the user queued "first" with Alt+Enter
    queue(&mut app, &["first"]).await?;

    // @step When the transcript replays an interrupt that carries no queued inputs
    mock.push_buffered_output(StreamChunk::InterruptedWithReason {
        queued_inputs: Vec::new(),
        reason: InterruptionReason::User,
    });
    // @step And the live Interrupted carries queued input "first"
    let live = StreamChunk::interrupted(vec!["first".to_string()]);
    mock.push_chunk(session.clone(), live);
    drain(&mut app).await?;

    // @step Then the queued list shows "Queued #1: first"
    let rows = screen(&mut app)?;
    assert_eq!(rows_containing(&rows, "Interrupted by user"), 1, "replayed");
    assert_eq!(queued_list(&app, &session)?, ["Queued #1: first"]);
    Ok(())
}
