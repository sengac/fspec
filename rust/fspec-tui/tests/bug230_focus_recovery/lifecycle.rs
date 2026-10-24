use super::*;

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn a_closed_cleared_session_rejects_delayed_focus_when_reopened() -> TestResult {
    // @step Given a session whose card focus was explicitly cleared
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("cleared");
    let mut snapshot = info(&backend, session.clone()).await?;
    snapshot.work_unit_focus.revision = 5;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionObserved(snapshot));
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    // @step When an older card update or duplicate cleared update arrives
    app.dispatch(Action::WorkUnitFocusChanged(
        session.clone(),
        WorkUnitFocus {
            revision: 4,
            context: Some(WorkUnitContext {
                id: "AUTH-002".into(),
                title: "Old".into(),
                status: "implementing".into(),
            }),
            source: "delayed".into(),
        },
    ));
    app.dispatch(Action::SessionCreated(session.clone()));
    // @step Then the session remains open without an active card
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&session)
        .is_none());
    Ok(())
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn dialog_creation_observed_before_acknowledgement_preserves_card_intent() -> TestResult {
    // @step Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("dialog-session");
    backend.script_create_session(session.clone());
    let mut app = App::new(backend.clone());
    app.dispatch(Action::WorkUnitsLoaded(units()));
    app.agent_view_store_mut()
        .set_current_work_unit(Some("AUTH-002".into()), Some("implementing".into()));
    // @step When the user enters card "AUTH-002"
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    app.dispatch(Action::SessionObserved(
        info(&backend, session.clone()).await?,
    ));
    settle(&mut app).await;
    // @step Then the backend and all card displays identify "AUTH-002"
    assert_eq!(app.board_store().session_for("AUTH-002"), Some(&session));
    Ok(())
}
