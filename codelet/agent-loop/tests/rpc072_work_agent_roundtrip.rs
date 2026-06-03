//! Feature: spec/features/rpc072-work-agent-roundtrip.feature
//!
//! Scenarios covered (1 feature file = 1 test file per ACDD rule):
//!
//!   1. "Stub-provider session emits Text+Done in response to typed
//!      input" — end-to-end behavioural test that drives a real
//!      [`codelet_sessions::SessionManager`] configured with the
//!      RPC-072 [`FspecAgentHooks`] impl and the deterministic stub
//!      LlmProvider, sends typed input, and asserts that:
//!      a. A `StreamChunk::Text` with text "hi back" arrives on the
//!      manager's `chunks_tx` broadcast within 5 seconds.
//!      b. A `StreamChunk::Done` arrives after the Text chunk.
//!      c. The session's status returns to `SessionStatus::Idle`
//!      after the Done chunk.
//!
//!   2. "codelet-agent-loop crate has zero dependency on codelet-napi" —
//!      dependency-rule regression that walks `cargo metadata` for the
//!      `codelet-agent-loop` package and asserts the transitive
//!      package set contains no `codelet-napi`, plus a source-tree
//!      scan asserting no `.rs` file under `agent-loop/src/`
//!      references `codelet_napi`.
//!
//! Before RPC-072 the round-trip scenario was covered only by the
//! `#[ignore]`'d `scenario_send_input_hello_yields_canned_stream` in
//! `codelet/fspec/tests/cross_frontend_parity.rs`. After RPC-072 lands,
//! the tests below are the primary regression net for the work-agent
//! round-trip; the cross-frontend test stays at the binary boundary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(feature = "test-support")]
use std::sync::Arc;
#[cfg(feature = "test-support")]
use std::time::Duration;

#[cfg(feature = "test-support")]
use codelet_agent_loop::FspecAgentHooks;
#[cfg(feature = "test-support")]
use codelet_rpc_types::{SessionStatus, StreamChunk};
#[cfg(feature = "test-support")]
use codelet_sessions::session_manager::SessionManager;
use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};
#[cfg(feature = "test-support")]
use uuid::Uuid;

// ===========================================================================
// Scenario: Stub-provider session emits Text+Done in response to typed input
// ===========================================================================
//
// This scenario requires the `test-support` feature so the agent loop's
// `Custom("stub")` provider-dispatch branch compiles in. Without that
// feature, the branch returns `AgentLoopError::ProviderUnavailable` and
// no Text chunk arrives. Run with:
//   cargo test -p codelet-agent-loop --features test-support
//   cargo test -p codelet-fspec  --features test-stub-provider
// The cfg-gate below keeps default `cargo test --workspace` green —
// the source-shape + boundary scenarios still run unconditionally.

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stub_provider_input_to_reply() {
    // @step Given the codelet-fspec binary is built with the test-stub-provider feature
    // (Asserted by `[features] test-support` in codelet/agent-loop/Cargo.toml
    //  dev-dependencies, which enables codelet-providers/test-support.)

    // RPC-025: ProviderManager's model-registry construction calls into
    // codelet_common::get_data_dir(), which panics if the global data
    // dir is not initialised. Point it at a tempdir so the round-trip
    // test is hermetic.
    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());

    // @step And build_service has been invoked, installing FspecAgentHooks on the SessionManager
    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    // @step And the stub LlmProvider is registered under the slug "stub"
    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    // Subscribe to the manager-owned chunks broadcast BEFORE creating
    // the session so we don't miss any chunks.
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let mut status_rx = manager.status_changes_tx().subscribe();

    // @step And a fresh BackgroundSession has been created with model "stub/canned"
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_uuid = Uuid::new_v4();
    let session_id_str = session_uuid.to_string();
    manager
        .create_session_with_id(&session_id_str, "stub/canned", &project, "stub-session")
        .await
        .expect("create_session_with_id");

    let session = manager
        .get_session(&session_id_str)
        .expect("session must exist after create_session_with_id");

    // @step When the test calls session.send_input("hello", None)
    session
        .send_input("hello".to_string(), None)
        .expect("send_input must succeed once FspecAgentHooks is installed");

    // @step Then within 5 seconds the chunks_tx broadcast yields a StreamChunk::Text with text "hi back"
    // @step And a StreamChunk::Done arrives after the Text chunk
    let mut got_text = false;
    let mut got_done = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let recv = tokio::time::timeout(Duration::from_millis(250), chunks_rx.recv()).await;
        match recv {
            Ok(Ok((sid, chunk))) => {
                if sid.value != session_id_str {
                    continue;
                }
                match chunk {
                    StreamChunk::Text { text, .. } => {
                        if text == "hi back" {
                            got_text = true;
                        }
                    }
                    StreamChunk::Done => {
                        got_done = true;
                        break;
                    }
                    _ => continue,
                }
            }
            Ok(Err(_)) | Err(_) => continue,
        }
    }
    assert!(
        got_text,
        "expected StreamChunk::Text {{ text: \"hi back\" }} within 5s; got_text={got_text}",
    );
    assert!(got_done, "expected StreamChunk::Done after the Text chunk");

    // @step And the session status returns to SessionStatus::Idle
    let mut saw_idle = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let recv = tokio::time::timeout(Duration::from_millis(250), status_rx.recv()).await;
        match recv {
            Ok(Ok((sid, status))) => {
                if sid.value == session_id_str && status == SessionStatus::Idle {
                    saw_idle = true;
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => continue,
        }
    }
    assert!(
        saw_idle,
        "expected SessionStatus::Idle to broadcast within 2s of the Done chunk",
    );
}

// ===========================================================================
// Scenario: codelet-agent-loop crate has zero dependency on codelet-napi
// ===========================================================================

#[test]
fn no_codelet_napi_in_transitive_dependency_graph() {
    // @step Given the codelet-agent-loop crate exists under codelet/agent-loop/
    // @step When cargo metadata is invoked for the codelet-agent-loop package
    // @step Then the resulting transitive package set does not contain "codelet-napi"
    assert_no_transitive_dependency!("codelet-agent-loop", "codelet-napi");
}

#[test]
fn no_codelet_napi_import_in_source() {
    // @step And no .rs file under codelet/agent-loop/src/ contains the substring "codelet_napi"
    assert_no_import_in_sources!("agent-loop", "codelet_napi");
}

// ===========================================================================
// Scenario: codelet-fspec still has zero dependency on codelet-napi after RPC-072
// ===========================================================================
//
// The headline forbidden-arrow rule for the `fspec` binary is enforced
// independently in `codelet/fspec/tests/no_napi_dependency.rs`. We re-assert
// it here so a regression in `codelet-agent-loop`'s dep graph (which is
// reached transitively from `codelet-fspec`) is visible at this card's
// dedicated test target.

#[test]
fn no_codelet_napi_in_codelet_fspec_transitive_graph_after_rpc072() {
    // @step Given the codelet-fspec build_service now installs FspecAgentHooks from codelet-agent-loop
    // @step When cargo metadata is invoked for the codelet-fspec package
    // @step Then the resulting transitive package set does not contain "codelet-napi"
    assert_no_transitive_dependency!("codelet-fspec", "codelet-napi");
}

// ===========================================================================
// Scenario: build_service installs FspecAgentHooks instead of FspecSessionManagerHooks
// ===========================================================================
//
// Source-shape regression: pin the call sites in `codelet/fspec/src/common.rs`
// that wire `FspecAgentHooks` into the SessionManager. A sabotage (e.g.
// re-introducing the no-op `FspecSessionManagerHooks` or dropping the
// `set_hooks(...)` call) fails this test loudly.

#[test]
fn build_service_installs_fspec_agent_hooks() {
    use std::path::PathBuf;

    // @step Given the codelet-fspec build_service source after RPC-072 lands
    let common_rs = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("agent-loop crate has parent workspace dir")
        .join("fspec")
        .join("src")
        .join("common.rs");

    // @step When the source is inspected
    let src = std::fs::read_to_string(&common_rs).unwrap_or_else(|e| {
        panic!(
            "must be able to read codelet/fspec/src/common.rs at {}: {e}",
            common_rs.display()
        )
    });

    // @step Then it imports codelet_agent_loop::FspecAgentHooks
    assert!(
        src.contains("use codelet_agent_loop::FspecAgentHooks")
            || src.contains("codelet_agent_loop::FspecAgentHooks"),
        "common.rs must reference codelet_agent_loop::FspecAgentHooks after RPC-072"
    );

    // @step And it constructs FspecAgentHooks::new and passes it to manager.set_hooks
    assert!(
        src.contains("FspecAgentHooks::new()"),
        "common.rs must construct FspecAgentHooks::new() after RPC-072"
    );
    assert!(
        src.contains("manager.set_hooks") && src.contains("FspecAgentHooks"),
        "common.rs must pass FspecAgentHooks into manager.set_hooks(...) after RPC-072"
    );

    // @step And the prior FspecSessionManagerHooks installation has been replaced or wrapped
    // The simplest guarantee: the literal call site
    // `set_hooks(Arc::new(FspecSessionManagerHooks)` MUST be gone. Comment
    // references to the historical impl are fine (they explain WHY we
    // replaced it); the call site itself is the regression.
    let bs_start = src
        .find("pub fn build_service")
        .expect("build_service function must exist");
    let bs_end = src[bs_start..]
        .find("\n}\n")
        .map(|i| bs_start + i)
        .unwrap_or(src.len());
    let body = &src[bs_start..bs_end];
    let stripped = codelet_test_helpers::dependency_rules::strip_rust_comments(body);
    assert!(
        !stripped.contains("FspecSessionManagerHooks"),
        "build_service body (comments stripped) MUST NOT install FspecSessionManagerHooks \
         after RPC-072. Replace with FspecAgentHooks."
    );
}
