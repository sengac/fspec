// Feature: spec/features/composer-draft-belongs-to-one-session.feature
// ─────────────────────────────────────────────────────────────────────
// Text put in the composer by code is a draft like typing (rule 5)
// ─────────────────────────────────────────────────────────────────────

// Scenario: A recalled history entry becomes the draft
#[tokio::test(start_paused = true)]
async fn a_recalled_history_entry_becomes_the_draft() -> TestResult {
    // @step Given session s-1 is focused and its input history holds "earlier prompt"
    let (mut app, mock) = app_with(&["s-1"]).await?;
    mock.script_history(sid("s-1"), vec!["earlier prompt".to_string()]);
    // @step When the user presses Shift+Up
    shift(&mut app, KeyCode::Up).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "earlier prompt"
    assert_eq!(composer(&app), "earlier prompt");
    // @step And the in-memory draft of s-1 is "earlier prompt"
    assert_eq!(memory_draft(&app, "s-1")?, "earlier prompt");
    // @step And the daemon draft of s-1 is "earlier prompt"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "earlier prompt");
    Ok(())
}

// Scenario: Text picked in the history search becomes the draft
#[tokio::test(start_paused = true)]
async fn text_picked_in_the_history_search_becomes_the_draft() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step When the user picks "searched prompt" in the Ctrl+R history search
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL).await?;
    assert!(app.navigator().agent.search_view.is_some());
    // The search view's pick emits InsertIntoInput with the match text.
    app.dispatch(Action::InsertIntoInput("searched prompt".to_string()));
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer shows "searched prompt"
    assert_eq!(composer(&app), "searched prompt");
    // @step And the in-memory draft of s-1 is "searched prompt"
    assert_eq!(memory_draft(&app, "s-1")?, "searched prompt");
    // @step And the daemon draft of s-1 is "searched prompt"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "searched prompt");
    Ok(())
}

// Scenario: The merge-conflict context is saved as the session's draft
#[tokio::test(start_paused = true)]
async fn the_merge_conflict_context_is_saved_as_the_session_draft() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step When a merge of s-1 reports conflicts in "src/lib.rs"
    mock.seed_merge_outcome(codelet_rpc_types::MergeOutcome {
        status: codelet_rpc_types::MergeStatus::Conflict,
        conflicts: vec!["src/lib.rs".to_string()],
        merge_commit: None,
        worktree_path: None,
    });
    app.dispatch(Action::MergeConfirmed {
        session_id: sid("s-1"),
    });
    drain(&mut app).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then the composer holds the conflict context naming "src/lib.rs"
    let context = composer(&app);
    assert!(
        context.contains("Merge produced conflicts") && context.contains("src/lib.rs"),
        "the composer holds the conflict context: {context:?}"
    );
    // @step And the in-memory draft and the daemon draft of s-1 hold the same conflict context
    assert_eq!(memory_draft(&app, "s-1")?, context);
    assert_eq!(daemon_draft(&mock, "s-1").await?, context);
    Ok(())
}

// Scenario: Answering a command's stdin prompt never touches the draft
#[tokio::test(start_paused = true)]
async fn answering_a_command_stdin_prompt_never_touches_the_draft() -> TestResult {
    // @step Given session s-1 is focused and its turn is running
    let (mut app, mock) = app_with(&["s-1"]).await?;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And the user types "chat draft" into the composer
    type_text(&mut app, "chat draft").await?;
    // @step And a command run by s-1 may be waiting for input
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: codelet_rpc_types::ExecStdinRequest {
                exec_session_id: "exec-abc".to_string(),
                command: "git commit".to_string(),
                quiet_seconds: 5,
                ts_ms: 1_700_000_000_000,
            },
        },
    ));
    drain(&mut app).await?;
    // @step When the user opens the stdin answer box, types "y" and presses Enter
    press(&mut app, KeyCode::Char('o'), KeyModifiers::CONTROL).await?;
    type_text(&mut app, "y").await?;
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then "y" is written to the command's stdin
    assert_eq!(
        mock.write_exec_stdin_log(),
        vec![(sid("s-1"), "exec-abc".to_string(), "y".to_string())]
    );
    // @step And the composer shows "chat draft"
    assert_eq!(composer(&app), "chat draft");
    // @step And the in-memory draft of s-1 is "chat draft"
    assert_eq!(memory_draft(&app, "s-1")?, "chat draft");
    // @step And the daemon draft of s-1 is "chat draft"
    assert_eq!(daemon_draft(&mock, "s-1").await?, "chat draft");
    Ok(())
}

// Scenario: A captured HITL freeform answer ends the draft
#[tokio::test(start_paused = true)]
async fn a_captured_hitl_freeform_answer_ends_the_draft() -> TestResult {
    // @step Given session s-1 is focused
    let (mut app, mock) = app_with(&["s-1"]).await?;
    // @step And the user types "deploy the fix" into the composer
    type_text(&mut app, "deploy the fix").await?;
    // @step And a freeform HITL question is showing for s-1
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        route: hitl_route(&sid("s-1")),
        request: codelet_rpc_types::HitlRequest {
            questions: vec![codelet_rpc_types::HitlQuestion {
                id: "q-1".to_string(),
                header: "Header".to_string(),
                question: "What should I do?".to_string(),
                options: vec![],
            }],
        },
    });
    drain(&mut app).await?;
    // @step When the user presses Enter
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    // @step And the draft save window has passed
    pass_save_window(&mut app).await?;
    // @step Then "deploy the fix" is sent as the HITL answer
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    assert_eq!(
        log.first().map(|(_, response)| response.answers.clone()),
        Some(vec![codelet_rpc_types::HitlAnswer {
            id: "q-1".to_string(),
            selected: vec![],
            other: Some("deploy the fix".to_string()),
        }])
    );
    // @step And the composer, the in-memory draft and the daemon draft of s-1 are empty
    assert_eq!(all_copies(&app, &mock, "s-1").await?, empty_copies());
    Ok(())
}
