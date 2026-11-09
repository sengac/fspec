//! TUI-117 — stdin prompt recovery and per-session state: a failed send keeps
//! the text with a red error, the command moving on closes the notice and
//! keeps unsent text in the box history, Ctrl+X mutes one command run, and
//! every session keeps its own notice, answer box and chat draft.
//!
//! Feature: spec/features/stdin-prompt-recovery-and-per-session-state.feature
//!
//! Harness: the TUI-116 App + MockBackend support (mounted below) plus
//! `tui117_stdin_prompt_recovery_support`.

use codelet_rpc_types::ExecStdinControl;
use crossterm::event::{KeyCode, KeyModifiers};

mod common;
#[path = "tui116_stdin_notice_answer_box_support/mod.rs"]
mod tui116_support;
mod tui117_stdin_prompt_recovery_support;
use tui116_support::*;
use tui117_stdin_prompt_recovery_support::*;

/// Open the answer box, type `text` and press Enter while the write fails
/// with `reason`; returns once the failed write was attempted.
async fn failed_send(
    app: &mut codelet_fspec_tui::App,
    mock: &common::MockBackend,
    text: &str,
    reason: &str,
) -> TestResult {
    mock.set_write_exec_stdin_error(reason.to_string());
    ctrl(app, 'o').await?;
    type_text(app, text).await?;
    key(app, KeyCode::Enter, KeyModifiers::NONE).await?;
    wait_for(|| mock.write_exec_stdin_calls() == 1, "write_exec_stdin").await?;
    drain(app).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_write_keeps_the_text_in_the_answer_box_and_shows_a_red_error() -> TestResult {
    // @step Given command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, mock, _session) = running_with_request("exec-abc", "git commit").await?;
    // @step And writing to the command fails with "broken pipe"
    // @step And the user has typed "y" into the answer box
    // @step When the user presses Enter
    failed_send(&mut app, &mock, "y", "broken pipe").await?;
    // @step Then the answer box still shows "y"
    let rows = screen(&mut app)?;
    expect_shown(&rows, "> y")?;
    expect_shown(&rows, BOX_HINT)?;
    // @step And a red row in the answer box reads "Not sent: broken pipe"
    expect_red(&mut app, "Not sent: broken pipe")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_interrupt_shows_a_red_error_in_the_answer_box() -> TestResult {
    // @step Given command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, mock, session) = running_with_request("exec-abc", "git commit").await?;
    // @step And interrupting the command fails with "no such process"
    mock.exec_control().fail_with("no such process")?;
    // @step And the answer box is open
    ctrl(&mut app, 'o').await?;
    // @step When the user presses Ctrl+C
    ctrl(&mut app, 'c').await?;
    let expected = vec![(session, "exec-abc".to_string(), ExecStdinControl::Interrupt)];
    assert_eq!(exec_controls(&mock)?, expected);
    // @step Then a red row in the answer box reads "Interrupt failed: no such process"
    expect_shown(&screen(&mut app)?, BOX_HINT)?;
    expect_red(&mut app, "Interrupt failed: no such process")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_the_answer_box_clears_the_error() -> TestResult {
    // @step Given a failed write left "y" and the error "Not sent: broken pipe" in the answer box
    let (mut app, mock, _session) = running_with_request("exec-abc", "git commit").await?;
    failed_send(&mut app, &mock, "y", "broken pipe").await?;
    expect_red(&mut app, "Not sent: broken pipe")?;
    // @step When the user types "e"
    type_text(&mut app, "e").await?;
    // @step Then the answer box shows "ye"
    let rows = screen(&mut app)?;
    expect_shown(&rows, "> ye")?;
    // @step And no error row is shown
    expect_hidden(&rows, "Not sent")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_command_printing_again_closes_the_notice_and_keeps_unsent_text_in_the_box_history(
) -> TestResult {
    // @step Given command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, _mock, session) = running_with_request("exec-abc", "git commit").await?;
    // @step And the user has typed "partial" into the answer box
    ctrl(&mut app, 'o').await?;
    type_text(&mut app, "partial").await?;
    // @step When the command prints again and its stdin request is cleared
    request_cleared(&mut app, &session).await?;
    // @step Then the notice and the answer box close
    let rows = screen(&mut app)?;
    expect_hidden(&rows, NOTICE)?;
    expect_hidden(&rows, BOX_HINT)?;
    // @step And when the command asks again the answer box opens empty
    asks(&mut app, &session, "exec-abc", "git commit").await?;
    ctrl(&mut app, 'o').await?;
    let rows = screen(&mut app)?;
    expect_shown(&rows, BOX_HINT)?;
    expect_hidden(&rows, "partial")?;
    // @step And Shift+Up in the answer box recalls "partial"
    shift(&mut app, KeyCode::Up).await?;
    expect_shown(&screen(&mut app)?, "> partial")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_x_mutes_further_notices_for_the_same_command_run() -> TestResult {
    // @step Given command "git commit" in exec session "exec-abc" may be waiting for input
    let (mut app, _mock, session) = running_with_request("exec-abc", "git commit").await?;
    expect_shown(&screen(&mut app)?, NOTICE)?;
    // @step When the user presses Ctrl+X
    ctrl(&mut app, 'x').await?;
    // @step And exec session "exec-abc" asks for input again
    asks(&mut app, &session, "exec-abc", "git commit").await?;
    // @step Then no notice is shown
    expect_hidden(&screen(&mut app)?, NOTICE)?;
    // @step And when exec session "exec-def" asks for input the notice is shown
    asks(&mut app, &session, "exec-def", "git commit").await?;
    expect_shown(&screen(&mut app)?, NOTICE)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_session_keeps_its_own_notice_answer_box_and_chat_draft() -> TestResult {
    // @step Given sessions "s-1" and "s-2" are open
    let (mut app, mock) = app_with(&["s-1", "s-2"]);
    // @step And command "git commit" in session "s-1" may be waiting for input
    mock.script_exec_stdin_request(sid("s-1"), Some(exec_request("exec-abc", "git commit")));
    // @step And command "cargo build" in session "s-2" may be waiting for input
    mock.script_exec_stdin_request(sid("s-2"), Some(exec_request("exec-cb", "cargo build")));
    // @step And in session "s-1" the chat draft is "one" and the answer box holds "y"
    app.dispatch(codelet_fspec_tui::Action::SessionPrev);
    drain(&mut app).await?;
    type_text(&mut app, "one").await?;
    ctrl(&mut app, 'o').await?;
    type_text(&mut app, "y").await?;
    // @step When the user switches to session "s-2"
    app.dispatch(codelet_fspec_tui::Action::SessionNext);
    drain(&mut app).await?;
    // @step Then the "cargo build" notice is shown with the answer box closed and an empty chat draft
    let rows = screen(&mut app)?;
    expect_shown(&rows, "cargo build may be waiting for input")?;
    expect_hidden(&rows, BOX_HINT)?;
    assert_eq!(chat_draft(&app), "");
    // @step And opening the answer box in session "s-2" shows it empty
    ctrl(&mut app, 'o').await?;
    let rows = screen(&mut app)?;
    expect_shown(&rows, BOX_HINT)?;
    expect_hidden(&rows, "> y")?;
    // @step When the user switches back to session "s-1"
    app.dispatch(codelet_fspec_tui::Action::SessionPrev);
    drain(&mut app).await?;
    // @step Then the answer box is focused and shows "y"
    let rows = screen(&mut app)?;
    expect_shown(&rows, "git commit · your answer goes to its stdin")?;
    expect_shown(&rows, "> y")?;
    // @step And the chat draft is "one"
    assert_eq!(chat_draft(&app), "one");
    Ok(())
}

#[test]
fn without_a_runtime_an_answer_is_put_back_in_the_answer_box() -> TestResult {
    // @step Given command "git commit" in exec session "exec-abc" may be waiting for input and there is no tokio runtime
    let (mut app, _mock) = app_with(&["s-1"]);
    let session = sid("s-1");
    app.agent_view_store_mut()
        .set_session_status(session.clone(), codelet_rpc_types::SessionStatus::Running);
    app.dispatch(codelet_fspec_tui::Action::ChunkReceived(
        session,
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit"),
        },
    ));
    // @step When the user submits "y" to the command
    // Open the answer box (Ctrl+O) and type, all synchronously: no runtime.
    screen(&mut app)?;
    let press = |app: &mut codelet_fspec_tui::App, code, modifiers| {
        app.handle_event(&crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(code, modifiers),
        ));
    };
    press(&mut app, KeyCode::Char('o'), KeyModifiers::CONTROL);
    screen(&mut app)?;
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // BUG-459: views emit actions on the App's channel; with no runtime the
    // test pumps them synchronously, as the run loop would.
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then the answer box still shows "y"
    expect_shown(&screen(&mut app)?, "> y")?;
    // @step And a red row in the answer box reads "Not sent: no tokio runtime: write_exec_stdin not sent"
    expect_red(
        &mut app,
        "Not sent: no tokio runtime: write_exec_stdin not sent",
    )
}
