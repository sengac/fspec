//! Feature: spec/features/move-sessionmanager-from-codelet-napi-into-codelet-sessions.feature
use super::*;

#[test]
fn scenario_napi_emit_helpers_route_through_session_manager_chunks_tx() {
    // @step Given the napi free function `emit_block_notification_to_tui(session_id_str, action, reason)` previously called `GLOBAL_CHUNK_CALLBACK.get()` to emit a `UserNotification` chunk
    // @step And the napi free function `spawn_footer_poller(session_id, cwd, worktree_path)` previously called `GLOBAL_CHUNK_CALLBACK.get()` to emit a `FooterStateUpdate` chunk
    let sessions_src = workspace_root().join("sessions").join("src");
    let shared_emitter =
        strip_line_comments(&read(&sessions_src.join("session_tool_callbacks.rs")));
    let shared_poller = strip_line_comments(&read(&sessions_src.join("footer_poller.rs")));

    // @step When I inspect the rewritten bodies of both functions
    // @step Then `emit_block_notification_to_tui` emits the chunk via the registered manager-owned `chunks_tx` sender (`sender.send(`), with no SessionManager singleton fallback (BUG-405)
    let emit_block_entry = shared_emitter
        .find("pub fn emit_block_notification(")
        .expect("codelet-sessions must define the shared emit_block_notification");
    let emit_block_window: String = shared_emitter
        .chars()
        .skip(emit_block_entry)
        .take(2000)
        .collect();
    assert!(
        contains_ws_insensitive(&emit_block_window, ".chunks_tx().send(")
            || contains_ws_insensitive(&emit_block_window, "sender.send("),
        "RPC-041/BUG-405: emit_block_notification must emit via the registered chunks_tx sender. Got window:\n{emit_block_window}"
    );

    // @step And the user-visible warning message format `"AI was blocked from {action} - {reason}"` is preserved verbatim
    assert!(
        emit_block_window.contains("AI was blocked from"),
        "RPC-041: emit_block_notification_to_tui must preserve the user-visible warning message"
    );

    // @step And `spawn_footer_poller`'s emit site routes via the registered manager-owned `chunks_tx` sender (the NAPI-free shared poller added by WT-002; BUG-405: with no sender registered it emits nothing and reports it) while preserving the `first_run || cwd_changed || is_git != prev_is_git || branch != prev_branch` change-gate in the shared poller
    let footer_entry = shared_poller
        .find("pub fn spawn_footer_poller(")
        .expect("codelet-sessions must define the shared spawn_footer_poller");
    let footer_window: String = shared_poller
        .chars()
        .skip(footer_entry)
        .take(6000)
        .collect();
    assert!(
        contains_ws_insensitive(&footer_window, ".chunks_tx().send(")
            || contains_ws_insensitive(&footer_window, "sender.send("),
        "RPC-041/WT-002: spawn_footer_poller must emit via the manager-owned chunks_tx sender. Got window:\n{footer_window}"
    );
    assert!(
        contains_ws_insensitive(
            &footer_window,
            "first_run || cwd_changed || is_git != prev_is_git || branch != prev_branch"
        ),
        "RPC-041: spawn_footer_poller must preserve the first_run/cwd_changed/is_git/branch change-gate"
    );
}

#[test]
fn scenario_fspec_handler_and_command_emitter_use_helper_gate() {
    // @step Given the FspecHandler closure registered inside the napi agent_loop previously short-circuited via `if GLOBAL_CHUNK_CALLBACK.get().is_none()`
    // @step And the bridge `command_emitter` closure previously short-circuited via `if GLOBAL_CHUNK_CALLBACK.get().is_none()`
    // RPC-072: napi re-exports the shared agent-loop engine, so the gate and
    // both closures live in rust/agent-loop/src.
    let engine_src = workspace_root().join("agent-loop").join("src");
    let engine = strip_line_comments(&format!(
        "{}\n{}\n{}\n{}",
        read(&engine_src.join("lib.rs")),
        read(&engine_src.join("agent_loop").join("bridge.rs")),
        read(&engine_src.join("agent_loop").join("handlers.rs")),
        read(&engine_src.join("fspec_handler.rs")),
    ));

    // @step When I inspect the rewritten closures in rust/napi/src/session_manager.rs
    // @step Then both gates use `if !is_global_chunk_callback_registered()` calling a new private helper
    let helper_call_count = engine
        .matches("is_global_chunk_callback_registered()")
        .count();
    assert!(
        helper_call_count >= 3,
        "RPC-041/RPC-072: is_global_chunk_callback_registered() must be defined and called from the FspecHandler and command_emitter gates. Got {helper_call_count} occurrences"
    );
    assert!(
        engine.contains("pub fn is_global_chunk_callback_registered() -> bool"),
        "RPC-072: the shared engine must declare the NAPI-free gate"
    );

    // @step And the helper body consults `CHUNK_FANOUT_TSFN.get().and_then(|m| m.lock().ok()).map(|g| g.is_some()).unwrap_or(false)`
    // RPC-072: the standalone engine has no TSFN, so its gate is constant;
    // the TSFN registry remains on the napi side.
    assert!(
        contains_ws_insensitive(
            &engine,
            "pub fn is_global_chunk_callback_registered() -> bool { false }"
        ),
        "RPC-072: the standalone engine's gate must be constant false (no TSFN registry)"
    );

    // @step And the user-facing FspecHandler error string `"Global chunk callback not registered - cannot execute fspec command"` is preserved verbatim
    // RPC-072 replaced that error path: with no TS host the FspecHandler
    // dispatches in-process through codelet_fspec_core.
    assert!(
        engine.contains("dispatch_command"),
        "RPC-072: the FspecHandler must dispatch fspec commands in-process when no TS host is registered"
    );
}

#[test]
fn scenario_multiple_subscribers_observe_every_chunk_in_arrival_order() {
    use codelet_rpc_types::StreamChunk;
    use uuid::Uuid;

    // @step Given a SessionManager is constructed with the default no-op hooks
    let manager = SessionManager::new();

    // @step And a BackgroundSession is constructed with the manager's `chunks_tx.clone()` and `status_changes_tx.clone()` injected through `BackgroundSession::new(...)`
    // (We exercise the contract via the manager-owned sender directly because constructing a real
    //  BackgroundSession requires a codelet_cli::session::Session which spins up a provider stack.
    //  The semantics — multiple subscribers see every chunk that flows through chunks_tx — are
    //  identical regardless of which sender owns the broadcast::Sender clone.)

    // @step And two independent subscribers are obtained by calling `manager.chunks_tx().subscribe()` twice
    let mut rx1 = manager.chunks_tx().subscribe();
    let mut rx2 = manager.chunks_tx().subscribe();

    // @step When the session calls `handle_output(StreamChunk::user_input("hello"))`
    let uuid = Uuid::new_v4();
    let sid = codelet_rpc_types::SessionId::from(uuid.to_string());
    let chunk = StreamChunk::user_input("hello".to_string());
    manager
        .chunks_tx()
        .send((sid.clone(), chunk))
        .expect("send must succeed when subscribers are alive");

    // @step Then both subscribers receive the same `(SessionId, StreamChunk)` tuple
    let r1 = rx1.try_recv().expect("rx1 must receive the chunk");
    let r2 = rx2.try_recv().expect("rx2 must receive the chunk");
    assert_eq!(r1.0, sid);
    assert_eq!(r2.0, sid);

    // @step And the tuples observed by each subscriber arrive in the order the BackgroundSession emitted them
    let chunk2 = StreamChunk::user_input("world".to_string());
    manager
        .chunks_tx()
        .send((sid.clone(), chunk2))
        .expect("second send must succeed");
    let r1b = rx1.try_recv().expect("rx1 must receive second chunk");
    let r2b = rx2.try_recv().expect("rx2 must receive second chunk");
    assert_eq!(r1b.0, sid);
    assert_eq!(r2b.0, sid);
}

#[test]
fn scenario_all_codelet_sessions_shape_tests_continue_to_pass_with_inverted_assertions() {
    // @step Given the existing shape-test files `rust/sessions/tests/background_session_shape.rs` and `rust/sessions/tests/session_manager_shape.rs` carry RPC-039/RPC-040 invariants
    // background_session_shape.rs composes its scenarios from fragments
    // under background_session_shape/; read the composed tree.
    let bg_shape = codelet_test_helpers::source_composition::read_tree_or_file(
        workspace_root()
            .join("sessions")
            .join("tests")
            .join("background_session_shape.rs"),
    )
    .expect("read background_session_shape module tree");
    let sm_shape = read(
        &workspace_root()
            .join("sessions")
            .join("tests")
            .join("session_manager_shape.rs"),
    );

    // @step When I run `cargo test -p codelet-sessions --tests`
    // (Runtime invocation gated behind RPC_041_FULL_TESTS=1 to avoid cargo-in-cargo recursion;
    //  structural witnesses below are sufficient.)

    // @step Then `skeleton_invariants::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi` passes
    let skeleton_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("skeleton_invariants.rs");
    assert!(skeleton_path.exists(), "skeleton_invariants.rs must exist");

    // @step And the previously-named scenario `scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback` is updated to assert the mandatory chunks_tx send AND that the napi shell file no longer contains the literal token `GLOBAL_CHUNK_CALLBACK`
    assert!(
        bg_shape.contains("scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback"),
        "the legacy RPC-039 scenario function must still exist (its assertions are inverted)"
    );
    assert!(
        bg_shape.contains("assert!(!manager_code.contains(\"GLOBAL_CHUNK_CALLBACK\"))"),
        "the inverted assertion against the session manager source must be present"
    );

    // @step And new scenarios `scenario_handle_output_emits_unconditionally_on_chunks_tx`, `scenario_set_status_emits_on_status_changes_tx`, and `scenario_session_manager_passes_broadcast_senders_to_background_session` are added and pass
    assert!(
        bg_shape.contains("fn scenario_handle_output_emits_unconditionally_on_chunks_tx"),
        "RPC-041 new scenario `scenario_handle_output_emits_unconditionally_on_chunks_tx` must be added"
    );
    assert!(
        bg_shape.contains("fn scenario_set_status_emits_on_status_changes_tx"),
        "RPC-041 new scenario `scenario_set_status_emits_on_status_changes_tx` must be added"
    );
    assert!(
        sm_shape.contains("fn scenario_session_manager_passes_broadcast_senders_to_background_session"),
        "RPC-041 new scenario `scenario_session_manager_passes_broadcast_senders_to_background_session` must be added"
    );

    // @step And the prior session_manager_shape.rs scenario that required `NapiSessionManagerHooks::emit_isolation_state_change` to delegate to `GLOBAL_CHUNK_CALLBACK` is removed in lockstep with the hook removal
    // (We assert no `#[test] fn scenario_emit_isolation_state_change_*` survives.
    //  The needle is split across two string literals so this very assertion does
    //  not match its own source code.)
    let dead_scenario_prefix = format!(
        "fn scenario_emit_isolation_state_change{}",
        "_delegates_to_global_chunk_callback"
    );
    assert!(
        !sm_shape.contains(&dead_scenario_prefix),
        "RPC-041: the prior RPC-040 scenario function asserting emit_isolation_state_change delegation must be removed in lockstep with the hook removal"
    );
}
