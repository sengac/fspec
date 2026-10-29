// ─────────────────────────────────────────────────────────────────────────
// Removed invented behaviors + paste routing (options mode)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Hotkeys, Tab, scroll, and paste do nothing on an options question
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hotkeys_tab_scroll_and_paste_do_nothing_on_an_options_question() {
    // @step Given an App with a MockBackend and a focused session s-1
    let (mut app, mock) = fresh_app(&["s-1"]);

    // @step And the composer input contains the draft "agent draft"
    app.navigator_mut().agent.input.set_value("agent draft");

    // @step And session s-1 has an inline HITL prompt with one question with options "Yes" and "No"
    app.dispatch(Action::HitlPromptFetched {
        session_id: sid("s-1"),
        request: yes_no_request("q-1"),
    });
    render_app(&mut app);
    let before = hitl_slot(&app, &sid("s-1"))
        .expect("slot active")
        .selected_option;

    // @step When the user types the characters "abc" and presses Tab
    for c in "abc".chars() {
        app.handle_event(&key_event(KeyCode::Char(c)));
        drain_pending(&mut app).await;
    }
    app.handle_event(&key_event(KeyCode::Tab));
    drain_pending(&mut app).await;

    // @step Then backend.send_hitl_response is NEVER called
    assert_eq!(
        mock.send_hitl_response_calls(),
        0,
        "no hotkeys / Tab-cycle: letters and Tab must never select or submit"
    );

    // @step And the HITL selection for s-1 is unchanged
    assert_eq!(
        hitl_slot(&app, &sid("s-1"))
            .expect("slot active")
            .selected_option,
        before,
        "letters and Tab must not move the selection"
    );

    // @step When a paste of "clipboard\ncontents" arrives
    let _ = app.handle_paste("clipboard\ncontents");
    drain_pending(&mut app).await;

    // @step Then the composer input still contains exactly "agent draft"
    assert_eq!(
        app.navigator().agent.input.value(),
        "agent draft",
        "options-mode paste must be swallowed, never reach the composer"
    );

    // @step And the HITL prompt for s-1 is still active
    assert!(
        hitl_slot(&app, &sid("s-1")).is_some(),
        "swallowed keys must not dismiss the prompt"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape — modal removal lock (mirrors RPC-406 pause-modal lock)
// ─────────────────────────────────────────────────────────────────────────

// Scenario: The HITL modal is deleted and no construction site remains
#[test]
fn the_hitl_modal_is_deleted_and_no_construction_site_remains() {
    // @step Given the codelet-fspec-tui crate sources
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then the file rust/fspec-tui/src/components/hitl_dialog.rs does not exist
    assert!(
        !src.join("components").join("hitl_dialog.rs").exists(),
        "components/hitl_dialog.rs must be deleted"
    );

    // @step And the crate sources never mention HITL_DIALOG_ID or HitlDialog
    fn rust_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    let composed = [
        src.join("components/mod.rs"),
        src.join("transport/mod.rs"),
        src.join("transport/embedded.rs"),
        src.join("transport/websocket.rs"),
        src.join("views/navigator_mux_events.rs"),
    ];
    for f in &files {
        let body = if composed.contains(f) {
            codelet_test_helpers::source_composition::read_source(f)
        } else {
            std::fs::read_to_string(f)
        }
        .expect("read source file");
        assert!(
            !body.contains("HITL_DIALOG_ID"),
            "{} still mentions HITL_DIALOG_ID",
            f.display()
        );
        assert!(
            !body.contains("HitlDialog"),
            "{} still mentions HitlDialog",
            f.display()
        );
    }

    // @step And components/mod.rs declares no OpenHitlDialog action variant
    let components = codelet_test_helpers::source_composition::read_source(
        &src.join("components").join("mod.rs"),
    )
    .expect("read mod.rs");
    assert!(
        !components.contains("OpenHitlDialog"),
        "Action::OpenHitlDialog must be replaced by Action::HitlPromptFetched"
    );

    // @step And the files hitl_state.rs, hitl_keys.rs, and hitl_prompt.rs each stay under 300 lines
    let files = [
        src.join("store").join("agent_view").join("hitl_state.rs"),
        src.join("views").join("agent").join("hitl_keys.rs"),
        src.join("views").join("agent").join("hitl_prompt.rs"),
    ];
    for f in files {
        let body =
            std::fs::read_to_string(&f).unwrap_or_else(|_| panic!("{} must exist", f.display()));
        let n = body.lines().count();
        assert!(n < 300, "{} has {n} lines, must be < 300", f.display());
    }
}
