//! Feature: spec/features/session-creation-intent.feature
use super::*;

#[tokio::test]
#[serial_test::serial]
async fn board_entry_navigates_to_the_earliest_associated_open_agent() -> TestResult {
    let _isolation = Isolation::new()?;
    for in_mux in [false, true] {
        // @step Given unrelated and multiple associated agents are open
        let backend = Arc::new(common::MockBackend::new());
        let mut app = app(backend.clone());
        add(&mut app, &backend, "first", Some("AUTH-002")).await?;
        add(&mut app, &backend, "second", Some("AUTH-002")).await?;
        add(&mut app, &backend, "unrelated", Some("AUTH-001")).await?;
        let assignments = backend.set_work_unit_context_calls();
        // @step When the user enters the associated story from the board or mux
        enter(&mut app, in_mux);
        settle(&mut app).await;
        // @step Then the earliest associated agent is focused without a creation dialog
        assert_eq!(
            app.agent_view_store().current_session(),
            Some(&sid("first"))
        );
        assert!(!dialog(&app));
        if in_mux {
            assert_eq!(app.navigator().mux.focused_session_id(), Some(sid("first")));
        }
        // @step And no agent assignment changes
        assert_eq!(backend.set_work_unit_context_calls(), assignments);
        assert_eq!(focus(&app, "unrelated").as_deref(), Some("AUTH-001"));
        assert_eq!(focus(&app, "second").as_deref(), Some("AUTH-002"));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unassociated_board_entry_offers_story_specific_creation() -> TestResult {
    let _isolation = Isolation::new()?;
    for in_mux in [false, true] {
        for has_agent in [false, true] {
            // @step Given a story has no associated open agent with or without unrelated agents
            let backend = Arc::new(common::MockBackend::new());
            let mut app = app(backend.clone());
            if has_agent {
                add(&mut app, &backend, "unrelated", Some("AUTH-001")).await?;
            }
            let assignments = backend.set_work_unit_context_calls();
            // @step When the user enters that story from the board or mux
            enter(&mut app, in_mux);
            settle(&mut app).await;
            // @step Then a story-specific dialog offers normal isolated and cancel choices
            assert!(dialog(&app));
            let rendered = text(&mut app);
            for label in ["Work on AUTH-002?", "Yes", "Yes - Isolated", "Cancel"] {
                assert!(rendered.contains(label), "missing {label}");
            }
            // @step And no session is created or reassigned before confirmation
            assert_eq!(backend.create_session_calls(), 0);
            assert_eq!(backend.create_isolated_session_calls(), 0);
            assert_eq!(backend.set_work_unit_context_calls(), assignments);
            assert_eq!(
                app.navigator().active_view,
                if in_mux {
                    ViewMode::Mux
                } else {
                    ViewMode::Board
                }
            );
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn board_entry_preserves_an_explicitly_resumed_agent() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given an explicitly resumed agent with its own revisioned story focus
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app(backend.clone());
    backend
        .set_work_unit_context(sid("resumed"), Some(card("AUTH-001")))
        .await?;
    app.dispatch(Action::AttachToSession(sid("resumed")));
    settle(&mut app).await;
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("resumed"),
        WorkUnitFocus {
            revision: 8,
            context: Some(card("AUTH-001")),
            source: "restored".into(),
        },
    ));
    let assignments = backend.set_work_unit_context_calls();
    // @step When the user enters an unrelated story and delayed older focus arrives
    enter(&mut app, false);
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("resumed"),
        WorkUnitFocus {
            revision: 7,
            context: Some(card("AUTH-002")),
            source: "delayed".into(),
        },
    ));
    settle(&mut app).await;
    // @step Then story creation is offered without changing the resumed agent focus
    assert!(dialog(&app));
    assert_eq!(focus(&app, "resumed").as_deref(), Some("AUTH-001"));
    assert_eq!(
        app.agent_view_store()
            .work_unit_focus_revision_for(&sid("resumed")),
        Some(8)
    );
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    Ok(())
}
