// ─────────────────────────────────────────────────────────────────────────
// Selection wrap — useHitlInput.ts:210-222 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Up and Down wrap the selection across options and the virtual Other entry
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn up_and_down_wrap_the_selection_across_options_and_the_virtual_other_entry() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, _mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question with options "Deploy" and "Skip"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Deploy", "d"), opt("Skip", "s")],
        ),
    });
    render_app(&mut app);

    // @step When the user presses Down three times
    for _ in 0..3 {
        app.handle_event(&key_event(KeyCode::Down));
        drain_pending(&mut app).await;
    }

    // @step Then the HITL selection for s-1 is back on option 0
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        0,
        "3 Downs over 2 options + Other must wrap back to 0"
    );

    // @step When the user presses Up
    app.handle_event(&key_event(KeyCode::Up));
    drain_pending(&mut app).await;

    // @step Then the HITL selection for s-1 is on the virtual "Other..." entry at index 2
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        2,
        "Up from 0 must wrap to the virtual Other... index"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Multi-question accumulation — useHitlInput.ts:134-151 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Answers accumulate across questions and submit together after the last question
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answers_accumulate_across_questions_and_submit_together_after_the_last_question() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with two options questions "q-1" with options "Yes" and "No" and "q-2" with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: HitlRequest {
            questions: vec![
                question("q-1", "H1", "Q1?", vec![opt("Yes", "y"), opt("No", "n")]),
                question("q-2", "H2", "Q2?", vec![opt("Yes", "y"), opt("No", "n")]),
            ],
        },
    });
    render_app(&mut app);

    // @step When the user presses Enter on question 1 with "Yes" selected
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response has not been called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "nothing may be sent before the last question is answered"
    );

    // @step And the prompt advances to question 2 with the selection reset to 0
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot still active");
    assert_eq!(slot.question_index, 1, "must advance to question 2");
    assert_eq!(slot.selected_option, 0, "selection must reset to 0");

    // @step When the user presses Down then Enter on question 2
    render_app(&mut app);
    app.handle_event(&key_event(KeyCode::Down));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and answers [{id q-1, selected [Yes]}, {id q-2, selected [No]}]
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(!response.cancelled, "submit must carry cancelled:false");
    assert_eq!(
        response.answers,
        vec![
            HitlAnswer {
                id: "q-1".to_string(),
                selected: vec!["Yes".to_string()],
                other: None,
            },
            HitlAnswer {
                id: "q-2".to_string(),
                selected: vec!["No".to_string()],
                other: None,
            },
        ]
    );

    // @step And the HITL prompt for s-1 is cleared
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "slot must clear after the final submit"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Esc/cancel correctness — the stranding-bug fix
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Esc on an options question cancels the whole request through the backend
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_on_an_options_question_cancels_the_whole_request_through_the_backend() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.navigator_mut().agent.input.set_value("keep this draft");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: yes_no_request("q-1"),
    });
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled true and empty answers
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(response.cancelled, "Esc must send cancelled:true");
    assert!(response.answers.is_empty(), "cancel carries no answers");

    // @step And the HITL prompt for s-1 is cleared
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "no path may leave the slot set after cancelling"
    );

    // @step And the composer draft is left untouched
    assert_eq!(
        app.navigator().agent.input.value(),
        "keep this draft",
        "Esc-cancel must not clear the remaining composer draft"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Slot lifecycle — HITL wins over pause, Running clears
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paused chunk stores a HITL slot that wins over the pause slot and Running clears it
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paused_chunk_stores_a_hitl_slot_that_wins_over_the_pause_slot_and_running_clears_it() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the MockBackend's get_pause_state is scripted to return Some PauseState for s-1
    mock.script_pause_state(
        sid("s-1"),
        Some(PauseState {
            kind: PauseKind::Confirm,
            prompt: "p".to_string(),
            tool_call_id: None,
        }),
    );

    // @step And the MockBackend's get_hitl_request is scripted to return Some HitlRequest for s-1
    let request = yes_no_request("q-1");
    mock.script_hitl_request(sid("s-1"), Some(request.clone()));

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
        || hitl_slot(&app, &sid("s-1")).is_some(),
        "HITL slot set from the Paused chunk",
    )
    .await;

    // @step Then the HITL prompt slot for s-1 holds the fetched request
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot set");
    assert_eq!(slot.request, request, "slot must hold the wire request");

    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "HITL wins on tie — no pause slot may be set"
    );

    // @step And no compositor layer is mounted for the HITL prompt
    assert_eq!(
        app.compositor().len(),
        0,
        "the inline prompt must never mount a modal layer; got ids: {:?}",
        app.compositor().layer_ids()
    );

    // @step When Action::ChunkReceived(s-1, SessionStateChange Running) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Running,
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the HITL prompt slot for s-1 is empty
    assert!(
        hitl_slot(&app, &sid("s-1")).is_none(),
        "Running must clear the stale HITL prompt"
    );
}
