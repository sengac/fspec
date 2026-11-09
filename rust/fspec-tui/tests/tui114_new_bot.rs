//! TUI-114 — New Bot starts an orchestrator bot like New Agent.
//!
//! Feature: spec/features/tui-new-bot.feature
//!
//! Harness: App + MockBackend rendered at 160x30, as the RPC-445/448 board
//! tests do; the bot role text comes from codelet_sessions::bot.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
use codelet_sessions::bot::{is_bot_role, BOT_ROLE_MARKER, ORCHESTRATOR_ROLE};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

async fn board_app() -> Result<(App, Arc<MockBackend>), codelet_test_helpers::TestError> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    drain(&mut app).await?;
    Ok((app, mock))
}

async fn drain(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

async fn press(app: &mut App, code: KeyCode) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&key(code));
    drain(app).await?;
    Ok(())
}

fn render(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    render_at(app, 160)
}

fn render_at(app: &mut App, width: u16) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(width, 30)).or_fail("terminal")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn rows(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect()
        })
        .collect()
}

fn bar_row(buf: &Buffer) -> Result<String, codelet_test_helpers::TestError> {
    Ok(rows(buf)
        .into_iter()
        .find(|row| row.contains("[ New Agent ]"))
        .or_fail("a menu bar row with [ New Agent ]")?)
}

/// Board -> b -> Enter: a bot session `id` is created and opened.
async fn create_bot(
    app: &mut App,
    mock: &MockBackend,
    id: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    mock.script_create_session(SessionId::new(id));
    press(app, KeyCode::Char('b')).await?;
    press(app, KeyCode::Enter).await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_board_offers_new_bot_beside_new_agent() -> TestResult {
    // @step Given the board view is showing
    let (mut app, _mock) = board_app().await?;

    // @step When the board menu bar is painted
    let buf = render(&mut app)?;

    // @step Then Zone C shows "[ New Bot ]" beside "[ New Agent ]"
    let bar = bar_row(&buf)?;
    assert!(bar.contains("[ New Bot ]"), "{bar}");

    // @step And the Kanban menu lists a "New Bot" row on key "b"
    let row = codelet_fspec_tui::components::menu_bar::items::KANBAN
        .iter()
        .find(|entry| entry.label == "New Bot")
        .or_fail("Kanban New Bot row")?;
    assert_eq!(row.key, "b");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pressing_b_on_the_board_opens_the_new_bot_dialog() -> TestResult {
    // @step Given the board view is showing
    let (mut app, _mock) = board_app().await?;

    // @step When the user presses "b"
    press(&mut app, KeyCode::Char('b')).await?;

    // @step Then the create-session dialog opens titled "New Bot"
    let text = rows(&render(&mut app)?).join("\n");
    assert!(text.contains("Start New Bot?"), "{text}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn confirming_new_bot_creates_an_orchestrator_session_and_opens_it() -> TestResult {
    // @step Given the New Bot dialog is open
    let (mut app, mock) = board_app().await?;
    mock.script_create_session(SessionId::new("s-bot"));
    press(&mut app, KeyCode::Char('b')).await?;

    // @step When the user confirms "Start"
    press(&mut app, KeyCode::Enter).await?;

    // @step Then a session is created with the orchestrator bot role
    assert_eq!(
        mock.created_session_roles(),
        [Some(ORCHESTRATOR_ROLE.to_string())]
    );

    // @step And the Agent view opens on the new bot session without binding it to a work unit
    assert_eq!(app.active_view(), ViewMode::Agent);
    let sid = SessionId::new("s-bot");
    assert_eq!(app.agent_view_store().current_session(), Some(&sid));
    assert!(app.agent_view_store().work_unit_context_for(&sid).is_none());
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_new_bot_leaves_the_board_untouched() -> TestResult {
    // @step Given the New Bot dialog is open over the board
    let (mut app, mock) = board_app().await?;
    press(&mut app, KeyCode::Char('b')).await?;

    // @step When the user cancels
    press(&mut app, KeyCode::Esc).await?;

    // @step Then no session is created
    assert_eq!(mock.create_session_calls(), 0);

    // @step And the board view is still showing
    assert_eq!(app.active_view(), ViewMode::Board);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_agent_bar_offers_new_bot() -> TestResult {
    // @step Given the Agent view is showing a session
    let (mut app, mock) = board_app().await?;
    create_bot(&mut app, &mock, "s-bot").await?;
    assert_eq!(app.active_view(), ViewMode::Agent);

    // @step When the agent bar is painted
    let buf = render(&mut app)?;

    // @step Then Zone C shows "[ New Bot ]" beside "[ New Agent ]"
    let bar = bar_row(&buf)?;
    assert!(bar.contains("[ New Bot ]"), "{bar}");
    Ok(())
}

#[test]
fn the_orchestrator_role_tells_the_bot_to_delegate_through_agent_manager() -> TestResult {
    // @step Given the orchestrator bot role from codelet-sessions
    let role = ORCHESTRATOR_ROLE;

    // @step When its text is inspected
    let lower = role.to_lowercase();

    // @step Then it starts with the bot marker line
    assert!(role.starts_with(BOT_ROLE_MARKER));
    assert!(is_bot_role(role));
    assert!(!is_bot_role("You are a reviewer."));

    // @step And it instructs the session to spawn, message and await sub-agents with the AgentManager tool
    for needle in ["AgentManager", "spawn", "message", "await_idle"] {
        assert!(role.contains(needle), "role must mention {needle}");
    }

    // @step And it instructs the session to report its sub-agents' results rather than do the work itself
    assert!(lower.contains("report"));
    assert!(lower.contains("do not do the work yourself"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn bot_sessions_are_marked_as_bots_in_the_agent_bar() -> TestResult {
    // @step Given an open bot session and an open agent session
    let (mut app, mock) = board_app().await?;
    create_bot(&mut app, &mock, "s-bot").await?;
    app.dispatch(Action::BackToBoard);
    drain(&mut app).await?;
    mock.script_create_session(SessionId::new("s-agent"));
    press(&mut app, KeyCode::Char('.')).await?;
    press(&mut app, KeyCode::Enter).await?;

    // @step When the agent bar is painted
    let bar = bar_row(&render(&mut app)?)?;

    // @step Then the bot session's chip carries the "bot:" marker
    assert!(bar.contains("bot:#1"), "{bar}");

    // @step And the agent session's chip does not
    assert!(!bar.contains("bot:#2"), "{bar}");
    assert!(bar.contains("#2"), "{bar}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn on_a_narrow_board_new_bot_gives_way_before_new_agent() -> TestResult {
    // @step Given the board view is showing in an 80-column terminal
    let (mut app, _mock) = board_app().await?;

    // @step When the board menu bar is painted
    let buf = render_at(&mut app, 80)?;

    // @step Then Zone C still shows "[ New Agent ]"
    let bar = bar_row(&buf)?;

    // @step And "[ New Bot ]" is not painted
    assert!(!bar.contains("[ New Bot ]"), "{bar}");
    Ok(())
}
