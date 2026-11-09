// ─────────────────────────────────────────────────────────────────────────
// Draft preservation — MultiLineInput parity (RPC-404/405 geometry)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Draft text and cursor survive the pause round-trip
#[test]
fn draft_text_and_cursor_survive_the_pause_round_trip() -> TestResult {
    // @step Given the agent view is rendered with a focused session s-1
    let (mut view, mut store) = view_harness(&["s-1"]);

    // @step And the input buffer contains the draft "deploy the fix" with the cursor after "fix"
    view.input.set_value("deploy the fix");
    assert_eq!(view.input.cursor(), (0, 14), "cursor parked after 'fix'");

    // @step And session s-1 has a Triple pause slot
    store.set_pause_state(
        sid("s-1"),
        pause_route(&sid("s-1")),
        triple_state("Read: approval", None),
    );

    // @step When the agent view renders a frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view)?;
    assert!(
        rows[input.y as usize].contains("⏸ Read"),
        "prompt painted during the pause"
    );

    // @step And the pause slot for s-1 is cleared as if the user denied
    store.clear_pause_state(&sid("s-1"));

    // @step And the agent view renders another frame
    let (rows, _buf, input) = render_frame(100, 14, &mut store, &mut view)?;

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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape — modal removal + Esc-deny lock
// ─────────────────────────────────────────────────────────────────────────

// Scenario: The pause modal is deleted and resume is unreachable from the prompt
#[test]
fn the_pause_modal_is_deleted_and_resume_is_unreachable_from_the_prompt() -> TestResult {
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
    .or_fail("read mod.rs")?;
    assert!(
        !components.contains("OpenPauseDialog"),
        "Action::OpenPauseDialog must be removed"
    );
    assert!(
        !components.contains("pub mod pause_dialog"),
        "pub mod pause_dialog must be removed"
    );

    // @step And views/agent/pause_keys.rs sends Action::PauseHumanAction only from the human-action prompt, never from an approval or confirm prompt
    let pause_keys = std::fs::read_to_string(src.join("views").join("agent").join("pause_keys.rs"))
        .or_fail("pause_keys.rs exists")?;
    let handler = pause_keys.split("fn handle_pause_prompt_key").nth(1).or_fail("key handler")?;
    let (approval_arms, human_arm) = handler
        .split_once("PauseKind::HumanAction =>")
        .or_fail("pause_keys.rs has a human-action arm")?;
    assert!(
        !approval_arms.contains("PauseHumanAction") && human_arm.contains("PauseHumanAction"),
        "only the human-action arm may emit Action::PauseHumanAction (AMGR-023)"
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
            std::fs::read_to_string(&f).or_fail(&format!("{} must exist", f.display()))?;
        let n = body.lines().count();
        assert!(n < 300, "{} has {n} lines, must be < 300", f.display());
    }
    Ok(())
}
