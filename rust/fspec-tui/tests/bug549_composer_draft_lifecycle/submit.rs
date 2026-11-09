// Feature: spec/features/composer-draft-belongs-to-one-session.feature
// ─────────────────────────────────────────────────────────────────────
// Submitting ends the draft (rule 2)
// ─────────────────────────────────────────────────────────────────────

// Scenario: Pressing Enter before the draft save fires leaves no draft anywhere
#[tokio::test(start_paused = true)]
async fn pressing_enter_before_the_draft_save_fires_leaves_no_draft_anywhere() -> TestResult {
    // @step Given session s-1 is focused and session s-2 is open
    let (mut app, mock) = app_with(&["s-1", "s-2"]).await?;
    focus_previous(&mut app).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step And the user types "deploy the fix" into the composer
    type_text(&mut app, "deploy the fix").await?;
    // @step When the user presses Enter before the 300ms draft save fires
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    assert_eq!(
        mock.last_send_input(),
        Some((sid("s-1"), "deploy the fix".to_string()))
    );
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer is empty
    assert_eq!(composer(&app), "");
    // @step And the in-memory draft of s-1 is empty
    assert_eq!(memory_draft(&app, "s-1")?, "");
    // @step And the daemon draft of s-1 is empty
    assert_eq!(daemon_draft(&mock, "s-1").await?, "");
    // @step When the user presses Shift+Right and then Shift+Left
    shift(&mut app, KeyCode::Right).await?;
    shift(&mut app, KeyCode::Left).await?;
    assert_eq!(app.current_session(), Some(sid("s-1")));
    // @step Then the composer is empty
    assert_eq!(composer(&app), "");
    Ok(())
}

// Scenario: Steering a running turn ends the draft
#[tokio::test(start_paused = true)]
async fn steering_a_running_turn_ends_the_draft() -> TestResult {
    // @step Given session s-1 is focused and its turn is running
    let (mut app, mock) = app_with(&["s-1"]).await?;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And the user types "also check the logs" into the composer
    type_text(&mut app, "also check the logs").await?;
    // @step When the user presses Enter
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then "also check the logs" is submitted to s-1 as a steer
    assert_eq!(
        mock.submitted_inputs(),
        vec![(
            sid("s-1"),
            "also check the logs".to_string(),
            codelet_rpc_types::InputMode::Steer
        )]
    );
    // @step And the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}

// Scenario: Queueing behind a running turn ends the draft
#[tokio::test(start_paused = true)]
async fn queueing_behind_a_running_turn_ends_the_draft() -> TestResult {
    // @step Given session s-1 is focused and its turn is running
    let (mut app, mock) = app_with(&["s-1"]).await?;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And the user types "next task" into the composer
    type_text(&mut app, "next task").await?;
    // @step When the user presses Alt+Enter
    press(&mut app, KeyCode::Enter, KeyModifiers::ALT).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then "next task" is submitted to s-1 as a queued input
    assert_eq!(
        mock.submitted_inputs(),
        vec![(
            sid("s-1"),
            "next task".to_string(),
            codelet_rpc_types::InputMode::Queue
        )]
    );
    // @step And the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}

// Scenario: A slash command handled locally ends the draft
#[tokio::test(start_paused = true)]
async fn a_slash_command_handled_locally_ends_the_draft() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the user types "/role reviewer" into the composer
    type_text(&mut app, "/role reviewer").await?;
    // @step When the user presses Enter
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the role "reviewer" is set on s-1 and nothing is sent to the model
    assert_eq!(mock.set_session_role_calls(), 1);
    assert_eq!(mock.send_input_calls(), 0);
    assert!(mock.submitted_inputs().is_empty());
    // @step And the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}

// Scenario: A slash command picked from the popup ends the draft
#[tokio::test(start_paused = true)]
async fn a_slash_command_picked_from_the_popup_ends_the_draft() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the user types "/hel" into the composer
    type_text(&mut app, "/hel").await?;
    // @step When the user picks Help in the slash command popup
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the help dialog opens
    assert!(app.compositor().contains("help-dialog"));
    // @step And the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}
