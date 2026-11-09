//! Feature: spec/features/exec-stdin-prompt.feature
//!
//! TOOL-022 P2 — backend (codelet-sessions) acceptance tests:
//! - Backend round-trip surfaces the request only while a live exec
//!   session is quiet
//! - write_exec_stdin appends a newline to the typed text
//! - write_exec_stdin on an unknown exec session returns a clean error
//! - The exec-stdin overlay does not flip the agent session status
//!
//! Harness notes:
//! - One process-wide data dir (lazily created at first use) — the
//!   persistence `SessionStore` caches its sessions_dir at first touch,
//!   so swapping `set_data_directory` per test would leave the cached
//!   store pointing at a deleted tempdir (ENOENT on manifest writes).
//! - `ANTHROPIC_API_KEY` is set to a fake value: `create_session`
//!   constructs a real rig agent (provider "claude" → env-credential
//!   path) and declines with an empty id when the env key is missing.
//! - `#[serial]` on every scenario: they mutate `ANTHROPIC_API_KEY`
//!   and share the tools `global_store` / detector cooldown map.
//!
//! The live-exec-session scenarios use a REAL child process in the
//! tools `global_store` (the same shape P1 uses), so `write_exec_stdin`
//! reaches the real `stdin_tx` clone path.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod exec_stdin_prompt_p2_support;

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::{
    ExecStdinRequest as InternalExecStdinRequest, UnifiedExecArgs, UnifiedExecResult,
};
use exec_stdin_prompt_p2_support::{
    close_exec, data_dir, fresh_session, run_still_running, wait_for_stored_request,
    wait_store_state,
};
use rig::tool::Tool;
use serial_test::serial;
use uuid::Uuid;

/// Scenario: Backend round-trip surfaces the request only while a live exec session is quiet
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_backend_round_trip_surfaces_request_only_while_live_exec_is_quiet() {
    // @step Given a live unified_exec session "exec-live" has been quiet for 3 seconds while running
    let _ = data_dir();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");

    // Register the agent-session callback the way agent_loop.rs does:
    // detector fires → store the request on the BackgroundSession.
    let session = manager
        .get_session(&agent.to_string())
        .expect("session must exist");
    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(std::sync::Arc::new(
            move |request: Option<InternalExecStdinRequest>| {
                if let Err(error) = session_for_cb.set_exec_stdin_request(request) {
                    tracing::error!(%error, "cannot record the exec-stdin request");
                }
            },
        )),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;

    // @step When the agent session detector fires for that exec session
    // (quiet >= 3s → the 2s-cadence detector emits on its own cadence;
    // TEST-042: the test waits on the stored request, not a fixed time)
    wait_for_stored_request(&session).await;

    // @step Then the agent session stores an exec-stdin request for "exec-live" with its command display and quiet seconds
    let stored = session.get_exec_stdin_request().ok().flatten();
    assert!(
        stored.is_some(),
        "detector must have stored a request on the agent session after quiet >= 3s"
    );
    let stored = stored.expect("just asserted Some");
    assert_eq!(stored.exec_session_id, exec_live);
    assert!(
        stored.quiet_seconds >= 3,
        "quiet_seconds must reflect the quiet window, got {}",
        stored.quiet_seconds
    );
    assert!(!stored.command.is_empty(), "command display must be stored");

    // @step When the TUI probes the agent session for its exec-stdin request
    let handle: &dyn SessionManagerHandle = &*manager;
    let wire = handle
        .get_exec_stdin_request(&sid)
        .expect("get_exec_stdin_request reads the session");

    // @step Then the TUI receives the stored request
    let wire = wire.expect("get_exec_stdin_request must surface the stored request");
    assert_eq!(wire.exec_session_id, exec_live);
    assert_eq!(wire.command, stored.command);
    assert_eq!(wire.quiet_seconds, stored.quiet_seconds as i64);

    // @step When the exec session exits
    close_exec(&tool, &exec_live).await;
    wait_store_state(&exec_live, false).await;

    // @step Then the agent session has no stored exec-stdin request
    // (the getter clears the slot for the dead exec session; the TUI's
    // next probe returns None)
    assert!(
        handle
            .get_exec_stdin_request(&sid)
            .expect("get_exec_stdin_request reads the session")
            .is_none(),
        "after the exec session exits, the probe must return None"
    );
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: write_exec_stdin appends a newline to the typed text
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_write_exec_stdin_appends_newline_to_typed_text() {
    // @step Given a live unified_exec session "exec-abc" has been quiet while running
    // (cat prints each stdin line with a trailing newline → the output
    // buffer carries the exact bytes the child received)
    let _ = data_dir();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = fresh_session(&manager);

    let (tool, exec_id) = run_still_running(Uuid::new_v4(), "cat").await;

    // @step When the backend writes "yes" to exec session "exec-abc" stdin
    let result = handle.write_exec_stdin(&sid, &exec_id, "yes");

    // @step Then the exec session receives exactly "yes" plus a newline on its stdin
    assert!(
        result.is_ok(),
        "write_exec_stdin must succeed, got: {:?}",
        result.err()
    );
    // Poll until the child echoes the bytes back (the reader task drains
    // into the output buffer).
    // TEST-042: no sleep between polls; each poll waits on the exec session
    // itself (yield_time_ms), under the deadlock guard.
    let mut output = String::new();
    let echoed = codelet_test_helpers::deadlock_guard("the child echoes yes", async {
        loop {
            let poll: UnifiedExecResult = tool
                .call(UnifiedExecArgs(serde_json::json!({
                    "action": "poll",
                    "session_id": exec_id,
                    "yield_time_ms": 50
                })))
                .await
                .expect("poll must succeed");
            output = poll.output.unwrap_or_default();
            if output.contains("yes\n") {
                break;
            }
        }
    })
    .await;
    assert!(echoed.is_ok(), "{echoed:?}");
    assert!(
        output.contains("yes\n"),
        "child must have received exactly \"yes\\n\" (echoed back); got: {output:?}"
    );
    assert!(
        !output.contains("yes\nyes"),
        "newline must be appended at most once; got: {output:?}"
    );

    close_exec(&tool, &exec_id).await;
}

/// Scenario: write_exec_stdin on an unknown exec session returns a clean error
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_write_exec_stdin_unknown_session_returns_clean_error() {
    // @step Given an agent session exists
    let _ = data_dir();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = &*manager;
    let sid = fresh_session(&manager);

    // @step When the backend writes "x" to exec session "nope" stdin
    let result = handle.write_exec_stdin(&sid, "nope", "x");

    // @step Then the backend returns an error naming the unknown exec session
    let err = result.expect_err("write to an unknown exec session must fail");
    assert!(
        err.contains("nope"),
        "error must name the unknown exec session id; got: {err}"
    );

    // @step And the error does not contain the reaper race exit code noise
    assert!(
        !err.contains("-1"),
        "error must not carry reaper-race exit-code noise; got: {err}"
    );
}

/// Scenario: The exec-stdin overlay does not flip the agent session status
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_exec_stdin_overlay_does_not_flip_agent_session_status() {
    // @step Given a live unified_exec session "exec-live" has been quiet for 3 seconds while running
    let _ = data_dir();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager
        .get_session(&agent.to_string())
        .expect("session exists");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(std::sync::Arc::new(
            move |request: Option<InternalExecStdinRequest>| {
                if let Err(error) = session_for_cb.set_exec_stdin_request(request) {
                    tracing::error!(%error, "cannot record the exec-stdin request");
                }
            },
        )),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;

    // @step When the agent session detector fires for that exec session
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "detector must have fired and stored the request"
    );

    // @step Then the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "the exec-stdin overlay must NOT flip the status"
    );

    // @step And no Paused chunk was emitted for the agent session
    let chunks = session.get_buffered_output(usize::MAX);
    assert!(
        !chunks.iter().any(|c| {
            matches!(
                c,
                codelet_rpc_types::StreamChunk::SessionStateChange {
                    state: codelet_rpc_types::SessionState::Paused
                }
            )
        }),
        "no SessionStateChange{{Paused}} chunk may be emitted for the exec-stdin overlay"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}
