//! TUI-116 — a quiet command's stdin request shows a notice above the
//! composer instead of taking it over, and Ctrl+O opens a separate answer box.
//!
//! Feature: spec/features/stdin-notice-and-answer-box.feature
//!
//! Harness: App + MockBackend (`tui116_stdin_notice_answer_box_support`).
//! The request reaches the store as the exec-stdin push chunk
//! (`StreamChunk::ExecStdinRequest`), the way the daemon delivers it.

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{ExecStdinControl, HitlQuestion, HitlRequest};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::hitl_route;
mod tui116_stdin_notice_answer_box_support;
use tui116_stdin_notice_answer_box_support::*;

fn hitl_request() -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "q-1".to_string(),
            header: "Header".to_string(),
            question: "Which branch?".to_string(),
            options: vec![],
        }],
    }
}

/// Render one wide frame and return the text of each pane rect.
fn pane_texts(app: &mut App, panes: &[Rect]) -> Vec<String> {
    let area = Rect::new(0, 0, 240, 40);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    panes
        .iter()
        .map(|pane| rows_of(&buffer, pane.intersection(area)).join("\n"))
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn the_notice_sits_above_the_composer_and_leaves_the_chat_draft_in_focus() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" may be waiting for input
    let (mut app, _mock, _session) = running_with_request("exec-abc", "git commit").await?;
    // @step When the user types "hello"
    type_text(&mut app, "hello").await?;
    // @step Then the row above the activity row reads "git commit may be waiting for input · Ctrl+O answer · Ctrl+X dismiss"
    let rows = screen(&mut app)?;
    let notice = row_of(&rows, NOTICE)?;
    let activity = row_of(&rows, "Thinking")?;
    assert_eq!(
        notice + 1,
        activity,
        "the notice sits on top of:\n{}",
        rows.join("\n")
    );
    // @step And the chat draft is "hello"
    assert_eq!(chat_draft(&app), "hello");
    expect_shown(&rows, "> hello")
}

#[tokio::test(flavor = "current_thread")]
async fn ctrl_o_opens_a_separate_answer_box_with_its_own_buffer() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" may be waiting for input
    let (mut app, _mock, _session) = running_with_request("exec-abc", "git commit").await?;
    // @step And the chat draft is "hello"
    type_text(&mut app, "hello").await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step When the user presses Ctrl+O and types "y"
    ctrl(&mut app, 'o').await?;
    type_text(&mut app, "y").await?;
    // @step Then the answer box shows "y" with the hint "Enter send · Esc chat · Ctrl+C interrupt · Ctrl+D end input"
    let rows = screen(&mut app)?;
    expect_shown(&rows, "> y")?;
    expect_shown(&rows, BOX_HINT)?;
    // @step And the chat draft is still "hello"
    assert_eq!(chat_draft(&app), "hello");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn enter_in_the_answer_box_writes_the_line_to_the_command_and_sends_no_chat_message(
) -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, mock, session) = running_with_request("exec-abc", "git commit").await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step And the user has typed "y" into the answer box
    ctrl(&mut app, 'o').await?;
    type_text(&mut app, "y").await?;
    // @step When the user presses Enter
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then the backend writes "y" to exec session "exec-abc"
    wait_for(|| mock.write_exec_stdin_calls() == 1, "write_exec_stdin").await?;
    let expected = vec![(session, "exec-abc".to_string(), "y".to_string())];
    assert_eq!(mock.write_exec_stdin_log(), expected);
    // @step And no chat message is sent
    assert!(
        mock.submitted_inputs().is_empty(),
        "no chat input may be submitted"
    );
    assert_eq!(mock.send_input_calls(), 0);
    // @step And the notice and the answer box close
    let rows = screen(&mut app)?;
    expect_hidden(&rows, NOTICE)?;
    expect_hidden(&rows, BOX_HINT)
}

#[tokio::test(flavor = "current_thread")]
async fn esc_in_the_answer_box_returns_to_the_chat_draft_and_keeps_both_drafts() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" may be waiting for input
    let (mut app, mock, _session) = running_with_request("exec-abc", "git commit").await?;
    // @step And the chat draft is "draft"
    type_text(&mut app, "draft").await?;
    // @step And the user has typed "n" into the answer box
    ctrl(&mut app, 'o').await?;
    type_text(&mut app, "n").await?;
    expect_shown(&screen(&mut app)?, BOX_HINT)?;
    // @step When the user presses Esc
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE).await?;
    // @step Then the chat draft is "draft" and the notice is still shown
    assert_eq!(chat_draft(&app), "draft");
    let rows = screen(&mut app)?;
    expect_shown(&rows, NOTICE)?;
    expect_shown(&rows, "> draft")?;
    // @step And the session turn is not interrupted
    assert_eq!(mock.interrupt_calls(), 0);
    // @step And pressing Ctrl+O again shows "n" in the answer box
    ctrl(&mut app, 'o').await?;
    expect_shown(&screen(&mut app)?, "> n")
}

#[tokio::test(flavor = "current_thread")]
async fn ctrl_c_in_the_answer_box_interrupts_only_the_command() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, mock, session) = running_with_request("exec-abc", "git commit").await?;
    // @step And the answer box is open
    ctrl(&mut app, 'o').await?;
    expect_shown(&screen(&mut app)?, BOX_HINT)?;
    // @step When the user presses Ctrl+C
    ctrl(&mut app, 'c').await?;
    // @step Then the backend sends an interrupt to exec session "exec-abc"
    let expected = vec![(session, "exec-abc".to_string(), ExecStdinControl::Interrupt)];
    assert_eq!(exec_controls(&mock)?, expected);
    // @step And the session turn is not interrupted
    assert_eq!(mock.interrupt_calls(), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn ctrl_d_in_the_answer_box_ends_the_commands_input() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, mock, session) = running_with_request("exec-abc", "git commit").await?;
    // @step And the answer box is open
    ctrl(&mut app, 'o').await?;
    expect_shown(&screen(&mut app)?, BOX_HINT)?;
    // @step When the user presses Ctrl+D
    ctrl(&mut app, 'd').await?;
    // @step Then the backend sends end of input to exec session "exec-abc"
    let expected = vec![(session, "exec-abc".to_string(), ExecStdinControl::Eof)];
    assert_eq!(exec_controls(&mock)?, expected);
    // @step And the TUI keeps running
    assert!(
        !app.should_quit(),
        "Ctrl+D in the answer box must not quit the TUI"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn esc_with_the_chat_focused_still_interrupts_the_agent_turn() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" may be waiting for input
    let (mut app, mock, _session) = running_with_request("exec-abc", "git commit").await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step When the user presses Esc
    key(&mut app, KeyCode::Esc, KeyModifiers::NONE).await?;
    // @step Then the session turn is interrupted
    wait_for(|| mock.interrupt_calls() == 1, "session interrupt").await?;
    // @step And nothing is sent to the command
    assert_eq!(mock.write_exec_stdin_calls(), 0);
    assert!(
        exec_controls(&mock)?.is_empty(),
        "no exec-stdin control may be sent"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn ctrl_x_dismisses_the_notice_without_touching_the_command() -> TestResult {
    // @step Given the focused session's turn is running
    // @step And command "git commit" may be waiting for input
    let (mut app, mock, session) = running_with_request("exec-abc", "git commit").await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step When the user presses Ctrl+X
    ctrl(&mut app, 'x').await?;
    // @step Then the notice is no longer shown
    expect_hidden(&screen(&mut app)?, NOTICE)?;
    assert!(app.agent_view_store().exec_stdin_for(&session).is_none());
    // @step And nothing is sent to the command
    assert_eq!(mock.write_exec_stdin_calls(), 0);
    assert!(
        exec_controls(&mock)?.is_empty(),
        "no exec-stdin control may be sent"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_hitl_prompt_wins_over_the_notice() -> TestResult {
    // @step Given the focused session has a pending HITL request
    let (mut app, _mock) = app_with(&["s-1"]);
    let session = sid("s-1");
    app.dispatch(Action::HitlPromptFetched {
        session_id: session.clone(),
        route: hitl_route(&session),
        request: hitl_request(),
    });
    // @step When command "git commit" may be waiting for input
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit"),
        },
    ));
    drain(&mut app).await?;
    // @step Then the input area shows the HITL prompt
    let rows = screen(&mut app)?;
    expect_shown(&rows, "Which branch?")?;
    // @step And the notice is not shown
    expect_hidden(&rows, "may be waiting for input")?;
    // @step And the notice is shown once the HITL prompt is cleared and the request arrives again
    app.dispatch(Action::PauseCleared(session.clone()));
    app.dispatch(Action::ChunkReceived(
        session,
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit"),
        },
    ));
    drain(&mut app).await?;
    expect_shown(&screen(&mut app)?, NOTICE)
}

#[tokio::test(flavor = "current_thread")]
async fn ghost_panes_show_no_notice() -> TestResult {
    // @step Given agent session A has command "cmd-a" waiting for input
    let (mut app, _mock) = app_with(&["s-1", "s-2"]);
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request("exec-a", "cmd-a"),
        },
    ));
    // @step And agent session B is focused while agent session A is a ghost pane
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(
            session,
            "/mux board, agent, agent".to_string(),
        ));
    }
    drain(&mut app).await?;
    pane_texts(&mut app, &[]);
    let panes = app.navigator().mux.pane_rects().to_vec();
    let (a_pane, b_pane) = match panes.as_slice() {
        [_, a, b, ..] => (*a, *b),
        other => return Err(format!("expected three panes, got {other:?}").into()),
    };
    app.navigator_mut().mux.set_focus(2);
    // @step When the agent view paints all panes
    let texts = pane_texts(&mut app, &[a_pane, b_pane]);
    // @step Then neither pane shows a notice for "cmd-a"
    for text in &texts {
        assert!(
            !text.contains("cmd-a may be waiting"),
            "no notice in:\n{text}"
        );
    }
    // @step And pane A shows the notice for "cmd-a" once it is focused
    app.navigator_mut().mux.set_focus(1);
    let texts = pane_texts(&mut app, &[a_pane]);
    let focused = texts.concat();
    assert!(
        focused.contains("cmd-a may be waiting"),
        "focused pane A:\n{focused}"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn a_re_probe_with_no_request_closes_the_notice_and_the_answer_box() -> TestResult {
    // @step Given command "git commit" in exec session "exec-gone" may be waiting for input
    let (mut app, mock) = app_with(&["s-1", "s-2"]);
    mock.script_exec_stdin_request(sid("s-1"), Some(exec_request("exec-gone", "git commit")));
    app.dispatch(Action::SessionPrev);
    drain(&mut app).await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step And the answer box is open
    ctrl(&mut app, 'o').await?;
    expect_shown(&screen(&mut app)?, BOX_HINT)?;
    // @step When the exec session has exited and the agent view re-probes the backend
    mock.script_exec_stdin_request(sid("s-1"), None);
    app.dispatch(Action::SessionNext);
    app.dispatch(Action::SessionPrev);
    drain(&mut app).await?;
    // @step Then the notice and the answer box are no longer shown
    let rows = screen(&mut app)?;
    expect_hidden(&rows, NOTICE)?;
    expect_hidden(&rows, BOX_HINT)?;
    // @step And the chat composer is shown again
    expect_shown(&rows, "Type a message")
}
