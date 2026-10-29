/// Paint one App frame and return the rendered rows + buffer (used by
/// the freeform/Other-mode scenarios to assert painted content).
fn render_app_frame(app: &mut App) -> (Vec<String>, Buffer) {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    let rows: Vec<String> = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    (rows, buf)
}

/// Drive the App into Other mode on a fresh one-question options
/// prompt: navigate the selection onto the virtual "Other..." entry
/// (index = options.len()) and press Enter.
async fn enter_other_mode(app: &mut App) {
    render_app(app);
    app.handle_event(&key_event(KeyCode::Up));
    drain_pending(app).await;
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(app).await;
    render_app(app);
}

// ─────────────────────────────────────────────────────────────────────────
// Other... freeform mode — TOOL-018 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Enter on Other activates freeform mode with the shared input, empty-submit hint, and submit
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_on_other_activates_freeform_mode_with_the_shared_input_empty_submit_hint_and_submit()
{
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question "q-1" with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Yes", "y"), opt("No", "n")],
        ),
    });

    // @step When the user navigates the selection to "Other..." and presses Enter
    enter_other_mode(&mut app).await;

    // @step Then the prompt is in Other mode and the header hint shows " (Enter Submit | Esc Back to options)" in dim
    assert!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .other_active,
        "Enter on Other... must activate Other mode"
    );
    let (rows, buf) = render_app_frame(&mut app);
    let header_y = (0..rows.len())
        .find(|&y| rows[y].contains("⏸ Header: Question?"))
        .expect("HITL header row rendered");
    let hr = &rows[header_y];
    assert!(
        hr.contains(" (Enter Submit | Esc Back to options)"),
        "Other-mode header hint missing; got: {hr}"
    );
    let hint_x = char_x(hr, "(Enter Submit | Esc Back to options)");
    assert!(
        buf[(hint_x, header_y as u16)]
            .modifier
            .contains(Modifier::DIM),
        "header hint must be dim"
    );

    // @step And the shared composer input renders with the placeholder "Type your answer..."
    assert!(
        rows.iter().any(|r| r.contains("Type your answer...")),
        "shared input placeholder missing"
    );

    // @step When the user presses Enter with the shared input empty
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then the yellow hint "  ⚠ Please type a response or press Esc to go back" is shown
    assert!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "empty Enter must set show_empty_hint"
    );
    let (rows, buf) = render_app_frame(&mut app);
    let hint_y = (0..rows.len())
        .find(|&y| rows[y].contains("⚠ Please type a response or press Esc to go back"))
        .expect("empty-submit hint row rendered");
    let warn_x = char_x(&rows[hint_y], "⚠");
    assert_eq!(
        buf[(warn_x, hint_y as u16)].fg,
        Color::Yellow,
        "empty-submit hint must be yellow"
    );

    // @step And backend.send_hitl_response has not been called
    assert_eq!(mock.send_hitl_response_calls(), 0);

    // @step When the user types "ship it"
    for c in "ship it".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }

    // @step Then the empty-submit hint is cleared
    assert!(
        !hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .show_empty_hint,
        "typing must clear the empty-submit hint"
    );

    // @step When the user presses Enter
    app.handle_event(&key_event(KeyCode::Enter));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is called exactly once with cancelled false and one answer {id q-1, selected [], other "ship it"}
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
            other: Some("ship it".to_string()),
        }]
    );

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "captured answer must clear the shared input"
    );

    // @step And the HITL prompt for s-1 is cleared
    assert!(hitl_slot(&app, &sid("s-1")).is_none());
}

// Scenario: Esc in Other mode returns to the options list without sending anything
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_in_other_mode_returns_to_the_options_list_without_sending_anything() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt in Other mode with the empty-submit hint showing and "half an answer" in the shared input
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
    app.handle_event(&key_event(KeyCode::Enter)); // empty submit → hint
    drain_pending(&mut app).await;
    assert!(hitl_slot(&app, &sid("s-1")).expect("slot").show_empty_hint);
    app.navigator_mut().agent.input.set_value("half an answer");
    render_app(&mut app);

    // @step When the user presses Esc
    app.handle_event(&key_event(KeyCode::Esc));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is NEVER called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "Esc in Other mode is local only — nothing may be sent"
    );

    // @step And the prompt is back in options mode with the empty-submit hint cleared
    let slot = hitl_slot(&app, &sid("s-1")).expect("slot still active");
    assert!(!slot.other_active, "must return to options mode");
    assert!(!slot.show_empty_hint, "hint must be cleared");

    // @step And the shared composer input is empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "Esc from Other mode must clear the shared input value"
    );

    // @step And the HITL prompt for s-1 is still active
    assert!(hitl_slot(&app, &sid("s-1")).is_some());
}
