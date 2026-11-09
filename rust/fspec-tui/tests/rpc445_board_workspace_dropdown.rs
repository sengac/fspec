//! RPC-445 — board header workspace dropdown.
//!
//! Feature: spec/features/board-workspace-dropdown.feature
//!
//! Harness: App + MockBackend (`bug196_menu_dropdown_row_click.rs`
//! pattern) at 120x24. Header row 0 (`Checkpoints:`) is y1, so the
//! workspace row 1 is y2 and the dropdown's entries start at y4 (top
//! border y3).

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::views::board::menu_snapshot::active_menu_session_ids;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitInfo, WorkspaceEntry};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::workspace_mock::session_in;
use common::MockBackend;

const ALPHA: &str = "/projects/alpha";
const BETA: &str = "/projects/beta";

fn entries() -> Vec<WorkspaceEntry> {
    vec![
        WorkspaceEntry {
            path: ALPHA.into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: BETA.into(),
            name: "beta".into(),
            start: false,
        },
    ]
}

fn unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// An App over a MockBackend that lists alpha (start, ALPHA-001) and
/// beta (BETA-001), bootstrapped so the workspace list has loaded.
async fn booted_app() -> Result<(App, Arc<MockBackend>), codelet_test_helpers::TestError> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![unit("ALPHA-001")]);
    mock.seed_workspaces(entries());
    mock.seed_workspace_work_units(BETA, vec![unit("BETA-001")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("bootstrap")?;
    drain_pending(&mut app).await?;
    Ok((app, mock))
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

/// Let subscriber tasks forward pushes, then dispatch them.
async fn settle(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        drain_pending(app).await?;
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn click(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn render_app(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
}

/// Header row 1 = the row after the `Checkpoints:` status row.
fn workspace_row(buf: &Buffer) -> Result<u16, codelet_test_helpers::TestError> {
    Ok((0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .or_fail("checkpoint status row")?
        + 1)
}

fn board_ids(app: &App) -> Vec<String> {
    app.board_store()
        .work_units()
        .iter()
        .map(|u| u.id.clone())
        .collect()
}

async fn choose_beta_with_keys(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&key(KeyCode::Char('w')));
    drain_pending(app).await?;
    app.handle_event(&key(KeyCode::Down));
    drain_pending(app).await?;
    app.handle_event(&key(KeyCode::Enter));
    drain_pending(app).await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_header_shows_the_start_workspace_by_default() -> TestResult {
    // @step Given the daemon lists workspaces "alpha" (start) and "beta"
    let (mut app, _mock) = booted_app().await?;

    // @step When the board renders
    let buf = render_app(&mut app)?;

    // @step Then header row 1 shows "Workspace: alpha"
    let row = row_text(&buf, workspace_row(&buf)?);
    assert!(row.contains("Workspace: alpha"), "row 1 was {row:?}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pressing_w_opens_a_dropdown_of_every_workspace() -> TestResult {
    // @step Given the daemon lists workspaces "alpha" (start) and "beta"
    let (mut app, _mock) = booted_app().await?;

    // @step When the user presses "w" on the board
    app.handle_event(&key(KeyCode::Char('w')));
    drain_pending(&mut app).await?;

    // @step Then a dropdown under the header lists "alpha" and "beta"
    let buf = render_app(&mut app)?;
    let first = workspace_row(&buf)? + 2;
    assert!(row_text(&buf, first).contains("alpha (start)"));
    assert!(row_text(&buf, first + 1).contains("beta"));
    assert_eq!(app.board_store().workspace_dropdown(), Some(0));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn choosing_a_workspace_rescopes_the_board() -> TestResult {
    // @step Given the board is showing alpha's work unit "ALPHA-001" and beta has work unit "BETA-001"
    let (mut app, mock) = booted_app().await?;
    assert_eq!(board_ids(&app), ["ALPHA-001"]);

    // @step When the user opens the workspace dropdown, moves down and presses Enter
    choose_beta_with_keys(&mut app).await?;

    // @step Then the board shows "BETA-001" and header row 1 shows "Workspace: beta"
    assert_eq!(board_ids(&app), ["BETA-001"]);
    let buf = render_app(&mut app)?;
    assert!(row_text(&buf, workspace_row(&buf)?).contains("Workspace: beta"));

    // @step And a later work-unit push for alpha does not replace beta's board
    mock.push_work_units(vec![unit("ALPHA-002")]);
    settle(&mut app).await?;
    assert_eq!(board_ids(&app), ["BETA-001"]);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pushes_for_the_selected_workspace_refresh_the_board() -> TestResult {
    // @step Given the user has selected workspace "beta"
    let (mut app, mock) = booted_app().await?;
    choose_beta_with_keys(&mut app).await?;

    // @step When beta's work units are pushed with "BETA-002"
    mock.push_workspace_work_units("/projects/other", vec![unit("OTHER-001")]);
    mock.push_workspace_work_units(BETA, vec![unit("BETA-002")]);
    settle(&mut app).await?;

    // @step Then the board shows "BETA-002"
    assert_eq!(board_ids(&app), ["BETA-002"]);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn clicking_the_workspace_row_and_an_entry_selects_that_workspace() -> TestResult {
    // @step Given the daemon lists workspaces "alpha" (start) and "beta"
    let (mut app, _mock) = booted_app().await?;
    let buf = render_app(&mut app)?;
    let row = workspace_row(&buf)?;
    let label_x = row_text(&buf, row)
        .find("Workspace:")
        .and_then(|byte| row_text(&buf, row)[..byte].chars().count().try_into().ok())
        .or_fail("label x")?;

    // @step When the user clicks header row 1 and then clicks the "beta" entry
    app.handle_event(&click(label_x, row));
    drain_pending(&mut app).await?;
    render_app(&mut app)?;
    app.handle_event(&click(label_x + 1, row + 3));
    drain_pending(&mut app).await?;

    // @step Then header row 1 shows "Workspace: beta"
    let buf = render_app(&mut app)?;
    assert!(row_text(&buf, row).contains("Workspace: beta"));
    assert_eq!(app.board_store().workspace_dropdown(), None);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn only_the_selected_workspaces_sessions_are_shown() -> TestResult {
    // @step Given an open session in alpha and an open session in beta
    let (mut app, _mock) = booted_app().await?;
    app.dispatch(Action::SessionObserved(session_in("s-alpha", ALPHA)));
    app.dispatch(Action::SessionObserved(session_in("s-beta", BETA)));
    drain_pending(&mut app).await?;
    assert_eq!(
        active_menu_session_ids(app.agent_view_store()),
        [SessionId::new("s-alpha")]
    );

    // @step When the user selects workspace "beta"
    choose_beta_with_keys(&mut app).await?;

    // @step Then the menu bar shows only beta's session
    assert_eq!(
        active_menu_session_ids(app.agent_view_store()),
        [SessionId::new("s-beta")]
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn new_agents_start_in_the_selected_workspace() -> TestResult {
    // @step Given the user has selected workspace "beta"
    let (mut app, mock) = booted_app().await?;
    choose_beta_with_keys(&mut app).await?;

    // @step When the user creates a new agent from the board
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await?;

    // @step Then the session is created in beta's path
    assert_eq!(mock.session_created_in().as_deref(), Some(BETA));
    Ok(())
}
