//! Feature: spec/features/rust-mux-mode.feature
use super::*;

fn seed_focus(app: &mut App) {
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-001".into()), Some("implementing".into()));
    app.dispatch(Action::WorkUnitFocusChanged(
        SessionId::new("s-1"),
        codelet_rpc_types::WorkUnitFocus {
            revision: 1,
            context: Some(codelet_rpc_types::WorkUnitContext {
                id: "AUTH-001".into(),
                title: "Login".into(),
                status: "implementing".into(),
            }),
            source: "fixture".into(),
        },
    ));
}

#[tokio::test]
async fn slash_mux_off_returns_to_the_pre_mux_view() {
    // @step Given the TUI is showing the single Agent view
    let _guard = DATA_DIR_GUARD.lock().await;
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await;
    seed_focus(&mut app);
    app.dispatch(Action::OpenAgentView(Some(SessionId::new("s-1"))));
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step When I submit the slash command "/mux on"
    submit(&mut app, "/mux on");
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And I submit the slash command "/mux off"
    submit(&mut app, "/mux off");
    // @step Then mux mode is inactive
    assert!(!app.mux_state().config().enabled);
    // @step And the TUI shows the single Agent view that was active before mux was entered
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And the board and agent store state is unchanged by the round trip
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        Some("AUTH-001")
    );
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&SessionId::new("s-1"))
            .map(|context| context.id.as_str()),
        Some("AUTH-001")
    );
}

#[tokio::test]
async fn enter_on_a_board_work_unit_in_mux_mode_focuses_the_agent_pane_app() {
    // @step Given mux mode is active with Board and Agent panes
    let (mut app, mock) = fresh_app();
    app_with_mux(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the Board pane is focused with work unit AUTH-001 selected
    app.navigator_mut().mux.set_focus(0);
    // @step And an open agent is already focused on AUTH-001
    seed_focus(&mut app);
    // @step When I press Enter
    app.dispatch(Action::MuxEnterWorkUnit("AUTH-001".into()));
    // @step Then the associated agent is focused without reassignment
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-1"))
    );
    assert_eq!(mock.set_work_unit_context_calls(), 0);
    // @step And the Agent pane is focused
    assert_eq!(app.navigator().mux.focus(), 1);
    // @step And the TUI is still in mux mode showing both panes
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And the board stays visible in its pane
    assert!(app
        .navigator()
        .mux
        .effective_panes()
        .contains(&MuxPaneKind::Board));
}

#[tokio::test]
async fn existing_single_view_behavior_is_unchanged_when_mux_is_off_app(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the TUI is showing the single Board view with mux inactive
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled);
    // @step And an open agent is already focused on the selected work unit
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    seed_focus(&mut app);
    // @step When I press Enter on a selected work unit
    app.dispatch(Action::EnterWorkUnit("AUTH-001".into()));
    drain_pending(&mut app).await;
    // @step Then the TUI flips to the single Agent view exactly as before mux existed
    assert_eq!(app.active_view(), ViewMode::Agent);
    // @step And pressing Esc in the Agent view returns to the single Board view
    app.dispatch(Action::BackToBoard);
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And no mux footer row is painted and no pane divider is rendered
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))?;
    terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut()))?;
    let buffer = terminal.backend().buffer();
    let rendered: String = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(
        !rendered.contains("MUX"),
        "unexpected mux footer: {rendered}"
    );
    Ok(())
}
