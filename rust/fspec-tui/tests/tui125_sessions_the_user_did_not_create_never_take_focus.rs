//! TUI-125 — sessions the user did not create never take focus.
//!
//! Feature: spec/features/sessions-the-user-did-not-create-never-take-focus.feature
//!
//! A sub-agent spawned with AgentManager (or a session created from another
//! client) reaches the TUI as the daemon's session-created broadcast, which
//! the bootstrap subscriber folds into `Action::SessionObserved` (pinned by
//! RPC-385). These tests dispatch that action into the real `App` over a
//! `MockBackend` and drive the user's own steps (typing, Shift+arrows, the
//! New Agent dialog) through `App::handle_event`, on a paused tokio clock.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// An agent-view App with one session per name, each created the way the
/// New Agent dialog's answer arrives (`Action::SessionCreated`); the last
/// one is current.
async fn app_with(sessions: &[&str]) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    for session in sessions {
        app.dispatch(Action::SessionCreated(sid(session)));
    }
    drain(&mut app).await?;
    Ok((app, mock))
}

/// Run every parked task and queued action until the App is quiet.
async fn drain(app: &mut App) -> TestResult {
    for _ in 0..50 {
        let mut progressed = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            progressed = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            progressed = true;
        }
        if !progressed {
            return Ok(());
        }
    }
    Ok(())
}

/// The session-created broadcast payload for `id`, as the daemon sends it.
fn announced(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: String::new(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
        work_unit_focus: Default::default(),
    }
}

/// The daemon announces `id` (the subscriber's fold of its broadcast).
async fn announce(app: &mut App, id: &str) -> TestResult {
    app.dispatch(Action::SessionObserved(announced(id)));
    drain(app).await?;
    render(app);
    Ok(())
}

/// Paint one frame, as the run loop does between events.
fn render(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
}

async fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    render(app);
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

async fn type_text(app: &mut App, text: &str) -> TestResult {
    for character in text.chars() {
        press(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

fn composer(app: &App) -> String {
    app.navigator().agent.input.value()
}

fn tab_open(app: &App, id: &str) -> bool {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .any(|context| context.id == sid(id))
}

/// MUX-010: the layout string of a space-separated pane list
/// (`board agent agent` -> `board, agent, agent`).
fn layout_of(panes: &str) -> String {
    panes.split_whitespace().collect::<Vec<_>>().join(", ")
}

/// Enter mux mode with `/mux <panes>` (submitted like a typed command).
async fn enter_mux(app: &mut App, panes: &str) -> TestResult {
    let session = app.current_session().or_fail("a session is open")?;
    app.dispatch(Action::InputSubmitted(
        session,
        format!("/mux {}", layout_of(panes)),
    ));
    drain(app).await?;
    render(app);
    Ok(())
}

/// Click inside the Nth rendered mux pane (production click-to-focus).
async fn click_pane(app: &mut App, pane_index: usize) -> TestResult {
    render(app);
    let rect = app
        .navigator()
        .mux
        .pane_rects()
        .get(pane_index)
        .copied()
        .or_fail("the mux pane is rendered")?;
    let _ = app.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 1,
        row: rect.y + 1,
        modifiers: KeyModifiers::NONE,
    }));
    drain(app).await
}

fn focused_pane(app: &App) -> Option<MuxPaneKind> {
    let mux = &app.navigator().mux;
    mux.effective_panes().get(mux.focus()).copied()
}

#[tokio::test(start_paused = true)]
async fn a_sub_agent_spawned_by_the_viewed_agent_opens_a_tab_without_taking_focus() -> TestResult {
    // @step Given I am viewing agent A with a draft typed in the composer
    let (mut app, mock) = app_with(&["agent-a"]).await?;
    type_text(&mut app, "half a thought").await?;
    let pointer_moves = mock.set_active_session_calls();

    // @step When A's AgentManager spawns sub-agent S and the daemon announces S
    announce(&mut app, "sub-s").await?;

    // @step Then a tab for S is open
    assert!(tab_open(&app, "sub-s"));
    // @step And the agent view still shows A
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    // @step And the composer still holds A's draft
    assert_eq!(composer(&app), "half a thought");
    // @step And the daemon's active session is not moved to S
    assert_eq!(mock.set_active_session_calls(), pointer_moves);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_sub_agent_spawned_by_the_viewed_bot_opens_a_tab_without_taking_focus() -> TestResult {
    // @step Given I am viewing bot B
    let (mut app, mock) = app_with(&[]).await?;
    mock.script_create_session(sid("bot-b"));
    app.dispatch(Action::CreateBotSubmitted { isolated: false });
    drain(&mut app).await?;
    assert!(app.agent_view_store().is_bot_session(&sid("bot-b")));

    // @step When B's AgentManager spawns sub-agent S and the daemon announces S
    announce(&mut app, "sub-s").await?;

    // @step Then a tab for S is open
    assert!(tab_open(&app, "sub-s"));
    // @step And the agent view still shows B
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    assert_eq!(app.current_session(), Some(sid("bot-b")));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_sub_agent_spawned_while_the_board_is_shown_leaves_the_board_and_the_current_session_alone(
) -> TestResult {
    // @step Given the board is shown and agent A is the current session
    let (mut app, _mock) = app_with(&["agent-a"]).await?;
    app.navigator_mut().active_view = ViewMode::Board;

    // @step When a sub-agent S is announced by the daemon
    announce(&mut app, "sub-s").await?;

    // @step Then the board is still shown
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    // @step And A is still the current session
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    assert!(tab_open(&app, "sub-s"));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn in_mux_view_a_spawned_sub_agent_fills_a_free_agent_pane_while_the_focused_agent_pane_keeps_focus(
) -> TestResult {
    // @step Given mux view "board agent agent" with agent A focused in the first agent pane
    let (mut app, mock) = app_with(&["agent-a"]).await?;
    enter_mux(&mut app, "board agent agent").await?;
    click_pane(&mut app, 1).await?;
    assert_eq!(
        app.navigator().mux.focused_session_id(),
        Some(sid("agent-a"))
    );
    let pointer_moves = mock.set_active_session_calls();

    // @step When a sub-agent S is announced by the daemon
    announce(&mut app, "sub-s").await?;

    // @step Then the second agent pane shows S
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("agent-a"), sid("sub-s")]
    );
    // @step And the focused pane still shows A
    assert_eq!(
        app.navigator().mux.focused_session_id(),
        Some(sid("agent-a"))
    );
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    assert_eq!(mock.set_active_session_calls(), pointer_moves);
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn in_mux_view_a_spawned_sub_agent_added_before_the_focused_board_pane_does_not_move_focus(
) -> TestResult {
    // @step Given mux view "agent agent board" with one agent open and the board pane focused
    let (mut app, _mock) = app_with(&["agent-a"]).await?;
    enter_mux(&mut app, "agent agent board").await?;
    click_pane(&mut app, 1).await?;
    assert_eq!(focused_pane(&app), Some(MuxPaneKind::Board));

    // @step When a sub-agent S is announced by the daemon
    announce(&mut app, "sub-s").await?;

    // @step Then the second agent pane shows S
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("agent-a"), sid("sub-s")]
    );
    // @step And the board pane is still focused
    assert_eq!(focused_pane(&app), Some(MuxPaneKind::Board));
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_sub_agent_spawned_while_the_mux_new_agent_prompt_is_open_does_not_take_the_prompts_pane(
) -> TestResult {
    // @step Given mux view with agent A focused and the Shift+Right new-agent prompt open
    let (mut app, mock) = app_with(&["agent-a"]).await?;
    enter_mux(&mut app, "board agent").await?;
    click_pane(&mut app, 1).await?;
    press(&mut app, KeyCode::Right, KeyModifiers::SHIFT).await?;
    assert!(app.navigator().mux.pending_new_agent());
    assert!(app
        .compositor()
        .contains(codelet_fspec_tui::CREATE_SESSION_DIALOG_ID));

    // @step When a sub-agent S is announced by the daemon before I confirm the prompt
    announce(&mut app, "sub-s").await?;

    // @step Then the focused pane still shows A
    assert_eq!(
        app.navigator().mux.focused_session_id(),
        Some(sid("agent-a"))
    );
    assert_eq!(app.current_session(), Some(sid("agent-a")));

    // @step When I confirm the prompt and my new agent N is created
    mock.script_create_session(sid("new-n"));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    render(&mut app);
    assert_eq!(mock.create_session_calls(), 1);

    // @step Then N is the current session and the focused pane shows N
    assert_eq!(app.current_session(), Some(sid("new-n")));
    assert_eq!(app.navigator().mux.focused_session_id(), Some(sid("new-n")));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn a_session_the_user_creates_opens_focused_even_when_the_daemon_announces_it_first(
) -> TestResult {
    // @step Given I am viewing agent A
    let (mut app, mock) = app_with(&["agent-a"]).await?;
    mock.script_create_session(sid("new-n"));

    // @step When I create agent N from the New Agent dialog and the daemon announces N before the creation answer arrives
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    app.dispatch(Action::SessionObserved(announced("new-n")));
    drain(&mut app).await?;
    assert_eq!(mock.create_session_calls(), 1);

    // @step Then N is the current session
    assert_eq!(app.current_session(), Some(sid("new-n")));
    assert_eq!(mock.last_set_active_session(), Some(sid("new-n")));

    // @step And a repeated creation answer for N after I go back to A leaves A current
    press(&mut app, KeyCode::Left, KeyModifiers::SHIFT).await?;
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    app.dispatch(Action::SessionCreated(sid("new-n")));
    drain(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("agent-a")));
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn the_first_session_to_appear_when_none_is_open_becomes_the_current_session() -> TestResult {
    // @step Given no session is open
    let (mut app, _mock) = app_with(&[]).await?;
    assert_eq!(app.current_session(), None);

    // @step When a session created from another client is announced by the daemon
    announce(&mut app, "remote-r").await?;

    // @step Then that session is the current session
    assert_eq!(app.current_session(), Some(sid("remote-r")));
    Ok(())
}
