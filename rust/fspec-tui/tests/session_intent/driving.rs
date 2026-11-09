//! App-driving helpers for the session-intent scenarios (board entry,
//! mux toggling, the create-session dialog). Split from fixture.rs so a
//! binary that only needs the fixture core does not compile unused helpers.

use codelet_fspec_tui::{MuxConfig, CREATE_SESSION_DIALOG_ID};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};

use super::*;

pub fn app(backend: Arc<common::MockBackend>) -> App {
    let mut app = App::new(backend);
    app.dispatch(Action::WorkUnitsLoaded(
        ["AUTH-001", "AUTH-002"]
            .map(|value| codelet_rpc_types::WorkUnitInfo {
                id: value.into(),
                title: value.into(),
                status: "implementing".into(),
                work_type: "story".into(),
                description: None,
                estimate: None,
                epic: None,
                attachments: Vec::new(),
                last_state_change_at: None,
            })
            .to_vec(),
    ));
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-001".into()), Some("implementing".into()));
    app
}

pub fn text(app: &mut App) -> String {
    let area = Rect::new(0, 0, 120, 35);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub fn mux(app: &mut App) {
    let sessions = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|context| context.id.clone())
        .collect::<Vec<_>>();
    app.navigator_mut().mux.enable_with_config(
        MuxConfig {
            enabled: true,
            ..Default::default()
        },
        ViewMode::Board,
    );
    app.navigator_mut().mux.sync_window(&sessions);
    app.navigator_mut().active_view = ViewMode::Mux;
}

pub fn enter(app: &mut App, in_mux: bool) {
    if in_mux {
        mux(app);
        app.dispatch(Action::MuxEnterWorkUnit("AUTH-002".into()));
    } else {
        app.navigator_mut().active_view = ViewMode::Board;
        app.dispatch(Action::EnterWorkUnit("AUTH-002".into()));
    }
}

pub async fn confirm(app: &mut App, isolated: bool) -> codelet_test_helpers::TestResult {
    if isolated {
        let _ = app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Right,
            KeyModifiers::NONE,
        )));
    }
    let _ = app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    settle(app).await?;
    Ok(())
}

pub async fn cancel(app: &mut App) -> codelet_test_helpers::TestResult {
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    settle(app).await?;
    Ok(())
}

pub fn dialog(app: &App) -> bool {
    app.compositor().contains(CREATE_SESSION_DIALOG_ID)
}

pub fn open_generic(app: &mut App) {
    app.dispatch(Action::OpenCreateSessionDialog { preselect: None });
}
