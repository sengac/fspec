//! RPC-062: Runtime lifecycle tests for `codelet_tools::init_mcp_session`,
//! `codelet_tools::get_mcp_connections`, and `codelet_tools::cleanup_mcp_session`
//! — the three NAPI-free helpers that `codelet_sessions::SessionManager`
//! relies on for per-session MCP plumbing.
//!
//! Feature: spec/features/rpc-062-mcp-injection-lifecycle.feature
//!
//! These tests do NOT spin up a full `SessionManager` (that would
//! require real provider credentials). Instead they exercise the
//! process-global `MCP_SESSIONS` registry directly through the
//! public `codelet_tools` API surface that the extracted
//! `SessionManager` consumes at `session_manager.rs:600`,
//! `session_manager.rs:846`, and `session_manager.rs:935`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use codelet_tools::{cleanup_mcp_session, get_mcp_connections, init_mcp_session, McpInjection};
use uuid::Uuid;

/// Helper: drain any pending MCP injections so the test can prove the
/// receiver is alive without blocking. Returns immediately if nothing
/// is queued.
async fn no_pending_injection(rx: &mut tokio::sync::mpsc::Receiver<McpInjection>) -> bool {
    tokio::select! {
        biased;
        _ = tokio::time::sleep(Duration::from_millis(20)) => true,
        msg = rx.recv() => {
            // Drain whatever we got and report unexpected traffic.
            let _ = msg;
            false
        }
    }
}

// =============================================================================
// Scenario: init_mcp_session registers per-session state and
//           get_mcp_connections returns Some
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_init_registers_state_and_get_mcp_connections_returns_some() {
    // @step Given the codelet-tools crate is compiled and the process-global MCP_SESSIONS registry is in its initial state
    // (Compiled by the harness; per-uuid isolation is achieved by
    // generating a fresh Uuid below, so no global reset is required.)

    // @step And I generate a fresh uuid via uuid::Uuid::new_v4()
    let uuid = Uuid::new_v4();
    assert!(
        get_mcp_connections(uuid).is_none(),
        "fresh uuid must not appear in MCP_SESSIONS before init"
    );

    // @step When I call codelet_tools::init_mcp_session(uuid)
    let (mut rx, map) = init_mcp_session(uuid);

    // @step Then the returned tuple yields an mpsc::Receiver<McpInjection> and an McpConnectionMap
    assert!(
        no_pending_injection(&mut rx).await,
        "the freshly-created receiver must have no pending messages"
    );
    let map_len = map.read().await.len();
    assert_eq!(
        map_len, 0,
        "the freshly-created connection map must be empty"
    );

    // @step And codelet_tools::get_mcp_connections(uuid) returns Some(map)
    let observed =
        get_mcp_connections(uuid).expect("get_mcp_connections must return Some after init");
    assert_eq!(
        observed.read().await.len(),
        0,
        "the registry-observed connection map must match the one returned by init"
    );

    // Cleanup so we don't leak entries into MCP_SESSIONS between test runs.
    cleanup_mcp_session(uuid);
}

// =============================================================================
// Scenario: cleanup_mcp_session removes per-session state and
//           get_mcp_connections returns None
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_cleanup_removes_state_and_get_mcp_connections_returns_none() {
    // @step Given I have called codelet_tools::init_mcp_session(uuid) for a fresh uuid
    let uuid = Uuid::new_v4();
    let (_rx, _map) = init_mcp_session(uuid);

    // @step And codelet_tools::get_mcp_connections(uuid) currently returns Some
    assert!(
        get_mcp_connections(uuid).is_some(),
        "init_mcp_session must register the uuid before cleanup runs"
    );

    // @step When I call codelet_tools::cleanup_mcp_session(uuid)
    cleanup_mcp_session(uuid);

    // @step Then the cleanup call does not panic
    // (Reaching this line is proof — `cleanup_mcp_session` returns `()`.)

    // @step And codelet_tools::get_mcp_connections(uuid) returns None
    assert!(
        get_mcp_connections(uuid).is_none(),
        "cleanup_mcp_session must remove the uuid from MCP_SESSIONS"
    );
}

// =============================================================================
// Scenario: Idempotent re-init replaces the entry without leaking the
//           previous receiver
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_idempotent_reinit_replaces_entry() {
    // @step Given I have called codelet_tools::init_mcp_session(uuid) once and held onto its receiver
    let uuid = Uuid::new_v4();
    let (mut rx_first, _map_first) = init_mcp_session(uuid);
    assert!(
        get_mcp_connections(uuid).is_some(),
        "first init must register the uuid"
    );

    // @step When I call codelet_tools::init_mcp_session(uuid) a second time for the same uuid
    let (mut rx_second, _map_second) = init_mcp_session(uuid);

    // @step Then the second call returns a fresh mpsc::Receiver<McpInjection> distinct from the first
    //
    // Proof of distinctness: the second init_mcp_session call drops
    // the previous McpSessionState (HashMap::insert replaces the
    // existing entry). Dropping the previous state drops its
    // injection_tx, which closes the FIRST receiver. The SECOND
    // receiver, in contrast, is still open and starts empty. This is
    // exactly the singleton-per-session semantic the RPC-062
    // attachment requires (no leaked old connections).
    let first_after_replace = tokio::time::timeout(Duration::from_millis(100), rx_first.recv())
        .await
        .expect("rx_first.recv() must complete (channel was closed by re-init)");
    assert!(
        first_after_replace.is_none(),
        "the FIRST receiver must observe its channel as closed after idempotent re-init",
    );
    assert!(
        no_pending_injection(&mut rx_second).await,
        "the SECOND receiver must be open and start empty",
    );

    // @step And codelet_tools::get_mcp_connections(uuid) still returns Some after both calls
    assert!(
        get_mcp_connections(uuid).is_some(),
        "the registry must still hold an entry after idempotent re-init"
    );

    // @step And calling codelet_tools::cleanup_mcp_session(uuid) afterwards drops the entry and returns None
    cleanup_mcp_session(uuid);
    assert!(
        get_mcp_connections(uuid).is_none(),
        "cleanup after idempotent re-init must drop the entry"
    );
}

// =============================================================================
// Scenario: cleanup_mcp_session on an unknown uuid is a silent no-op
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_cleanup_on_unknown_uuid_is_silent_noop() {
    // @step Given I generate a fresh uuid that has never been registered via init_mcp_session
    let uuid = Uuid::new_v4();
    assert!(
        get_mcp_connections(uuid).is_none(),
        "fresh uuid must not be in MCP_SESSIONS before the test starts"
    );

    // @step When I call codelet_tools::cleanup_mcp_session(uuid) for the unknown uuid
    cleanup_mcp_session(uuid);

    // @step Then the call returns without panicking
    // (Reaching this line is proof — `cleanup_mcp_session` returns `()`.)

    // @step And codelet_tools::get_mcp_connections(uuid) returns None
    assert!(
        get_mcp_connections(uuid).is_none(),
        "cleanup_mcp_session on an unknown uuid must leave the registry empty for that uuid"
    );
}

// =============================================================================
// Scenario: MCP_SESSIONS registry isolates entries per session uuid
// =============================================================================
#[tokio::test(flavor = "multi_thread")]
async fn scenario_registry_isolates_entries_per_session_uuid() {
    // @step Given I have called codelet_tools::init_mcp_session(uuid_a) for a fresh uuid_a
    let uuid_a = Uuid::new_v4();
    let (_rx_a, _map_a) = init_mcp_session(uuid_a);

    // @step And I have called codelet_tools::init_mcp_session(uuid_b) for a separate fresh uuid_b
    let uuid_b = Uuid::new_v4();
    let (_rx_b, _map_b) = init_mcp_session(uuid_b);

    assert_ne!(
        uuid_a, uuid_b,
        "the two uuids must differ — Uuid::new_v4() collisions are astronomically unlikely"
    );
    assert!(
        get_mcp_connections(uuid_a).is_some(),
        "uuid_a must be registered before the cleanup step runs"
    );
    assert!(
        get_mcp_connections(uuid_b).is_some(),
        "uuid_b must be registered before the cleanup step runs"
    );

    // @step When I call codelet_tools::cleanup_mcp_session(uuid_a)
    cleanup_mcp_session(uuid_a);

    // @step Then codelet_tools::get_mcp_connections(uuid_a) returns None
    assert!(
        get_mcp_connections(uuid_a).is_none(),
        "cleanup_mcp_session(uuid_a) must drop only uuid_a's entry"
    );

    // @step And codelet_tools::get_mcp_connections(uuid_b) still returns Some
    assert!(
        get_mcp_connections(uuid_b).is_some(),
        "cleanup_mcp_session(uuid_a) must NOT affect uuid_b's entry"
    );

    // Final cleanup so the test is hygienic.
    cleanup_mcp_session(uuid_b);
}
