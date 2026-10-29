// ─────────────────────────────────────────────────────────────────────────
// Error tolerance
// ─────────────────────────────────────────────────────────────────────────

// Scenario: Backend.get_hitl_request error during the chunk-driven probe sets no slot
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_hitl_request_error_sets_no_slot() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's get_pause_state is scripted to return None for s-1
    mock.script_pause_state(sid("s-1"), None);
    // @step And the MockBackend's get_hitl_request is scripted to return Err("decode failed") for s-1
    mock.set_get_hitl_request_error("decode failed".to_string());

    // @step When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        },
    ));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
    // @step And the AgentViewStore pause slot for s-1 is empty
    assert!(app
        .agent_view_store()
        .pause_state_for(&sid("s-1"))
        .is_none());
    // @step And the session s-1 scrollback contains no chunks mentioning "get_hitl_request"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session present");
    for ch in &ctx.scrollback.visible_window(1024) {
        for line in &ch.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("get_hitl_request"),
                "no error notice in scrollback (saw: {text})"
            );
        }
    }
}

// Scenario: Backend.send_hitl_response error on Esc-cancel still clears the slot and emits no scrollback notice
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_hitl_response_error_still_clears_slot_and_emits_no_notice() {
    // @step Given an App with a MockBackend
    // @step And session s-1 is the current session
    let (mut app, mock) = fresh_app_with_session("s-1");
    // @step And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    mock.set_send_hitl_response_error("decode failed".to_string());
    // @step And session s-1 has an active HITL slot
    seed_hitl_slot(&mut app, &sid("s-1"), hitl_request("q-1", &["Yes"]));

    // @step When the user presses Esc on the inline HITL prompt
    app.handle_event(&key_event(KeyCode::Esc));
    // @step And all pending tasks have drained
    drain_pending(&mut app).await;

    // @step Then the App must not panic
    // @step And the AgentViewStore HITL slot for s-1 is empty
    assert!(!hitl_slot_set(&app, &sid("s-1")));
    // @step And the session s-1 scrollback contains no chunks mentioning "send_hitl_response"
    let ctx = app
        .agent_view_store()
        .session_context_for(&sid("s-1"))
        .expect("session present");
    for ch in &ctx.scrollback.visible_window(1024) {
        for line in &ch.lines {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                !text.contains("send_hitl_response"),
                "no error notice in scrollback (saw: {text})"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Source shape
// ─────────────────────────────────────────────────────────────────────────

// Scenario: rust/fspec-tui/src/app/dispatch_pause_hitl.rs hosts the new pause/HITL helpers
#[test]
fn dispatch_pause_hitl_hosts_the_pause_hitl_helpers() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // @step Given the file rust/fspec-tui/src/app/dispatch_pause_hitl.rs exists
    let dph_path = src.join("app").join("dispatch_pause_hitl.rs");
    assert!(dph_path.exists(), "dispatch_pause_hitl.rs must exist");

    // @step When the file is compiled as part of codelet-fspec-tui
    let dph = std::fs::read_to_string(&dph_path).expect("read dispatch_pause_hitl.rs");

    // @step Then it must declare impl App methods named handle_pause_chunk, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    for method in [
        "fn handle_pause_chunk",
        "fn handle_pause_confirmed",
        "fn handle_pause_triple",
        "fn handle_pause_resumed",
        "fn handle_hitl_submitted",
        "fn handle_pause_cleared",
    ] {
        assert!(dph.contains(method), "missing {method}");
    }

    // @step And rust/fspec-tui/src/app/mod.rs must declare pub mod dispatch_pause_hitl and pub mod dispatch_hitl_prompt
    let app_mod = std::fs::read_to_string(src.join("app").join("mod.rs")).expect("read app/mod.rs");
    assert!(app_mod.contains("pub mod dispatch_pause_hitl"));
    assert!(app_mod.contains("pub mod dispatch_hitl_prompt"));

    // @step And rust/fspec-tui/src/components/mod.rs must declare no pub mod hitl_dialog and no pub mod pause_dialog
    let components = codelet_test_helpers::source_composition::read_source(
        &src.join("components").join("mod.rs"),
    )
    .expect("read components/mod.rs");
    assert!(!components.contains("pub mod hitl_dialog"));
    assert!(!components.contains("pub mod pause_dialog"));

    // @step And rust/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
    let dispatch =
        std::fs::read_to_string(src.join("app").join("dispatch.rs")).expect("read dispatch.rs");
    assert!(
        dispatch.lines().count() <= 300,
        "app/dispatch.rs exceeds 300 lines"
    );
}
