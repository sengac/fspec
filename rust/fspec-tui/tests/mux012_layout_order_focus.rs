//! MUX-012 — Shift+Left/Right move to the previous/next mux pane in the
//! layout string's reading order, stop at both ends, never rotate the
//! agent window, and Shift+Right on a last agent pane prompts for a new
//! agent.
//!
//! Feature: spec/features/multiple-agent-panes-with-grouped-agent-view-cycling.feature
//!
//! Every test drives the real `App` over a `MockBackend` on a paused tokio
//! clock (TEST-042): sessions open through `Action::SessionCreated`, the
//! layout through `/mux <layout>`, keys through `App::handle_event`, and
//! frames are drawn on a `TestBackend` so the fit check sees a real body.

use std::sync::Arc;

use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;
use common::{MockBackend, TaskOutcome};

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Run every pending task and queued action until nothing moves.
async fn drain(app: &mut App) -> Result<(), TestError> {
    loop {
        let mut moved = false;
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
            moved = true;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            moved = true;
        }
        if !moved {
            return Ok(());
        }
    }
}

fn render(app: &mut App) -> Result<(), TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 40)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(())
}

/// An app with sessions `s-1..s-n` open and mux on with `layout` at
/// 120x40.
async fn mux_app(n: usize, layout: &str) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for i in 1..=n {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain(&mut app).await?;
    render(&mut app)?;
    let session = app.current_session().or_fail("a current session")?;
    app.dispatch(Action::InputSubmitted(session, format!("/mux {layout}")));
    drain(&mut app).await?;
    render(&mut app)?;
    if app.navigator().active_view != ViewMode::Mux {
        return Err(TestError::from(format!("/mux {layout} did not enter mux")));
    }
    Ok((app, mock))
}

async fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<(), TestError> {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

async fn shift(app: &mut App, code: KeyCode) -> Result<(), TestError> {
    press(app, code, KeyModifiers::SHIFT).await
}

fn focus(app: &App) -> usize {
    app.navigator().mux.focus()
}

fn kinds(app: &App) -> Vec<MuxPaneKind> {
    app.navigator().mux.effective_panes().to_vec()
}

fn window(app: &App) -> Vec<SessionId> {
    app.navigator().mux.window_session_ids()
}

fn dialog_open(app: &App) -> bool {
    app.compositor().contains(CREATE_SESSION_DIALOG_ID)
}

/// Put the agent window on `first..` and focus the agent pane at
/// `pane` (the chip path rotates the window to a session).
fn window_on(app: &mut App, first: &str, pane: usize) {
    let _ = app
        .navigator_mut()
        .mux
        .focus_agent_pane_for_session(&sid(first));
    app.navigator_mut().mux.set_focus(pane);
}

/// Scenario: Shift+Right and Shift+Left walk the panes in layout order and stop at both ends
#[tokio::test(start_paused = true)]
async fn shift_right_and_left_walk_the_panes_in_layout_order_and_stop_at_both_ends() -> TestResult {
    // @step Given mux mode is active with the layout "[board, agent], [agent, agent]"
    let (mut app, _mock) = mux_app(3, "[board, agent], [agent, agent]").await?;
    // @step And three agent sessions are open and the Board pane is focused
    app.navigator_mut().mux.set_focus(0);
    assert_eq!(
        kinds(&app),
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Agent,
            MuxPaneKind::Agent
        ]
    );
    // @step When I press Shift+Right three times
    let mut visited = Vec::new();
    for _ in 0..3 {
        shift(&mut app, KeyCode::Right).await?;
        visited.push(focus(&app));
    }
    // @step Then agent 1, agent 2 and agent 3 are focused in turn
    assert_eq!(
        visited,
        vec![1, 2, 3],
        "layout order is board, agent, agent, agent"
    );
    // @step When I press Shift+Right again
    shift(&mut app, KeyCode::Right).await?;
    // @step Then agent 3 stays focused and the new-agent dialog is shown
    assert_eq!(focus(&app), 3, "the focus never wraps past the last pane");
    assert!(
        dialog_open(&app),
        "the last pane is an agent: Shift+Right prompts"
    );
    // @step When I cancel the dialog and press Shift+Left four times
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE).await?;
    assert!(!dialog_open(&app), "Esc cancels the new-agent dialog");
    let mut back = Vec::new();
    for _ in 0..4 {
        shift(&mut app, KeyCode::Left).await?;
        back.push(focus(&app));
    }
    // @step Then agent 2, agent 1 and the Board pane are focused in turn
    assert_eq!(back[..3], [2, 1, 0]);
    // @step And the fourth Shift+Left leaves the Board pane focused
    assert_eq!(back[3], 0, "the focus never wraps past the first pane");
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    Ok(())
}

/// Scenario: Shift+Right follows the layout order through a nesting three levels deep
#[tokio::test(start_paused = true)]
async fn shift_right_follows_the_layout_order_through_a_nesting_three_levels_deep() -> TestResult {
    // @step Given mux mode is active with the layout "board, [agent, [files, [checkpoints, agent]]]"
    let (mut app, _mock) = mux_app(2, "board, [agent, [files, [checkpoints, agent]]]").await?;
    // @step And two agent sessions are open and the Board pane is focused
    app.navigator_mut().mux.set_focus(0);
    assert_eq!(app.agent_view_store().open_sessions().len(), 2);
    // @step When I press Shift+Right four times
    let mut visited = Vec::new();
    for _ in 0..4 {
        shift(&mut app, KeyCode::Right).await?;
        visited.push(kinds(&app)[focus(&app)]);
    }
    // @step Then agent 1, the files pane, the checkpoints pane and agent 2 are focused in turn
    assert_eq!(
        visited,
        vec![
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
            MuxPaneKind::Agent
        ]
    );
    assert_eq!(focus(&app), 4);
    // @step And the files pane is focused after agent 1 although it lies below agent 1
    let rects = app.navigator().mux.pane_rects().to_vec();
    assert!(
        rects[2].y >= rects[1].bottom(),
        "files ({:?}) lies below agent 1 ({:?})",
        rects[2],
        rects[1]
    );
    Ok(())
}

/// Scenario: Shift+Right on the last agent pane prompts for a new agent instead of rotating the agent window
#[tokio::test(start_paused = true)]
async fn shift_right_on_the_last_agent_pane_prompts_instead_of_rotating_the_agent_window(
) -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, mock) = mux_app(3, "board, agent, agent").await?;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the agent window shows agent 1 and agent 2 with the agent 2 pane focused
    window_on(&mut app, "s-1", 2);
    assert_eq!(window(&app), vec![sid("s-1"), sid("s-2")]);
    // @step When I press Shift+Right
    shift(&mut app, KeyCode::Right).await?;
    // @step Then the new-agent dialog is shown
    assert!(
        dialog_open(&app),
        "the last pane is an agent: Shift+Right prompts"
    );
    // @step And the agent window still shows agent 1 and agent 2
    assert_eq!(window(&app), vec![sid("s-1"), sid("s-2")], "no rotation");
    // @step When I confirm the new-agent dialog
    mock.script_create_session(sid("s-4"));
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step Then a fourth agent session is created WITHOUT work-unit attachment
    assert_eq!(app.agent_view_store().open_sessions().len(), 4);
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid("s-4")),
        None
    );
    // @step And the agent window shows agent 3 and agent 4
    assert_eq!(window(&app), vec![sid("s-3"), sid("s-4")]);
    // @step And the agent 4 pane is focused
    assert_eq!(focus(&app), 2);
    assert_eq!(app.navigator().mux.focused_session_id(), Some(sid("s-4")));
    Ok(())
}

/// Scenario: Cancelling the new-agent dialog leaves the panes, the focus and the agent window unchanged
#[tokio::test(start_paused = true)]
async fn cancelling_the_new_agent_dialog_leaves_the_panes_focus_and_window_unchanged() -> TestResult
{
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = mux_app(3, "board, agent, agent").await?;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the agent window shows agent 1 and agent 2 with the agent 2 pane focused
    window_on(&mut app, "s-1", 2);
    let panes_before = kinds(&app);
    // @step When I press Shift+Right and cancel the new-agent dialog
    shift(&mut app, KeyCode::Right).await?;
    assert!(dialog_open(&app));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE).await?;
    assert!(!dialog_open(&app));
    // @step Then no agent session is created
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    assert!(!app.navigator().mux.pending_new_agent());
    // @step And the agent window still shows agent 1 and agent 2
    assert_eq!(window(&app), vec![sid("s-1"), sid("s-2")]);
    assert_eq!(kinds(&app), panes_before);
    // @step And the agent 2 pane is still focused
    assert_eq!(focus(&app), 2);
    assert_eq!(app.navigator().mux.focused_session_id(), Some(sid("s-2")));
    Ok(())
}

/// Scenario: Shift+Right on a last pane that is not an agent does nothing
#[tokio::test(start_paused = true)]
async fn shift_right_on_a_last_pane_that_is_not_an_agent_does_nothing() -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and files
    let (mut app, _mock) = mux_app(1, "board, agent, files").await?;
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step And the files pane is focused
    app.navigator_mut().mux.set_focus(2);
    assert_eq!(kinds(&app)[2], MuxPaneKind::ChangedFiles);
    // @step When I press Shift+Right
    shift(&mut app, KeyCode::Right).await?;
    // @step Then no new-agent dialog is shown
    assert!(!dialog_open(&app));
    assert!(!app.navigator().mux.pending_new_agent());
    // @step And the files pane is still focused
    assert_eq!(focus(&app), 2);
    Ok(())
}

/// Scenario: Shift+Left moves to the previous pane without rotating the agent window
#[tokio::test(start_paused = true)]
async fn shift_left_moves_to_the_previous_pane_without_rotating_the_agent_window() -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = mux_app(3, "board, agent, agent").await?;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the agent window shows agent 2 and agent 3 with the agent 3 pane focused
    window_on(&mut app, "s-3", 2);
    assert_eq!(window(&app), vec![sid("s-2"), sid("s-3")]);
    // @step When I press Shift+Left
    shift(&mut app, KeyCode::Left).await?;
    // @step Then the agent 2 pane is focused
    assert_eq!(focus(&app), 1);
    assert_eq!(app.navigator().mux.focused_session_id(), Some(sid("s-2")));
    // @step And the agent window still shows agent 2 and agent 3
    assert_eq!(window(&app), vec![sid("s-2"), sid("s-3")], "no rotation");
    // @step When I press Shift+Left again
    shift(&mut app, KeyCode::Left).await?;
    // @step Then the Board pane is focused
    assert_eq!(focus(&app), 0);
    Ok(())
}
