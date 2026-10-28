//! Feature: spec/features/generic-agent-creation.feature
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::*;

#[tokio::test]
#[serial_test::serial]
async fn new_agent_starts_unassigned_despite_stale_selection() -> TestResult {
    let _isolation = Isolation::new()?;
    for story in [None, Some("AUTH-002")] {
        for isolated in [false, true] {
            // @step Given stale board selection and a current agent with different or cleared focus
            let backend = Arc::new(common::MockBackend::new());
            let mut app = app(backend.clone());
            add(&mut app, &backend, "original", story).await?;
            let assignments = backend.set_work_unit_context_calls();
            script(&backend, isolated, "blank");
            // @step When New Agent is confirmed with normal or isolated creation
            app.dispatch(Action::MenuZoneCActivate(0));
            settle(&mut app).await;
            let rendered = text(&mut app);
            confirm(&mut app, isolated).await;
            // @step Then the dialog is generic and the new session has no story
            assert!(rendered.contains("Start New Agent?"));
            assert!(app
                .agent_view_store()
                .session_context_for(&sid("blank"))
                .is_some());
            assert_eq!(focus(&app, "blank"), None);
            assert_eq!(backend.set_work_unit_context_calls(), assignments);
            // @step And existing authoritative focus is unchanged
            assert_eq!(focus(&app, "original").as_deref(), story);
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn shift_right_navigates_before_offering_unassigned_creation() -> TestResult {
    let _isolation = Isolation::new()?;
    for in_mux in [false, true] {
        for isolated in [false, true] {
            // @step Given multiple open agents and stale board selection
            let backend = Arc::new(common::MockBackend::new());
            let mut app = app(backend.clone());
            add(&mut app, &backend, "first", Some("AUTH-001")).await?;
            add(&mut app, &backend, "second", Some("AUTH-002")).await?;
            app.agent_view_store_mut().focus_session_index(0);
            if in_mux {
                mux(&mut app);
                app.navigator_mut()
                    .mux
                    .focus_agent_pane_for_session(&sid("first"));
            } else {
                app.navigator_mut().active_view = ViewMode::Agent;
            }
            let assignments = backend.set_work_unit_context_calls();
            script(&backend, isolated, "blank");
            // @step When Shift Right navigates within agents then creates at the right edge in agent or mux mode
            let key = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
            let _ = app.handle_event(&key);
            settle(&mut app).await;
            assert_eq!(
                app.agent_view_store().current_session(),
                Some(&sid("second"))
            );
            assert!(!dialog(&app));
            assert_eq!(backend.create_session_calls(), 0);
            let _ = app.handle_event(&key);
            settle(&mut app).await;
            assert!(dialog(&app));
            assert!(text(&mut app).contains("Start New Agent?"));
            cancel(&mut app).await;
            assert!(!dialog(&app));
            assert!(!app.navigator().mux.pending_new_agent());
            let _ = app.handle_event(&key);
            settle(&mut app).await;
            assert!(dialog(&app));
            confirm(&mut app, isolated).await;
            // @step Then navigation creates nothing and the confirmed new agent is unassigned
            assert_eq!(backend.create_session_calls(), usize::from(!isolated));
            assert_eq!(
                backend.create_isolated_session_calls(),
                usize::from(isolated)
            );
            assert!(app
                .agent_view_store()
                .session_context_for(&sid("blank"))
                .is_some());
            assert_eq!(focus(&app, "blank"), None);
            assert_eq!(backend.set_work_unit_context_calls(), assignments);
            // @step And existing authoritative focus is unchanged
            assert_eq!(focus(&app, "first").as_deref(), Some("AUTH-001"));
            assert_eq!(focus(&app, "second").as_deref(), Some("AUTH-002"));
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn generic_session_observation_and_completion_never_attach_ambient_selection() -> TestResult {
    let _isolation = Isolation::new()?;
    for observation_first in [false, true] {
        // @step Given stale board selection and an unrelated focused agent
        let backend = Arc::new(common::MockBackend::new());
        let mut app = app(backend.clone());
        add(&mut app, &backend, "original", Some("AUTH-001")).await?;
        let assignments = backend.set_work_unit_context_calls();
        // @step When a generic session is observed and creation completion is delivered repeatedly
        let observed = snapshot(&backend, "generic").await?;
        if observation_first {
            app.dispatch(Action::SessionObserved(observed.clone()));
        }
        app.dispatch(Action::SessionCreated(sid("generic")));
        app.agent_view_store_mut().focus_session_index(0);
        app.dispatch(Action::SessionObserved(observed));
        app.dispatch(Action::SessionCreated(sid("generic")));
        settle(&mut app).await;
        // @step Then the session is initialized once without any story assignment
        assert_eq!(app.agent_view_store().open_sessions().len(), 2);
        assert_eq!(
            app.agent_view_store().current_session(),
            Some(&sid("original"))
        );
        assert_eq!(focus(&app, "generic"), None);
        assert_eq!(backend.set_work_unit_context_calls(), assignments);
        // @step And no unrelated focus is cleared or reassigned
        assert_eq!(focus(&app, "original").as_deref(), Some("AUTH-001"));
    }
    Ok(())
}
