// Feature: spec/features/composer-draft-belongs-to-one-session.feature
// ─────────────────────────────────────────────────────────────────────
// Saves are ordered per session (rule 3); hydration never overwrites
// newer local text (rule 4)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Typing in another session never cancels a pending save
#[tokio::test(start_paused = true)]
async fn typing_in_another_session_never_cancels_a_pending_save() -> TestResult {
    // @step Given session s-1 is focused and session s-2 is open
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    focus_previous(&mut app).await?;
    // @step And the user types "alpha" into the composer
    type_text(&mut app, "alpha").await?;
    // @step When the user presses Shift+Right before the 300ms draft save fires
    shift(&mut app, KeyCode::Right).await?;
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the user types "beta" into the composer
    type_text(&mut app, "beta").await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the daemon draft of s-1 is "alpha"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "alpha");
    // @step And the daemon draft of s-2 is "beta"
    assert_eq!(daemon_draft(&mock, "s-2").await?, "beta");
    Ok(())
}

// Scenario: Text typed before the attach hydration answers is kept
#[tokio::test(start_paused = true)]
async fn text_typed_before_the_attach_hydration_answers_is_kept() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the daemon holds the draft "old draft" for session s-2
    mock.script_pending_input(sid("s-2"), Some("old draft".to_string()));
    // @step When the user attaches to session s-2 from the resume list
    // (dispatched without settling: the daemon has not answered yet)
    app.dispatch(Action::AttachToSession(sid("s-2")));
    assert_eq!(app.current_session(), Some(sid("s-2")));
    // @step And the user types "new" into the composer before the daemon answers
    for character in "new".chars() {
        let _ = app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char(character),
            KeyModifiers::NONE,
        )));
    }
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "new"
    assert_eq!(composer(&app), "new");
    // @step And the in-memory draft of s-2 is "new"
    assert_eq!(memory_draft(&app, "s-2")?, "new");
    // @step And the daemon draft of s-2 is "new"
    assert_eq!(daemon_draft(&mock, "s-2").await?, "new");
    Ok(())
}

// Scenario: Re-attaching a session keeps a draft the daemon has not stored yet
#[tokio::test(start_paused = true)]
async fn re_attaching_a_session_keeps_a_draft_the_daemon_has_not_stored_yet() -> TestResult {
    // @step Given session s-1 is focused and session s-2 is open
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    focus_previous(&mut app).await?;
    // @step And the user types "fresh text" into the composer
    type_text(&mut app, "fresh text").await?;
    // @step And another client has left the draft "stale text" for s-1 in the daemon
    mock.script_pending_input(sid("s-1"), Some("stale text".to_string()));
    // @step When the user attaches to session s-1 from the resume list before the 300ms draft save fires
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "fresh text"
    assert_eq!(composer(&app), "fresh text");
    // @step And the in-memory draft of s-1 is "fresh text"
    assert_eq!(memory_draft(&app, "s-1")?, "fresh text");
    // @step And the daemon draft of s-1 is "fresh text"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "fresh text");
    Ok(())
}
