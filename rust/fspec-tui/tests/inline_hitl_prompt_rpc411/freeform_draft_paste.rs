// ─────────────────────────────────────────────────────────────────────────
// Freeform question — shared composer draft semantics
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Freeform question consumes the composer draft as the answer
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn freeform_question_consumes_the_composer_draft_as_the_answer() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the composer input contains the draft "deploy the fix"
    app.navigator_mut().agent.input.set_value("deploy the fix");

    // @step And session s-1 has an inline HITL prompt with one freeform question "q-1" without options
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request("q-1", "Header", "Question?", vec![]),
    });

    // @step When the agent view renders a frame
    let (rows, buf) = render_app_frame(&mut app);

    // @step Then the header hint shows " (Enter Submit | Esc Cancel)" in dim
    let header_y = (0..rows.len())
        .find(|&y| rows[y].contains("⏸ Header: Question?"))
        .expect("HITL header row rendered");
    let hr = &rows[header_y];
    assert!(
        hr.contains(" (Enter Submit | Esc Cancel)"),
        "plain-freeform header hint missing; got: {hr}"
    );
    let hint_x = char_x(hr, "(Enter Submit | Esc Cancel)");
    assert!(
        buf[(hint_x, header_y as u16)]
            .modifier
            .contains(Modifier::DIM),
        "header hint must be dim"
    );

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "deploy the fix"}
    wait_until(
        || mock.send_hitl_response_calls() == 1,
        "send_hitl_response called once",
    )
    .await;
    let log = mock.send_hitl_response_log();
    assert_eq!(log.len(), 1);
    let (session, response) = &log[0];
    assert_eq!(session, &sid("s-1"));
    assert!(!response.cancelled);
    assert_eq!(
        response.answers,
        vec![HitlAnswer {
            id: "q-1".to_string(),
            selected: vec![],
            other: Some("deploy the fix".to_string()),
        }]
    );

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "the consumed draft must be cleared after capture"
    );
}

// Scenario: Composer draft and cursor survive an options-only HITL round-trip
#[test]
fn composer_draft_and_cursor_survive_an_options_only_hitl_round_trip() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    view.input.set_value("deploy the fix");
    assert_eq!(view.input.cursor(), (0, 14), "cursor parked after 'fix'");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    );

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the rendered input area does not contain the text "deploy the fix"
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("deploy the fix"),
            "draft chars must not paint during an options question; row {y}: {}",
            rows[y as usize]
        );
    }

    // @step When the HITL prompt for s-1 is cleared as if the user answered
    store.clear_hitl_prompt(&sid("s-1"));

    // @step And the agent view renders another frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);
    assert!(
        rows[input.y as usize].contains("deploy the fix"),
        "draft repainted after the round-trip; got: {}",
        rows[input.y as usize]
    );

    // @step Then the input buffer contains exactly "deploy the fix"
    assert_eq!(view.input.value(), "deploy the fix");

    // @step And the cursor is at the end of "deploy the fix"
    assert_eq!(
        view.input.cursor(),
        (0, 14),
        "cursor untouched by the options-only round-trip"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Paste routing — freeform/Other mode
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Paste in Other mode inserts into the shared input
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paste_in_other_mode_inserts_into_the_shared_input() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt in Other mode with an empty shared input
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    });
    enter_other_mode(&mut app).await;
    assert!(hitl_slot(&app, &sid("s-1")).expect("slot").other_active);
    assert_eq!(app.navigator().agent.input.value(), "");

    // @step When a paste of "pasted answer" arrives
    let _ = app.handle_paste("pasted answer");
    drain_pending(&mut app).await;

    // @step Then the shared composer input contains "pasted answer"
    assert_eq!(
        app.navigator().agent.input.value(),
        "pasted answer",
        "Other-mode paste must fall through to the shared MultiLineInput"
    );

    // @step And backend.send_hitl_response is NEVER called
    assert_eq!(mock.send_hitl_response_calls(), 0);
}
