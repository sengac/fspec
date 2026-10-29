// ─────────────────────────────────────────────────────────────────────────
// Dispatch and store — chunk-driven slot lifecycle
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paused chunk with pause state stores a per-session slot without mounting a modal
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paused_chunk_with_pause_state_stores_a_per_session_slot_without_mounting_a_modal() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some Triple PauseState for s-1
    let state = triple_state("Read: Access to .env requires approval", Some(".env"));
    mock.script_pause_state(sid("s-1"), Some(state.clone()));

    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;
    wait_until(
        || {
            app.agent_view_store()
                .pause_state_for(&sid("s-1"))
                .is_some()
        },
        "pause slot set",
    )
    .await;

    // @step Then the AgentViewStore pause slot for s-1 holds the fetched PauseState
    let slot = app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .expect("slot set");
    assert_eq!(slot.kind, PauseKind::Triple);
    assert_eq!(slot.prompt, "Read: Access to .env requires approval");
    assert_eq!(slot.tool_call_id.as_deref(), Some(".env"));

    // @step And no compositor layer with id "pause-dialog" is mounted
    assert!(
        !pause_dialog_layer_mounted(&app),
        "the modal PauseDialog must never mount for pause states"
    );
}

// Scenario: Stale Paused chunk with no pause state sets no slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_paused_chunk_with_no_pause_state_sets_no_slot() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);

    // @step And the MockBackend's get_hitl_request is scripted to return None for s-1
    mock.script_hitl_request(sid("s-1"), None);

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "stale Paused chunk must not set a slot"
    );

    // @step And no compositor layer with id "pause-dialog" is mounted
    assert!(!pause_dialog_layer_mounted(&app));
}

// Scenario: HITL wins on tie and no pause slot is set
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hitl_wins_on_tie_and_no_pause_slot_is_set() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(sid("s-1"), Some(confirm_state("p", None)));

    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    mock.script_hitl_request(sid("s-1"), Some(hitl_request("q-1", &["Yes", "No"])));

    // @step When Action::ChunkReceived(s-1, SessionStateChange Paused) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));

    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the Compositor contains a layer with id HITL_DIALOG_ID
    // RPC-411: the HitlDialog modal was deleted — HITL now wins into
    // the per-session inline HITL slot (no compositor layer).
    wait_until(
        || {
            app.agent_view_store()
                .hitl_prompt_for(&sid("s-1"))
                .is_some()
        },
        "HITL slot wins on tie",
    )
    .await;

    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "HITL wins on tie — no pause slot"
    );
}

// Scenario: Running chunk clears the pause slot and resets the selection
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn running_chunk_clears_the_pause_slot_and_resets_the_selection() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has a Triple pause slot with selection 2
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-1"),
        state: triple_state("Read: approval", None),
    });
    app.dispatch(Action::PausePromptNav {
        session_id: sid("s-1"),
        delta: 1,
    });
    app.dispatch(Action::PausePromptNav {
        session_id: sid("s-1"),
        delta: 1,
    });
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        2
    );

    // @step When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());

    // @step And the triple pause selection for s-1 is reset to 0
    assert_eq!(
        app.agent_view_store()
            .triple_pause_selection_for(&sid("s-1")),
        0
    );
}

// Scenario: Prompt keys route to the paused session in a multi-session app
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_keys_route_to_the_paused_session_in_a_multi_session_app() {
    // @step Given an App with a MockBackend and open sessions s-1 and s-2 with s-2 focused
    let (mut app, mock) = fresh_app(&["s-1", "s-2"]);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-2")));

    // @step And session s-2 has a Triple pause slot
    app.dispatch(Action::PauseStateFetched {
        session_id: sid("s-2"),
        state: triple_state("Read: approval", None),
    });
    render_app(&mut app);

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.pause_triple is called exactly once with (s-2, ApprovalChoice::Approve)
    wait_until(
        || mock.pause_triple_calls() == 1,
        "pause_triple called once",
    )
    .await;
    let log = mock.pause_triple_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], (sid("s-2"), ApprovalChoice::Approve));

    // @step And backend.pause_triple is NOT called for s-1
    assert!(!log.iter().any(|(s, _)| s == &sid("s-1")));
}
