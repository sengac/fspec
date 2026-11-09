// Feature: spec/features/composer-draft-belongs-to-one-session.feature
// ─────────────────────────────────────────────────────────────────────
// Every focus route swaps the drafts (rule 1)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Attaching to another session from the resume list swaps the drafts
#[tokio::test(start_paused = true)]
async fn attaching_to_another_session_from_the_resume_list_swaps_the_drafts() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the user types "half typed" into the composer
    type_text(&mut app, "half typed").await?;
    // @step And the daemon holds no draft for session s-2
    assert_eq!(daemon_draft(&mock, "s-2").await?, "");
    // @step When the user attaches to session s-2 from the resume list
    app.dispatch(Action::AttachToSession(sid("s-2")));
    drain(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step Then the composer is empty
    assert_eq!(composer(&app), "");
    // @step And the in-memory draft of s-1 is "half typed"
    assert_eq!(memory_draft(&app, "s-1")?, "half typed");
    // @step When the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the daemon draft of s-1 is "half typed"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "half typed");
    // @step When the user presses Shift+Left
    shift(&mut app, KeyCode::Left).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step Then the composer shows "half typed"
    assert_eq!(composer(&app), "half typed");
    Ok(())
}

// Scenario: A new session opens with an empty composer
#[tokio::test(start_paused = true)]
async fn a_new_session_opens_with_an_empty_composer() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, _mock) = app_with(&["s-1"]).await?;
    // @step And the user types "half typed" into the composer
    type_text(&mut app, "half typed").await?;
    // @step When a new session s-2 is created and focused
    app.dispatch(Action::SessionCreated(sid("s-2")));
    drain(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step Then the composer is empty
    assert_eq!(composer(&app), "");
    // @step And the in-memory draft of s-1 is "half typed"
    assert_eq!(memory_draft(&app, "s-1")?, "half typed");
    // @step And the in-memory draft of s-2 is empty
    assert_eq!(memory_draft(&app, "s-2")?, "");
    Ok(())
}

// Scenario: A child session spawned in mux mode does not take the focused draft
#[tokio::test(start_paused = true)]
async fn a_child_session_spawned_in_mux_mode_does_not_take_the_focused_draft() -> TestResult {
    // @step Given mux mode shows the board and two agent panes
    let (mut app, _mock) = app_with(&["s-1"]).await?;
    enter_mux(&mut app, "board agent agent").await?;
    assert_eq!(app.navigator().active_view, ViewMode::Mux);
    // @step And session s-1 is focused in an agent pane
    click_pane(&mut app, 1).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the user types "half typed" into the composer
    type_text(&mut app, "half typed").await?;
    // @step When a child session s-2 is spawned
    app.dispatch(Action::SessionCreated(sid("s-2")));
    drain(&mut app).await?;
    render(&mut app);
    // @step Then the in-memory draft of s-2 is empty
    assert_eq!(memory_draft(&app, "s-2")?, "");
    // @step And the composer shows "half typed"
    assert_eq!(app.current_session(), Some(sid("s-1")));
    assert_eq!(composer(&app), "half typed");
    // @step And the in-memory draft of s-1 is "half typed"
    assert_eq!(memory_draft(&app, "s-1")?, "half typed");
    Ok(())
}

// Scenario: Closing the focused tab shows the next session's own draft
#[tokio::test(start_paused = true)]
async fn closing_the_focused_tab_shows_the_next_session_own_draft() -> TestResult {
    // @step Given session s-1 holds the draft "s1 draft"
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    focus_previous(&mut app).await?;
    type_text(&mut app, "s1 draft").await?;
    // @step And session s-2 is focused
    shift(&mut app, KeyCode::Right).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the user types "throwaway" into the composer
    type_text(&mut app, "throwaway").await?;
    // @step When the user closes the focused session's tab before the 300ms draft save fires
    // TUI-124: closing goes through the close dialog and its facts.
    app.dispatch(Action::MenuZoneCActivate(2));
    drain(&mut app).await?;
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "s1 draft"
    assert_eq!(app.current_session(), Some(sid("s-1")));
    assert_eq!(composer(&app), "s1 draft");
    // @step And no save of "throwaway" reaches the daemon
    assert!(
        !saved(&mock, "throwaway"),
        "a closed session's draft must never be saved: {:?}",
        mock.pending_input_writes()
    );
    Ok(())
}

// Scenario: Activating another session's chip from the board swaps the drafts
#[tokio::test(start_paused = true)]
async fn activating_another_session_chip_from_the_board_swaps_the_drafts() -> TestResult {
    // @step Given session s-2 holds the draft "s2 text"
    let (mut app, _mock) = app_with(&["s-1", "s-2"]).await?;
    type_text(&mut app, "s2 text").await?;
    // @step And session s-1 is focused
    focus_previous(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the user types "s1 text" into the composer
    type_text(&mut app, "s1 text").await?;
    // @step And the user goes back to the board
    shift(&mut app, KeyCode::Left).await?;
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    // @step When the user activates the s-2 chip in the board menu bar
    app.dispatch(Action::MenuChipActivate(1));
    drain(&mut app).await?;
    // @step Then the agent view shows s-2 with the composer holding "s2 text"
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    assert_eq!(app.current_session(), Some(sid("s-2")));
    assert_eq!(composer(&app), "s2 text");
    // @step And the in-memory draft of s-1 is "s1 text"
    assert_eq!(memory_draft(&app, "s-1")?, "s1 text");
    Ok(())
}

// Scenario: Clicking another mux agent pane swaps the drafts and keeps both saves
#[tokio::test(start_paused = true)]
async fn clicking_another_mux_agent_pane_swaps_the_drafts_and_keeps_both_saves() -> TestResult {
    // @step Given mux mode shows the board and two agent panes for sessions s-1 and s-2
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    enter_mux(&mut app, "board agent agent").await?;
    assert_eq!(app.navigator().active_view, ViewMode::Mux);
    // @step And the s-1 pane is focused
    click_pane(&mut app, 1).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the user types "alpha" into the composer
    type_text(&mut app, "alpha").await?;
    // @step When the user clicks the s-2 pane before the 300ms draft save fires
    click_pane(&mut app, 2).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the user types "beta" into the composer
    type_text(&mut app, "beta").await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "beta"
    assert_eq!(composer(&app), "beta");
    // @step And the daemon draft of s-1 is "alpha"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "alpha");
    // @step And the daemon draft of s-2 is "beta"
    assert_eq!(daemon_draft(&mock, "s-2").await?, "beta");
    Ok(())
}
