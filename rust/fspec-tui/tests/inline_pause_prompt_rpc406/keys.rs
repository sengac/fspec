// ─────────────────────────────────────────────────────────────────────────
// Keys — TS AgentView.tsx:4521-4607 parity, Esc DENIES
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Left and Right cycle the triple selection with wraparound
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn left_and_right_cycle_the_triple_selection_with_wraparound() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Right twice
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 2
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );

    // @step When the user presses Right once more
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );

    // @step When the user presses Left
    app.handle_event(&key_event(KeyCode::Left));
    drain_pending(&mut app).await;

    // @step Then the triple pause selection for s-1 is 2
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );
}

// Scenario: Enter sends the selected approval choice and resets the selection
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_sends_the_selected_approval_choice_and_resets_the_selection() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Right then Enter
    app.handle_event(&key_event(KeyCode::Right));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::ApproveSession)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), ApprovalChoice::ApproveSession));

    // @step And the pause slot for s-1 is cleared
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "pause slot must be cleared after Enter"
    );

    // @step And the triple pause selection for s-1 is reset to 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );
}

// Scenario: Esc on a triple prompt denies instead of resuming
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_a_triple_prompt_denies_instead_of_resuming() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: Access to .env requires approval", Some(".env")),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-1, ApprovalChoice::Deny)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), ApprovalChoice::Deny));

    // @step And backend.pause_resume is NEVER called
    assert_eq!(
        mock.pause_resume_calls(),
        0,
        "Esc must DENY, never resume (security fix)"
    );

    // @step And the pause slot for s-1 is cleared
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
}

// Scenario: Y approves a confirm prompt
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn y_approves_a_confirm_prompt() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses the character "y"
    app.handle_event(&key_event(KeyCode::Char('y')));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, true)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), true));
}

// Scenario: N denies a confirm prompt
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn n_denies_a_confirm_prompt() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses the character "n"
    app.handle_event(&key_event(KeyCode::Char('n')));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, false)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    let log = mock.pause_confirm_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-1"), false));
}

// Scenario: Esc on a confirm prompt denies
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_a_confirm_prompt_denies() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: confirm_state("Bash: Run rm -rf build?", None),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.pause_confirm is called exactly once with (s-1, false)
    wait_until(
        || mock.pause_confirm_calls() == 1,
        "pause_confirm called once",
    )
    .await;
    assert_eq!(mock.pause_confirm_log()[0], (sid("s-1"), false));

    // @step And backend.pause_resume is NEVER called
    assert_eq!(
        mock.pause_resume_calls(),
        0,
        "Esc must deny the confirm prompt, never resume"
    );
}

// Scenario: Printable keys are swallowed while the pause prompt is showing
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn printable_keys_are_swallowed_while_the_pause_prompt_is_showing() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And the input buffer contains the draft "hello"
    app.navigator_mut().agent.input.set_value("hello");

    // @step And session s-1 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user types the characters "abc"
    for c in "abc".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }

    // @step Then the input buffer still contains exactly "hello"
    assert_eq!(
        app.navigator_mut().agent.input.value(),
        "hello",
        "printable keys must never reach the MultiLineInput while paused"
    );
}
