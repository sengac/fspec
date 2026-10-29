use codelet_fspec_tui::views::agent::RenderedChunk;
use codelet_fspec_tui::views::multiplex::{MuxConfig, MuxPaneKind};
use codelet_fspec_tui::{Action, App, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext};
use crossterm::event::KeyCode;
use ratatui::text::Line;

use super::fixture::*;

pub async fn open_board_providers(app: &mut App, menu: bool) {
    if menu {
        app.dispatch(Action::MenuExecuteItem {
            category: 2,
            row: 1,
        });
        drain(app).await;
    } else {
        key(app, KeyCode::Char('P')).await;
    }
    assert_eq!(app.active_view(), ViewMode::ProviderSettings);
}

fn assignment() -> WorkUnitContext {
    WorkUnitContext {
        id: "CARD-1".into(),
        title: "CARD-1".into(),
        status: "implementing".into(),
    }
}

pub fn seed_agent(app: &mut App) -> TestResult {
    app.agent_view_store_mut()
        .set_work_unit_context(sid(), assignment());
    app.navigator_mut()
        .agent
        .input
        .set_value("unfinished draft");
    app.agent_view_store_mut()
        .set_input_draft(0, "unfinished draft".into());
    let session = app
        .agent_view_store_mut()
        .current_session_context_mut()
        .ok_or("session missing")?;
    session.scrollback.push(RenderedChunk {
        seq: 1,
        lines: (0..80)
            .map(|row| Line::raw(format!("transcript {row}")))
            .collect(),
        source: None,
    });
    screen(app);
    app.agent_view_store_mut()
        .current_session_context_mut()
        .ok_or("session missing")?
        .scrollback
        .jump_to_offset(5);
    Ok(())
}

pub fn assert_agent(app: &App) -> TestResult {
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid()));
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid()),
        Some(&assignment())
    );
    assert_eq!(app.navigator().agent.input.value(), "unfinished draft");
    let session = app
        .agent_view_store()
        .current_session_context()
        .ok_or("session missing")?;
    assert_eq!(session.input_draft, "unfinished draft");
    assert_eq!(session.scrollback.scroll_state().offset, 5);
    assert!(!session.scrollback.scroll_state().stick_to_bottom);
    assert_eq!(session.scrollback.chunk_count(), 1);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub struct MuxSnapshot {
    config: MuxConfig,
    panes: Vec<MuxPaneKind>,
    focus: usize,
    window: Vec<SessionId>,
    window_start: usize,
}

impl MuxSnapshot {
    pub fn capture(app: &App) -> Self {
        let mux = &app.navigator().mux;
        Self {
            config: mux.config().clone(),
            panes: mux.effective_panes().to_vec(),
            focus: mux.focus(),
            window: mux.window_session_ids(),
            window_start: mux.window_start(),
        }
    }
}

pub async fn switch_siblings(app: &mut App, end_in_models: bool) {
    for _round in 0..3 {
        key(app, KeyCode::Tab).await;
        assert_eq!(app.active_view(), ViewMode::ModelSelector);
        key(app, KeyCode::Tab).await;
        assert_eq!(app.active_view(), ViewMode::ProviderSettings);
    }
    if end_in_models {
        key(app, KeyCode::Tab).await;
    }
}

pub async fn assert_explicit_navigation() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.caller(ViewMode::Agent).await;
    open(
        &mut fixture.app,
        Action::OpenProviderSettingsView,
        ViewMode::ProviderSettings,
    )
    .await;
    fixture.app.dispatch(Action::BackToBoard);
    open(
        &mut fixture.app,
        Action::OpenModelSelectorView,
        ViewMode::ModelSelector,
    )
    .await;
    key(&mut fixture.app, KeyCode::Esc).await;
    assert_eq!(fixture.app.active_view(), ViewMode::Board);
    open(
        &mut fixture.app,
        Action::OpenProviderSettingsView,
        ViewMode::ProviderSettings,
    )
    .await;
    fixture.app.dispatch(Action::OpenAgentView(Some(sid())));
    open(
        &mut fixture.app,
        Action::OpenBlocklistView,
        ViewMode::Blocklist,
    )
    .await;
    key(&mut fixture.app, KeyCode::Esc).await;
    assert_eq!(fixture.app.active_view(), ViewMode::Agent);
    Ok(())
}
