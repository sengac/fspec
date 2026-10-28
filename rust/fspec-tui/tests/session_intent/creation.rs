//! Feature: spec/features/session-creation-completion.feature
use super::*;

#[tokio::test]
#[serial_test::serial]
async fn story_creation_retains_its_target_across_observation_and_focus_races() -> TestResult {
    let _isolation = Isolation::new()?;
    for in_mux in [false, true] {
        for isolated in [false, true] {
            // @step Given a story-specific creation dialog and an unrelated open agent
            let backend = Arc::new(common::MockBackend::new());
            let mut app = app(backend.clone());
            add(&mut app, &backend, "unrelated", Some("AUTH-001")).await?;
            enter(&mut app, in_mux);
            let assignments = backend.set_work_unit_context_calls();
            script(&backend, isolated, "created");
            // @step When normal or isolated creation is observed before acknowledgement and focus changes
            app.compositor_mut()
                .remove(codelet_fspec_tui::CREATE_SESSION_DIALOG_ID);
            app.dispatch(Action::CreateSessionSubmitted { isolated });
            app.dispatch(Action::SessionObserved(
                snapshot(&backend, "created").await?,
            ));
            app.agent_view_store_mut().focus_session_index(0);
            app.agent_view_store_mut()
                .set_current_work_unit(Some("AUTH-001".into()), Some("implementing".into()));
            settle(&mut app).await;
            // @step Then only the returned new session is assigned the requested story once
            assert_eq!(backend.set_work_unit_context_calls(), assignments + 1);
            assert_eq!(
                backend
                    .last_set_work_unit_context()
                    .map(|(session, context)| (session, context.map(|card| card.id))),
                Some((sid("created"), Some("AUTH-002".into())))
            );
            assert_eq!(focus(&app, "created").as_deref(), Some("AUTH-002"));
            assert_eq!(
                backend.create_isolated_session_calls(),
                usize::from(isolated)
            );
            assert_eq!(backend.create_session_calls(), usize::from(!isolated));
            // @step And unrelated focus and mux layout remain unchanged
            assert_eq!(focus(&app, "unrelated").as_deref(), Some("AUTH-001"));
            assert_eq!(
                app.navigator().active_view,
                if in_mux {
                    ViewMode::Mux
                } else {
                    ViewMode::Agent
                }
            );
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cancelling_story_creation_cannot_leak_intent() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given a story-specific creation dialog over an unrelated agent
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app(backend.clone());
    add(&mut app, &backend, "unrelated", Some("AUTH-001")).await?;
    let assignments = backend.set_work_unit_context_calls();
    enter(&mut app, false);
    // @step When the user cancels and subsequently creates a generic agent
    cancel(&mut app).await;
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    assert_eq!(backend.create_session_calls(), 0);
    script(&backend, false, "blank");
    open_generic(&mut app);
    assert!(text(&mut app).contains("Start New Agent?"));
    confirm(&mut app, false).await;
    // @step Then the new agent has no story and the original agent keeps its story
    assert!(app
        .agent_view_store()
        .session_context_for(&sid("blank"))
        .is_some());
    assert_eq!(focus(&app, "blank"), None);
    assert_eq!(focus(&app, "unrelated").as_deref(), Some("AUTH-001"));
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn declined_or_failed_story_creation_cannot_leak_intent() -> TestResult {
    let _isolation = Isolation::new()?;
    for declined in [false, true] {
        // @step Given a story-specific creation dialog over an unrelated agent
        let backend = Arc::new(common::MockBackend::new());
        let mut app = app(backend.clone());
        add(&mut app, &backend, "unrelated", Some("AUTH-001")).await?;
        let assignments = backend.set_work_unit_context_calls();
        enter(&mut app, false);
        // @step When creation is declined or fails and a generic creation is subsequently confirmed
        if declined {
            backend.script_create_session(sid(""));
        } else {
            backend.seed_create_isolated_session_result(Err("synthetic creation failure".into()));
        }
        confirm(&mut app, !declined).await;
        let failure = text(&mut app);
        cancel(&mut app).await;
        script(&backend, false, "later");
        open_generic(&mut app);
        confirm(&mut app, false).await;
        // @step Then the failure is visible and the later agent has no story
        assert!(failure.contains(if declined {
            "declined"
        } else {
            "synthetic creation failure"
        }));
        assert!(app
            .agent_view_store()
            .session_context_for(&sid("later"))
            .is_some());
        assert_eq!(focus(&app, "later"), None);
        assert_eq!(backend.set_work_unit_context_calls(), assignments);
        // @step And the original agent keeps its story
        assert_eq!(focus(&app, "unrelated").as_deref(), Some("AUTH-001"));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn duplicate_story_completion_cannot_overwrite_newer_authoritative_focus() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given a completed story creation whose session later clears its focus
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app(backend.clone());
    script(&backend, false, "created");
    enter(&mut app, false);
    confirm(&mut app, false).await;
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("created"),
        WorkUnitFocus {
            revision: 50,
            context: None,
            source: "remote-clear".into(),
        },
    ));
    let assignments = backend.set_work_unit_context_calls();
    // @step When duplicate story completion and older focus acknowledgements arrive
    app.dispatch(Action::WorkUnitSessionCreated(
        sid("created"),
        "AUTH-002".into(),
    ));
    app.dispatch(Action::WorkUnitAttached(sid("created"), card("AUTH-002")));
    app.dispatch(Action::WorkUnitFocusChanged(
        sid("created"),
        WorkUnitFocus {
            revision: 2,
            context: Some(card("AUTH-002")),
            source: "delayed".into(),
        },
    ));
    settle(&mut app).await;
    // @step Then the newer cleared focus remains authoritative without another attachment
    assert_eq!(focus(&app, "created"), None);
    assert_eq!(
        app.agent_view_store()
            .work_unit_focus_revision_for(&sid("created")),
        Some(50)
    );
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    Ok(())
}
