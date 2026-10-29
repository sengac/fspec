//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::*;

#[tokio::test]
#[serial_test::serial]
async fn board_enter_opens_a_modal_chooser_and_escape_preserves_context() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given a board with two cards and an unrelated open agent
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app_with(backend.clone());
    intent_fixture::add(&mut app, &backend, "unrelated", Some("AUTH-002")).await?;
    app.navigator_mut().active_view = ViewMode::Board;
    let assignments = backend.set_work_unit_context_calls();
    assert_eq!(app.current_session(), Some(SessionId::new("unrelated")));
    assert_eq!(
        app.board_store()
            .selected_work_unit()
            .map(|card| card.id.as_str()),
        Some("AUTH-001")
    );
    // @step When the user presses Enter then navigation shortcut and paste keys
    chooser(&mut app);
    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('?'),
        KeyCode::Char('m'),
        KeyCode::Char('a'),
        KeyCode::Right,
    ] {
        assert!(press(&mut app, code));
    }
    assert!(app.handle_paste("do not leak\ninto an agent").is_consumed());
    flush(&mut app);
    // @step Then the chooser offers Open in agent and Show full details without changing context
    assert_chooser(&mut app);
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.current_session(), Some(SessionId::new("unrelated")));
    assert_eq!(
        app.board_store()
            .selected_work_unit()
            .map(|card| card.id.as_str()),
        Some("AUTH-001")
    );
    assert!(app.navigator().agent.input.value().is_empty());
    // @step And Escape closes only the chooser without creating or reassigning an agent
    press(&mut app, KeyCode::Esc);
    assert!(app.compositor().is_empty());
    assert_eq!(backend.create_session_calls(), 0);
    assert_eq!(backend.create_isolated_session_calls(), 0);
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    assert_eq!(backend.create_session_calls(), 0);
    assert_eq!(backend.create_isolated_session_calls(), 0);
    assert!(!app.should_quit());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn default_agent_choice_uses_the_captured_card_and_earliest_associated_agent() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given multiple associated agents and an unrelated focused agent
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app_with(backend.clone());
    for (session, card) in [
        ("first", "AUTH-001"),
        ("second", "AUTH-001"),
        ("unrelated", "AUTH-002"),
    ] {
        intent_fixture::add(&mut app, &backend, session, Some(card)).await?;
    }
    app.navigator_mut().active_view = ViewMode::Board;
    let assignments = backend.set_work_unit_context_calls();
    // @step When Enter opens the chooser and a board refresh selects a different card
    chooser(&mut app);
    app.dispatch(Action::WorkUnitsLoaded(vec![
        summary("AUTH-002"),
        summary("AUTH-001"),
    ]));
    app.board_store_mut().select_work_unit("AUTH-002", 20);
    // @step Then confirming the default choice opens the earliest agent for the captured card
    press(&mut app, KeyCode::Enter);
    settle(&mut app).await?;
    assert_eq!(app.current_session(), Some(SessionId::new("first")));
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert!(app.compositor().is_empty());
    // @step And no agent assignment or creation occurs
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    assert_eq!(backend.create_session_calls(), 0);
    assert_eq!(backend.create_isolated_session_calls(), 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unassociated_agent_choice_retains_normal_isolated_and_cancel_behavior() -> TestResult {
    let _isolation = Isolation::new()?;
    for choice in ["normal", "isolated", "cancel"] {
        // @step Given a card without an associated agent and an unrelated open agent
        let backend = Arc::new(common::MockBackend::new());
        let mut app = app_with(backend.clone());
        intent_fixture::add(&mut app, &backend, "unrelated", Some("AUTH-002")).await?;
        app.navigator_mut().active_view = ViewMode::Board;
        let assignments = backend.set_work_unit_context_calls();
        // @step When the user chooses Open in agent from the card chooser
        chooser(&mut app);
        app.dispatch(Action::WorkUnitsLoaded(vec![
            summary("AUTH-002"),
            summary("AUTH-001"),
        ]));
        app.board_store_mut().select_work_unit("AUTH-002", 20);
        press(&mut app, KeyCode::Enter);
        // @step Then the captured card offers normal isolated and cancel creation choices
        let rendered = text(&mut app);
        for label in ["Work on AUTH-001?", "Yes", "Yes - Isolated", "Cancel"] {
            assert!(
                rendered.contains(label),
                "missing creation choice {label}: {rendered}"
            );
        }
        assert_eq!(backend.create_session_calls(), 0);
        assert_eq!(backend.create_isolated_session_calls(), 0);
        assert_eq!(backend.set_work_unit_context_calls(), assignments);
        // @step And only a confirmed creation attaches the captured card to its returned agent
        if choice == "cancel" {
            press(&mut app, KeyCode::Right);
            press(&mut app, KeyCode::Right);
            press(&mut app, KeyCode::Enter);
            assert_eq!(backend.create_session_calls(), 0);
            assert_eq!(backend.create_isolated_session_calls(), 0);
            assert_eq!(backend.set_work_unit_context_calls(), assignments);
        } else {
            intent_fixture::script(&backend, choice == "isolated", "created");
            if choice == "isolated" {
                press(&mut app, KeyCode::Right);
            }
            press(&mut app, KeyCode::Enter);
            settle(&mut app).await?;
            assert_eq!(
                backend.create_session_calls(),
                usize::from(choice == "normal")
            );
            assert_eq!(
                backend.create_isolated_session_calls(),
                usize::from(choice == "isolated")
            );
            assert_eq!(
                intent_fixture::focus(&app, "created").as_deref(),
                Some("AUTH-001")
            );
            assert_eq!(backend.set_work_unit_context_calls(), assignments + 1);
        }
        assert_eq!(
            intent_fixture::focus(&app, "unrelated").as_deref(),
            Some("AUTH-002")
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn mux_enter_overlays_the_chooser_without_replacing_the_grid() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given a focused mux board pane and associated and unrelated open agents
    let backend = Arc::new(common::MockBackend::new());
    let mut app = app_with(backend.clone());
    intent_fixture::add(&mut app, &backend, "associated", Some("AUTH-001")).await?;
    intent_fixture::add(&mut app, &backend, "second-associated", Some("AUTH-001")).await?;
    intent_fixture::add(&mut app, &backend, "unrelated", Some("AUTH-002")).await?;
    app.navigator_mut().mux.enable_default();
    app.navigator_mut().active_view = ViewMode::Mux;
    let _ = text(&mut app);
    let layout = app.navigator().mux.config().clone();
    let focus = app.navigator().mux.focus();
    assert_eq!(focus, 0, "mux fixture must focus the board pane");
    let assignments = backend.set_work_unit_context_calls();
    // @step When Enter opens the chooser and Escape cancels before reopening and confirming
    chooser(&mut app);
    press(&mut app, KeyCode::Esc);
    // @step Then cancellation preserves layout pane focus card and agent assignments
    assert_eq!(app.navigator().mux.config(), &layout);
    assert_eq!(app.navigator().mux.focus(), focus);
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.current_session(), Some(SessionId::new("unrelated")));
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    assert_eq!(
        app.board_store()
            .selected_work_unit()
            .map(|card| card.id.as_str()),
        Some("AUTH-001")
    );
    // @step And confirmation focuses the earliest associated agent inside the same grid
    chooser(&mut app);
    press(&mut app, KeyCode::Enter);
    settle(&mut app).await?;
    assert_eq!(app.current_session(), Some(SessionId::new("associated")));
    assert_eq!(
        app.navigator().mux.focused_session_id(),
        Some(SessionId::new("associated"))
    );
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_mux_layout_preserved(app.navigator().mux.config(), &layout);
    assert_ne!(app.navigator().mux.focus(), focus);
    assert_eq!(backend.set_work_unit_context_calls(), assignments);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn empty_board_enter_does_not_offer_card_actions() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given an empty focused board column
    let backend = Arc::new(common::MockBackend::new());
    let mut app = App::new(backend.clone());
    // @step When the user presses Enter
    event(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    );
    // @step Then no chooser or agent creation opens and the board remains unchanged
    assert!(app.compositor().is_empty());
    assert_eq!(app.active_view(), ViewMode::Board);
    assert_eq!(app.current_session(), None);
    assert_eq!(backend.create_session_calls(), 0);
    assert_eq!(backend.create_isolated_session_calls(), 0);
    Ok(())
}
