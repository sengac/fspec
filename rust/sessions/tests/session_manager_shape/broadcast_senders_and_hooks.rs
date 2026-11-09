//! Feature: spec/features/replace-global-chunk-callback-napi-shell.feature
//!
//! Moved out of codelet_sessions_tests_assert.rs (COV-075: one test file per feature).

use super::*;

#[test]
fn scenario_session_manager_passes_broadcast_senders_to_background_session() {
    // @step Given the SessionManager code lives at rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());

    // @step And SessionManager owns `chunks_tx`, `logs_tx`, and `status_changes_tx` broadcast::Sender fields
    let struct_start = sm
        .find("pub struct SessionManager {")
        .expect("SessionManager struct must exist");
    let struct_body = &sm[struct_start..];
    let struct_end = struct_body.find("\n}\n").expect("struct must terminate");
    let fields_block = &struct_body[..struct_end];
    for field in ["chunks_tx:", "logs_tx:", "status_changes_tx:"] {
        assert!(
            fields_block.contains(field),
            "SessionManager must declare the `{field}` broadcast sender field"
        );
    }

    // @step When I inspect the two `BackgroundSession::new(...)` call sites in `create_session_with_id` and `create_isolated_session_with_id`
    let sm_code = strip_line_comments(&sm);
    // RPC-425: both creation paths now build the session through the shared
    // `create_background_session_inner` helper, which owns the single
    // `BackgroundSession::new(...)` call and receives the manager's senders.
    let count = sm_code.matches("create_background_session_inner(").count();
    assert!(
        count >= 2,
        "RPC-041/RPC-425: create_session_with_id and create_isolated_session_with_id must build sessions via create_background_session_inner. Got {count}"
    );
    let helper = strip_line_comments(&read(
        &workspace_root()
            .join("sessions")
            .join("src")
            .join("session_creation_helper.rs"),
    ));
    let new_call = helper
        .find("BackgroundSession::new(")
        .expect("session_creation_helper.rs must construct the BackgroundSession");
    let new_args = &helper[new_call
        ..helper[new_call..]
            .find(");")
            .map_or(helper.len(), |e| new_call + e)];
    assert!(
        new_args.contains("chunks_tx") && new_args.contains("status_changes_tx"),
        "RPC-041: BackgroundSession::new(...) must receive the manager's chunks_tx and status_changes_tx"
    );

    // @step Then both call sites pass `self.chunks_tx.clone()` and `self.status_changes_tx.clone()` as the last two arguments
    let chunks_clone_count = sm_code.matches("self.chunks_tx.clone()").count();
    let status_clone_count = sm_code.matches("self.status_changes_tx.clone()").count();
    assert!(
        chunks_clone_count >= 2,
        "RPC-041: both BackgroundSession::new(...) call sites must pass self.chunks_tx.clone(). Got {chunks_clone_count} occurrences"
    );
    assert!(
        status_clone_count >= 2,
        "RPC-041: both BackgroundSession::new(...) call sites must pass self.status_changes_tx.clone(). Got {status_clone_count} occurrences"
    );

    // @step And the `self.hooks().emit_isolation_state_change(...)` call inside `create_session_with_id` is removed
    // @step And the `self.hooks().emit_isolation_state_change(...)` call inside `create_isolated_session_with_id` is removed
    assert!(
        !sm_code.contains("emit_isolation_state_change"),
        "RPC-041: the emit_isolation_state_change hook calls inside SessionManager methods must be removed"
    );

    // @step And each of those two former hook-call sites is replaced by `let _ = self.chunks_tx.send((codelet_rpc_types::SessionId::from(id.to_string()), codelet_rpc_types::StreamChunk::isolation_state_change(...)))`
    assert!(
        sm_code.contains("StreamChunk::isolation_state_change(")
            || sm_code.contains("isolation_state_change("),
        "RPC-041: SessionManager must emit IsolationStateChange chunks via `self.chunks_tx.send(...)`"
    );
}

#[test]
fn scenario_session_manager_hooks_trait_no_longer_carries_emit_isolation_state_change() {
    // @step Given the SessionManagerHooks trait is defined in rust/sessions/src/session_manager.rs
    let sm = read(&moved_sm_path());
    let sm_code = strip_line_comments(&sm);

    // @step When I inspect the trait declaration
    let trait_start = sm
        .find("pub trait SessionManagerHooks")
        .expect("SessionManagerHooks trait must exist");
    let trait_body_open = sm[trait_start..]
        .find('{')
        .expect("trait body start must exist");
    let absolute_body_start = trait_start + trait_body_open;
    let bytes = sm.as_bytes();
    let mut depth = 0i32;
    let mut end = absolute_body_start;
    for (i, b) in bytes.iter().enumerate().skip(absolute_body_start) {
        if *b == b'{' {
            depth += 1;
        } else if *b == b'}' {
            depth -= 1;
            if depth == 0 {
                end = i + 1;
                break;
            }
        }
    }
    let trait_block = &sm[absolute_body_start..end];
    let trait_block_code = strip_line_comments(trait_block);

    // @step Then the trait declares exactly six methods: `spawn_agent_loop`, `spawn_scheduler`, `ensure_scheduler_running_for_loop`, `spawn_footer_poller`, `stop_footer_poller`, `cleanup_session_loops`
    for method in [
        "fn spawn_agent_loop",
        "fn spawn_scheduler",
        "fn ensure_scheduler_running_for_loop",
        "fn spawn_footer_poller",
        "fn stop_footer_poller",
        "fn cleanup_session_loops",
    ] {
        assert!(
            trait_block_code.contains(method),
            "RPC-041: SessionManagerHooks trait must declare `{method}(...)`"
        );
    }

    // @step And the trait does NOT declare a method named `emit_isolation_state_change`
    assert!(
        !trait_block_code.contains("fn emit_isolation_state_change"),
        "RPC-041: SessionManagerHooks trait must NOT declare emit_isolation_state_change"
    );

    // @step And `NoopSessionManagerHooks` provides no-op impls for the same six methods only
    let noop_impl_marker = "impl SessionManagerHooks for NoopSessionManagerHooks";
    assert!(
        sm.contains(noop_impl_marker),
        "NoopSessionManagerHooks impl must exist"
    );
    assert!(
        !sm_code.contains("fn emit_isolation_state_change"),
        "RPC-041: NoopSessionManagerHooks impl must NOT define emit_isolation_state_change"
    );

    // @step And the napi-side `NapiSessionManagerHooks` in rust/napi/src/session_manager.rs implements the same six methods only and no longer references `GLOBAL_CHUNK_CALLBACK`
    let hooks = strip_line_comments(&read(
        &workspace_root()
            .join("agent-loop")
            .join("src")
            .join("hooks.rs"),
    ));
    assert!(
        hooks.contains("impl SessionManagerHooks for FspecAgentHooks"),
        "the agent-loop FspecAgentHooks must implement SessionManagerHooks"
    );
    assert!(
        !hooks.contains("fn emit_isolation_state_change") && !hooks.contains("GLOBAL_CHUNK_CALLBACK"),
        "RPC-041: FspecAgentHooks must not implement emit_isolation_state_change or reference GLOBAL_CHUNK_CALLBACK"
    );
}
