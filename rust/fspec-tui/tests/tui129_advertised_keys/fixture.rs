//! TUI-129 test support: an `App` over `MockBackend`, key presses that run
//! every resulting task and action until nothing moves, and screen reads.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, MuxConfig, MuxLayout, MuxPaneKind, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitInfo, WorkspaceEntry};
use codelet_test_helpers::{OrFail, TestError, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::common::{MockBackend, TaskOutcome};

/// The notice every `Action::SessionOperationFailed` opens.
pub const NOTICE_DIALOG_ID: &str = "session-storage-dialog";

pub fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

pub fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    (App::new(backend), mock)
}

/// A booted App whose board shows workspace `path` (the start workspace).
pub async fn booted_app(path: &str) -> Result<(App, Arc<MockBackend>), TestError> {
    let mock = Arc::new(MockBackend::new());
    mock.seed_workspaces(vec![WorkspaceEntry {
        path: path.into(),
        name: "ws".into(),
        start: true,
    }]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.or_fail("App::bootstrap")?;
    drain(&mut app).await?;
    Ok((app, mock))
}

pub fn unit(id: &str, attachments: &[&str]) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "implementing".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: attachments.iter().map(|a| (*a).to_string()).collect(),
        last_state_change_at: None,
    }
}

/// Put `unit` on the board and select it.
pub async fn select_card(app: &mut App, card: WorkUnitInfo) -> TestResult {
    app.dispatch(Action::WorkUnitsLoaded(vec![card]));
    drain(app).await?;
    app.board_store_mut().set_focused_column("implementing");
    app.board_store_mut()
        .set_selected_index_for("implementing", 0);
    Ok(())
}

/// Run every pending task and every queued action until nothing moves.
pub async fn drain(app: &mut App) -> TestResult {
    for _ in 0..100 {
        let mut progressed = false;
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
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
    Err(TestError::from("the App never settled"))
}

pub async fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await
}

pub async fn press(app: &mut App, code: KeyCode) -> TestResult {
    press_with(app, code, KeyModifiers::NONE).await
}

/// A character as a terminal sends it: upper case letters carry Shift.
pub async fn type_char(app: &mut App, ch: char) -> TestResult {
    let modifiers = if ch.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    press_with(app, KeyCode::Char(ch), modifiers).await
}

/// The rows `App::render` paints at `width` x `height`.
pub fn screen(app: &mut App, width: u16, height: u16) -> Result<Vec<String>, TestError> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    terminal
        .draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("Terminal::draw")?;
    let buf = terminal.backend().buffer();
    Ok((0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect())
}

pub fn shows(rows: &[String], needle: &str) -> bool {
    rows.iter().any(|row| row.contains(needle))
}

/// The mux with `panes`, the first pane focused (the menu004 entry).
pub fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    // MUX-010: one equal column per pane (every caller passes panes).
    let Some(layout) = MuxLayout::columns(&panes) else {
        return;
    };
    let config = MuxConfig {
        layout,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
}

/// Open sessions s-1..s-n.
pub async fn open_sessions(app: &mut App, n: usize) -> TestResult {
    for i in 1..=n {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain(app).await
}
