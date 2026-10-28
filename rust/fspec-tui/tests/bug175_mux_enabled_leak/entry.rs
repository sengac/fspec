//! Feature: spec/features/mux-close-session-landing.feature
use super::*;

#[tokio::test]
async fn enter_on_a_board_work_unit_enters_the_single_agent_view_when_the_persisted_flag_is_on(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the TUI is on the single Board view with work unit AUTH-001 selected
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::WorkUnitsLoaded(vec![
        codelet_rpc_types::WorkUnitInfo {
            id: "AUTH-001".into(),
            title: "User Login".into(),
            work_type: "story".into(),
            status: "backlog".into(),
            description: None,
            estimate: None,
            epic: None,
            attachments: Vec::new(),
            last_state_change_at: None,
        },
    ]));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And the persisted tui.mux enabled flag is on
    simulate_leaked_persisted_flag(&mut app);
    // @step And an open agent is already focused on AUTH-001
    app_with_sessions(&mut app, 1).await;
    mock.set_work_unit_context(
        sid("s-1"),
        Some(codelet_rpc_types::WorkUnitContext {
            id: "AUTH-001".into(),
            title: "User Login".into(),
            status: "backlog".into(),
        }),
    )
    .await?;
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("s-1"),
        mock.get_work_unit_focus(sid("s-1")).await?,
    ));
    let assignments = mock.set_work_unit_context_calls();
    // @step When I press Enter
    let _ = app.handle_event(&enter());
    while let Some(action) = app.try_recv_action() {
        assert!(!matches!(action, Action::MuxEnterWorkUnit(_)));
        app.dispatch(action);
        while let Some(task) = app.next_pending_task() {
            task.await?;
        }
    }
    // @step Then the TUI is on the single Agent view
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(mock.set_work_unit_context_calls(), assignments);
    Ok(())
}
