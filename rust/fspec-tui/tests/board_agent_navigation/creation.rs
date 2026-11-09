//! Feature: spec/features/rpc012-board-agent-navigation.feature
use super::*;
use crate::common::TaskOutcome;
use codelet_test_helpers::TestResult;

#[tokio::test]
async fn open_agent_view_with_none_mounts_create_session_dialog() -> TestResult {
    // @step Given an App with bootstrap complete and BoardStore seeded with [AUTH-001 backlog]
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut app = App::new(mock.clone());
    app.bootstrap().await?;
    // @step And BoardStore has no session_attachments entry for "AUTH-001"
    assert!(app.board_store().session_for("AUTH-001").is_none());
    // @step When the App dispatches Action::OpenAgentView(None)
    app.dispatch(Action::OpenAgentView(None));
    // @step Then the creation dialog is visible
    assert!(app
        .compositor()
        .contains(codelet_fspec_tui::CREATE_SESSION_DIALOG_ID));
    // @step And AgentViewStore.show_create_session_dialog returns true
    assert!(app.agent_view_store().show_create_session_dialog());
    // @step And AgentViewStore.should_auto_create_session returns false
    assert!(!app.agent_view_store().should_auto_create_session());
    // @step And the Navigator's active_view equals ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);
    Ok(())
}

async fn settle(app: &mut App) -> TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        match app.try_recv_action() {
            Some(action) => app.dispatch(action),
            None => break,
        }
    }
    Ok(())
}

#[tokio::test]
async fn enter_on_an_unassociated_work_unit_offers_explicit_creation() -> TestResult {
    // @step Given an App with bootstrap complete and BoardStore seeded with [AUTH-002 implementing]
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-002", "implementing")]);
    mock.script_create_session(SessionId::new("s-explicit"));
    let mut app = App::new(mock.clone());
    app.bootstrap().await?;
    // @step And BoardStore.focused_column() returns "implementing" with selection 0
    app.board_store_mut().set_focused_column("implementing");
    app.board_store_mut()
        .set_selected_index_for("implementing", 0);
    // @step And AgentViewStore.current_session is None
    assert!(app.agent_view_store().current_session().is_none());
    // @step When the App dispatches Action::EnterWorkUnit("AUTH-002")
    app.dispatch(Action::EnterWorkUnit("AUTH-002".into()));
    // @step Then the creation dialog offers work on AUTH-002 while the board remains active
    assert!(app
        .compositor()
        .contains(codelet_fspec_tui::CREATE_SESSION_DIALOG_ID));
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And no session is created before confirmation
    assert_eq!(mock.create_session_calls(), 0);
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    settle(&mut app).await?;
    assert_eq!(mock.create_session_calls(), 1);
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&SessionId::new("s-explicit"))
            .map(|context| context.id.as_str()),
        Some("AUTH-002")
    );
    Ok(())
}

#[tokio::test]
async fn session_created_ignores_ambient_work_unit_selection() -> TestResult {
    // @step Given an App with AgentViewStore.current_work_unit_id = Some("AUTH-002")
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-002", "implementing")]);
    let mut app = App::new(mock.clone());
    app.bootstrap().await?;
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-002".into()), Some("implementing".into()));
    // @step When the App dispatches Action::SessionCreated(SessionId::new("s-1"))
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    settle(&mut app).await?;
    // @step Then session s-1 has no work unit and no board attachment
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&SessionId::new("s-1"))
        .is_none());
    assert!(app.board_store().session_for("AUTH-002").is_none());
    // @step And the backend receives no implicit assignment
    assert_eq!(mock.set_work_unit_context_calls(), 0);
    Ok(())
}

/// The rendered screen, one line per row.
fn screen(app: &mut App) -> String {
    let area = ratatui::layout::Rect::new(0, 0, 200, 50);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn entering_a_work_unit_missing_from_the_board_is_reported() -> TestResult {
    // @step Given an App with bootstrap complete and an empty board
    let mock = Arc::new(MockBackend::new());
    let mut app = App::new(mock.clone());
    app.bootstrap().await?;
    settle(&mut app).await?;
    // @step When the App dispatches Action::EnterWorkUnit("GONE-001")
    app.dispatch(Action::EnterWorkUnit("GONE-001".to_string()));
    settle(&mut app).await?;
    // @step Then the failure dialog says work unit GONE-001 is no longer on the board
    let shown = screen(&mut app);
    if !shown.contains("Work unit GONE-001 is no longer on the board") {
        return Err(format!("the missing work unit is not reported:\n{shown}").into());
    }
    // @step And no creation dialog is shown
    if app
        .compositor()
        .contains(codelet_fspec_tui::CREATE_SESSION_DIALOG_ID)
    {
        return Err("no creation dialog may open for a work unit not on the board".into());
    }
    Ok(())
}
