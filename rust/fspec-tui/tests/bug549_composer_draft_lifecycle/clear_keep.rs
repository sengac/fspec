// Feature: spec/features/composer-draft-belongs-to-one-session.feature
// ─────────────────────────────────────────────────────────────────────
// Clear and delete remove the draft (rule 6); an unsubmitted draft
// survives (rule 7)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Another client clearing the session removes its draft everywhere
#[tokio::test(start_paused = true)]
async fn another_client_clearing_the_session_removes_its_draft_everywhere() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the user types "scratch" into the composer
    type_text(&mut app, "scratch").await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    assert_eq!(daemon_draft(&mock, "s-1").await?, "scratch");
    // @step When another client clears session s-1
    app.dispatch(Action::SessionCleared(sid("s-1")));
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}

// Scenario: Deleting the focused session from the resume list shows the next session's draft
#[tokio::test(start_paused = true)]
async fn deleting_the_focused_session_from_the_resume_list_shows_the_next_session_draft(
) -> TestResult {
    // @step Given session s-2 holds the draft "s2 draft"
    let (mut app, _mock) = app_with(&["s-1", "s-2"]).await?;
    type_text(&mut app, "s2 draft").await?;
    // @step And session s-1 is focused
    focus_previous(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step When the user opens the resume list with "/resume"
    type_text(&mut app, "/resume").await?;
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    assert!(app.navigator().agent.resume_view.is_some());
    // @step And the user deletes session s-1 there
    app.dispatch(Action::ConfirmDeleteSession(sid("s-1")));
    drain(&mut app).await?;
    // @step Then the composer shows "s2 draft"
    assert_eq!(app.current_session(), Some(sid("s-2")));
    assert_eq!(composer(&app), "s2 draft");
    Ok(())
}

// Scenario: An unsubmitted draft survives cycling, the board and a reattach
#[tokio::test(start_paused = true)]
async fn an_unsubmitted_draft_survives_cycling_the_board_and_a_reattach() -> TestResult {
    // @step Given session s-1 is focused and session s-2 is open
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    focus_previous(&mut app).await?;
    // @step And the user types "keep me" into the composer
    type_text(&mut app, "keep me").await?;
    // @step When the user presses Shift+Right and then Shift+Left
    shift(&mut app, KeyCode::Right).await?;
    shift(&mut app, KeyCode::Left).await?;
    // @step Then the composer shows "keep me"
    assert_eq!(composer(&app), "keep me");
    // @step When the user presses Shift+Left to go to the board and Shift+Right to come back
    shift(&mut app, KeyCode::Left).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    shift(&mut app, KeyCode::Right).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step Then the composer shows "keep me"
    assert_eq!(app.current_session(), Some(sid("s-1")));
    assert_eq!(composer(&app), "keep me");
    // @step When the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step And a fresh TUI attaches to the same daemon and opens s-1
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut fresh = App::new(backend);
    fresh.navigator_mut().active_view = ViewMode::Agent;
    fresh.dispatch(Action::SessionCreated(sid("s-1")));
    drain(&mut fresh).await?;
    // @step Then the composer shows "keep me"
    assert_eq!(composer(&fresh), "keep me");
    Ok(())
}
