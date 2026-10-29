// ─────────────────────────────────────────────────────────────────────────
// Rendering — TS InputTransition.tsx:427-463 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Options question renders inline with radios, colors, Other entry, and footer
#[test]
fn options_question_renders_inline_with_radios_colors_other_entry_and_footer() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with one question "Header" / "Question?" and options "Deploy — push to prod" and "Skip — do nothing"
    store.set_hitl_prompt(
        sid("s-1"),
        one_question_request(
            "q-1",
            "Header",
            "Question?",
            vec![opt("Deploy", "push to prod"), opt("Skip", "do nothing")],
        ),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's header row shows "⏸ Header: Question?" with a magenta "⏸ " glyph, a bold header, and no "[1/1]" progress marker
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Header: Question?"),
        "header row missing; got: {r0}"
    );
    assert!(
        !r0.contains("[1/1]"),
        "single-question request must show no [n/m] marker; got: {r0}"
    );
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Magenta,
        "⏸ glyph must be magenta"
    );
    let header_x = char_x(r0, "Header");
    assert!(
        buf[(header_x, input.y)].modifier.contains(Modifier::BOLD),
        "header must be bold"
    );

    // @step And an option row shows " ● Deploy" with the radio and label green and " — push to prod" dim
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains(" ● Deploy — push to prod"),
        "selected option row missing; got: {r1}"
    );
    let radio_x = char_x(r1, "●");
    assert_eq!(buf[(radio_x, y1)].fg, Color::Green, "selected radio green");
    let label_x = char_x(r1, "Deploy");
    assert_eq!(buf[(label_x, y1)].fg, Color::Green, "selected label green");
    let desc_x = char_x(r1, "— push to prod");
    assert!(
        buf[(desc_x, y1)].modifier.contains(Modifier::DIM),
        "description must be dim"
    );

    // @step And an option row shows " ○ Skip" with the radio and label white
    let y2 = input.y + 2;
    let r2 = &rows[y2 as usize];
    assert!(r2.contains(" ○ Skip"), "unselected row missing; got: {r2}");
    let skip_radio_x = char_x(r2, "○");
    assert_eq!(
        buf[(skip_radio_x, y2)].fg,
        Color::White,
        "unselected radio white"
    );
    let skip_x = char_x(r2, "Skip");
    assert_eq!(buf[(skip_x, y2)].fg, Color::White, "unselected label white");

    // @step And the row below the real options shows " ○ Other..." with the label dim and italic
    let y3 = input.y + 3;
    let r3 = &rows[y3 as usize];
    assert!(r3.contains(" ○ Other..."), "Other row missing; got: {r3}");
    let other_x = char_x(r3, "Other...");
    assert!(
        buf[(other_x, y3)].modifier.contains(Modifier::DIM),
        "Other... label must be dim"
    );
    assert!(
        buf[(other_x, y3)].modifier.contains(Modifier::ITALIC),
        "Other... label must be italic"
    );

    // @step And the footer row shows " (↑/↓ Navigate | Enter Select | Esc Cancel)" in dim
    let y4 = input.y + 4;
    let r4 = &rows[y4 as usize];
    assert!(
        r4.contains(" (↑/↓ Navigate | Enter Select | Esc Cancel)"),
        "footer row missing; got: {r4}"
    );
    let footer_x = char_x(r4, "(↑/↓ Navigate");
    assert!(
        buf[(footer_x, y4)].modifier.contains(Modifier::DIM),
        "footer must be dim"
    );

    // @step And no modal layer is mounted on the compositor
    // (view-level harness has no compositor; the App-level lifecycle
    // scenario locks the no-modal invariant — here we lock that the
    // prompt paints inside the input area, not a popup.)
    assert!(input.height >= 5, "inline prompt occupies the input area");
}

// Scenario: Multi-question request shows a magenta question progress marker
#[test]
fn multi_question_request_shows_a_magenta_question_progress_marker() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has an inline HITL prompt with three options questions
    store.set_hitl_prompt(
        sid("s-1"),
        HitlRequest {
            questions: (1..=3)
                .map(|i| {
                    question(
                        &format!("q-{i}"),
                        &format!("Header-{i}"),
                        &format!("Question-{i}?"),
                        vec![opt("Yes", "y"), opt("No", "n")],
                    )
                })
                .collect(),
        },
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's header row shows "⏸ [1/3] Header-1: Question-1?"
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ [1/3] Header-1: Question-1?"),
        "multi-question header missing; got: {r0}"
    );

    // @step And the "[1/3] " progress marker is magenta
    let marker_x = char_x(r0, "[1/3]");
    assert_eq!(
        buf[(marker_x, input.y)].fg,
        Color::Magenta,
        "[1/3] marker must be magenta"
    );
}
