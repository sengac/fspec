// ─────────────────────────────────────────────────────────────────────────
// Draft preservation — MultiLineInput parity (RPC-404/405 geometry)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Draft text and cursor survive the pause round-trip
#[test]
fn draft_text_and_cursor_survive_the_pause_round_trip() {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    view.input.set_value("deploy the fix");
    assert_eq!(view.input.cursor(), (0, 14), "cursor parked after 'fix'");

    // @step And session s-1 has a Triple pause slot
    store.set_pause_state(sid("s-1"), triple_state("Read: approval", None));

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);
    assert!(
        rows[input.y as usize].contains("⏸ Read"),
        "prompt painted during the pause"
    );

    // @step And the pause slot for s-1 is cleared as if the user denied
    store.clear_pause_state(&sid("s-1"));

    // @step And the agent view renders another frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view);

    // @step Then the input buffer contains exactly "deploy the fix"
    assert_eq!(view.input.value(), "deploy the fix");

    // @step And the cursor is at the end of "deploy the fix"
    assert_eq!(
        view.input.cursor(),
        (0, 14),
        "cursor untouched by the pause"
    );

    // @step And the rendered input area shows the draft text again
    assert!(
        rows[input.y as usize].contains("deploy the fix"),
        "draft repainted after the pause; got: {}",
        rows[input.y as usize]
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape — modal removal + Esc-deny lock
// ─────────────────────────────────────────────────────────────────────────

// Scenario: The pause modal is deleted and resume is unreachable from the prompt
#[test]
fn the_pause_modal_is_deleted_and_resume_is_unreachable_from_the_prompt() {
    // @step Given the codelet-fspec-tui crate sources
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the file rust/fspec-tui/src/components/pause_dialog.rs does not exist
    assert!(
        !src.join("components").join("pause_dialog.rs").exists(),
        "components/pause_dialog.rs must be deleted"
    );

    // @step And components/mod.rs declares no OpenPauseDialog action variant
    let components = codelet_test_helpers::source_composition::read_source(
        &src.join("components").join("mod.rs"),
    )
    .expect("read mod.rs");
    assert!(
        !components.contains("OpenPauseDialog"),
        "Action::OpenPauseDialog must be removed"
    );
    assert!(
        !components.contains("pub mod pause_dialog"),
        "pub mod pause_dialog must be removed"
    );

    // @step And views/agent/pause_keys.rs and app/dispatch_pause_hitl.rs never construct Action::PauseResumed from a pause-prompt key path
    let pause_keys = std::fs::read_to_string(src.join("views").join("agent").join("pause_keys.rs"))
        .expect("pause_keys.rs exists");
    assert!(
        !pause_keys.contains("PauseResumed"),
        "the pause-prompt key path must never emit Action::PauseResumed"
    );
    let dph = std::fs::read_to_string(src.join("app").join("dispatch_pause_hitl.rs"))
        .expect("dispatch_pause_hitl.rs exists");
    assert!(
        !dph.contains("send(Action::PauseResumed"),
        "dispatch_pause_hitl.rs must not construct/send Action::PauseResumed"
    );

    // @step And the files pause_prompt.rs, pause_keys.rs, input_area.rs, and store/agent_view/pause_state.rs each stay under 300 lines
    let files = [
        src.join("views").join("agent").join("pause_prompt.rs"),
        src.join("views").join("agent").join("pause_keys.rs"),
        src.join("views").join("agent").join("input_area.rs"),
        src.join("store").join("agent_view").join("pause_state.rs"),
    ];
    for f in files {
        let body =
            std::fs::read_to_string(&f).unwrap_or_else(|_| panic!("{} must exist", f.display()));
        let n = body.lines().count();
        assert!(n < 300, "{} has {n} lines, must be < 300", f.display());
    }
}
