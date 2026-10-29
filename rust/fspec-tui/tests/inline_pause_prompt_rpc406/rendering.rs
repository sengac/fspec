// ─────────────────────────────────────────────────────────────────────────
// Rendering — TS InputTransition.tsx:467-533 parity
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Triple pause prompt renders inline with options, colors, and hint
#[test]
fn triple_pause_prompt_renders_inline_with_options_colors_and_hint() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Triple pause slot with prompt "Read: Access to .env requires approval" and details ".env"
    store.set_pause_state(
        sid("s-1"),
        triple_state("Read: Access to .env requires approval", Some(".env")),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Read: Access to .env requires approval (.env)"
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Read: Access to .env requires approval (.env)"),
        "triple prompt row missing; got: {r0}"
    );

    // @step And the "⏸ Read" prefix is cyan and the "(.env)" details are dim
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Cyan,
        "⏸ glyph must be cyan"
    );
    let read_x = char_x(r0, "Read:");
    assert_eq!(
        buf[(read_x, input.y)].fg,
        Color::Cyan,
        "tool name must be cyan"
    );
    let details_x = char_x(r0, "(.env)");
    assert!(
        buf[(details_x, input.y)].modifier.contains(Modifier::DIM),
        "details must be dim"
    );

    // @step And the input area's second row shows "[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)"
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains("[Allow Once] [Allow Session] [Deny] (←/→ Navigate | Enter Select | Esc Deny)"),
        "options row missing; got: {r1}"
    );

    // @step And "[Allow Once]" is green, "[Allow Session]" is blue, and "[Deny]" is red
    let once_x = char_x(r1, "[Allow Once]");
    let session_x = char_x(r1, "[Allow Session]");
    let deny_x = char_x(r1, "[Deny]");
    assert_eq!(buf[(once_x, y1)].fg, Color::Green, "[Allow Once] green");
    assert_eq!(buf[(session_x, y1)].fg, Color::Blue, "[Allow Session] blue");
    assert_eq!(buf[(deny_x, y1)].fg, Color::Red, "[Deny] red");

    // @step And "[Allow Once]" is rendered inverse because the selection defaults to 0
    assert!(
        buf[(once_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Allow Once] must be inverse when selection is 0"
    );
    assert!(
        !buf[(session_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Allow Session] must NOT be inverse"
    );
    assert!(
        !buf[(deny_x, y1)].modifier.contains(Modifier::REVERSED),
        "[Deny] must NOT be inverse"
    );

    // @step And the navigation hint is dim
    let hint_x = char_x(r1, "(←/→ Navigate");
    assert!(
        buf[(hint_x, y1)].modifier.contains(Modifier::DIM),
        "hint must be dim"
    );
}

// Scenario: Confirm pause prompt renders yellow header, dim details line, and Y/N row
#[test]
fn confirm_pause_prompt_renders_yellow_header_dim_details_and_yn_row() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot with prompt "Bash: Run rm -rf build?" and details "rm -rf build"
    store.set_pause_state(
        sid("s-1"),
        confirm_state("Bash: Run rm -rf build?", Some("rm -rf build")),
    );

    // @step When the agent view renders a frame
    let (rows, buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Bash: Run rm -rf build?" in yellow
    let r0 = &rows[input.y as usize];
    assert!(
        r0.contains("⏸ Bash: Run rm -rf build?"),
        "confirm header missing; got: {r0}"
    );
    let pause_x = char_x(r0, "⏸");
    assert_eq!(
        buf[(pause_x, input.y)].fg,
        Color::Yellow,
        "confirm header must be yellow"
    );

    // @step And the input area's second row shows the dim details line "rm -rf build"
    let y1 = input.y + 1;
    let r1 = &rows[y1 as usize];
    assert!(
        r1.contains("rm -rf build"),
        "details line missing; got: {r1}"
    );
    let details_x = char_x(r1, "rm -rf build");
    assert!(
        buf[(details_x, y1)].modifier.contains(Modifier::DIM),
        "details line must be dim"
    );

    // @step And the input area's third row shows "[Y] Approve [N] Deny (Esc to cancel)"
    let y2 = input.y + 2;
    let r2 = &rows[y2 as usize];
    assert!(
        r2.contains("[Y] Approve [N] Deny (Esc to cancel)"),
        "Y/N row missing; got: {r2}"
    );

    // @step And "[Y] Approve" is green, "[N] Deny" is red, and "(Esc to cancel)" is dim
    let approve_x = char_x(r2, "[Y] Approve");
    let deny_x = char_x(r2, "[N] Deny");
    let esc_x = char_x(r2, "(Esc to cancel)");
    assert_eq!(buf[(approve_x, y2)].fg, Color::Green, "[Y] Approve green");
    assert_eq!(buf[(deny_x, y2)].fg, Color::Red, "[N] Deny red");
    assert!(
        buf[(esc_x, y2)].modifier.contains(Modifier::DIM),
        "(Esc to cancel) must be dim"
    );
}

// Scenario: Confirm pause prompt without details omits the details line
#[test]
fn confirm_pause_prompt_without_details_omits_the_details_line() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Confirm pause slot with prompt "Bash: Run build?" and no details
    store.set_pause_state(sid("s-1"), confirm_state("Bash: Run build?", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area is 2 rows tall
    assert_eq!(input.height, 2, "confirm without details is 2 rows");

    // @step And the input area's second row shows "[Y] Approve [N] Deny (Esc to cancel)"
    let r1 = &rows[(input.y + 1) as usize];
    assert!(
        r1.contains("[Y] Approve [N] Deny (Esc to cancel)"),
        "Y/N row missing; got: {r1}"
    );
}

// Scenario: Pause prompt replaces the draft and paints no hardware cursor
#[test]
fn pause_prompt_replaces_the_draft_and_paints_no_hardware_cursor() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix"
    view.input.set_value("deploy the fix");

    // @step And session s-1 has a Triple pause slot with prompt "Read: approval" and no details
    store.set_pause_state(sid("s-1"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the rendered input area does not contain the text "deploy the fix"
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("deploy the fix"),
            "draft chars must not be painted while paused; row {y}: {}",
            rows[y as usize]
        );
    }

    // @step And the rendered input area contains "⏸ Read"
    assert!(
        rows[input.y as usize].contains("⏸ Read"),
        "prompt must be painted; got: {}",
        rows[input.y as usize]
    );

    // @step And the cursor is not visible while the pause prompt is showing
    assert!(
        !view.is_cursor_visible(&store, Some(SessionStatus::Paused)),
        "hardware cursor must not be painted inside the pause prompt"
    );
}

// Scenario: Prompt renders only when the paused session is focused
#[test]
fn prompt_renders_only_when_the_paused_session_is_focused() {
    // @step Given the agent view is rendered with open sessions s-1 and s-2 and s-1 focused
    let (mut view, mut store) = view_harness(&["s-1", "s-2"]);
    assert_eq!(store.current_session(), Some(&sid("s-1")));

    // @step And session s-2 has a Triple pause slot with prompt "Read: approval" and no details
    store.set_pause_state(sid("s-2"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area shows the normal input, not a pause prompt
    for y in input.y..input.y + input.height {
        assert!(
            !rows[y as usize].contains("⏸"),
            "no pause glyph for a non-focused session's pause; row: {}",
            rows[y as usize]
        );
    }

    // @step When focus switches to session s-2 and the agent view renders a frame
    store.focus_session_index(1);
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input area's first row shows "⏸ Read: approval"
    assert!(
        rows[input.y as usize].contains("⏸ Read: approval"),
        "focused paused session must show its prompt; got: {}",
        rows[input.y as usize]
    );
}

// Scenario: Long triple prompt header wraps instead of clipping
#[test]
fn long_triple_prompt_header_wraps_instead_of_clipping() {
    // @step Given the agent view is rendered on a narrow terminal with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And session s-1 has a Triple pause slot whose prompt and details exceed the input-area width
    let prompt = "Read: Access to /home/user/project/secrets/.env requires approval";
    let details = "/home/user/project/secrets/.env";
    store.set_pause_state(sid("s-1"), triple_state(prompt, Some(details)));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(44, 16, &mut store, &mut view);

    // @step Then the prompt header wraps across two or more input-area rows
    assert!(
        input.height >= 3,
        "input area must grow to hold a wrapped header + options row; height = {}",
        input.height
    );
    let header_rows = input.height - 1;
    assert!(
        header_rows >= 2,
        "header must occupy 2+ rows; got {header_rows}"
    );

    // @step And no header characters are lost to clipping
    // The padded body starts at x=1 and is 42 cols wide (1-col pad each
    // side). Interior wrapped rows are full slices, so concatenating the
    // body cells of every header row and trimming the tail must
    // reproduce the full header text.
    let header_text = format!("⏸ {prompt} ({details})");
    let body: String = (input.y..input.y + header_rows)
        .map(|y| {
            let row = &rows[y as usize];
            row.chars().skip(1).take(42).collect::<String>()
        })
        .collect();
    assert_eq!(
        body.trim_end(),
        header_text,
        "wrapped header must preserve every character"
    );

    // @step And the options row "[Allow Once] [Allow Session] [Deny]" and the hint still render below the wrapped header
    let opts_row = &rows[(input.y + header_rows) as usize];
    assert!(
        opts_row.contains("[Allow Once] [Allow Session] [Deny]"),
        "options row must render below the wrapped header; got: {opts_row}"
    );
    assert!(
        opts_row.contains("(←/→"),
        "hint must start rendering after the options; got: {opts_row}"
    );
}
