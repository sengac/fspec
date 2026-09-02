//! Feature: spec/features/exec-stdin-push-emission.feature
//!
//! BUG-171 — the exec-stdin overlay must surface via a PUSH StreamChunk,
//! not via a pull probe. The TUI only probes on focus switch and on a
//! Paused state change; exec-stdin deliberately performs NO status flip,
//! so while a session stays Running the overlay was dead-on-arrival.
//!
//! These scenarios pin the push contract on the sessions side:
//! - storing a request emits a `StreamChunk::ExecStdinRequest` on the
//!   session chunk stream (no status flip);
//! - clearing the slot (child exit alive-check, successful
//!   `write_exec_stdin`) emits `StreamChunk::ExecStdinRequestCleared`.
//!
//! Harness notes: mirror rust/sessions/tests/exec_stdin_prompt_p2.rs —
//! one process-wide data dir, fake ANTHROPIC_API_KEY, `#[serial]` on
//! every scenario (shared tools `global_store` + detector cooldown map).
//! The chunk stream is observed via `manager.chunks_tx().subscribe()`
//! (the same broadcast the TUI's chunks subscriber consumes).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, SessionState, SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::{
    ExecStdinRequest as InternalExecStdinRequest, UnifiedExecArgs, UnifiedExecResult,
    UnifiedExecTool,
};
use rig::tool::Tool;
use serial_test::serial;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Process-wide data dir (one for the whole test binary — the
/// persistence `SessionStore` caches its sessions_dir at first touch,
/// so per-test `set_data_directory` swaps would leave the cached store
/// pointing at a deleted tempdir).
static DATA_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn data_dir() -> &'static std::path::PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("bug171-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create bug171 data dir");
        let _ = codelet_common::set_data_directory(dir.clone());
        dir
    })
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
fn fresh_session(manager: &SessionManager) -> SessionId {
    std::env::set_var("ANTHROPIC_API_KEY", "bug171-fake-key");
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    let sid = handle.create_session(None);
    assert!(
        !sid.value.is_empty(),
        "create_session must succeed (fake env key + default model set)"
    );
    sid
}

/// Run a still-running command through the unified_exec tool, owned by
/// the given agent session (so the detector's emit lands on that
/// session's registered callback).
async fn run_still_running(agent: Uuid, command: &str) -> (UnifiedExecTool, String) {
    let tool = UnifiedExecTool::new(agent);
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "run",
            "command": command,
            "yield_time_ms": 50
        })))
        .await
        .expect("run action must succeed");
    let session_id = result
        .session_id
        .expect("still-running command must carry a session_id");
    (tool, session_id)
}

async fn close_exec(tool: &UnifiedExecTool, session_id: &str) {
    let _ = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "close",
            "session_id": session_id
        })))
        .await;
}

/// Collect every chunk delivered on the manager's chunk broadcast since
/// `rx` was created.
fn drained_chunks(rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();
    while let Ok((_, chunk)) = rx.try_recv() {
        chunks.push(chunk);
    }
    chunks
}

/// Scenario: Detector fire while session stays Running pushes an exec-stdin request chunk
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_fire_while_session_stays_running_pushes_an_exec_stdin_request_chunk() {
    // @step Given the agent session is Running and focused with no HITL prompt in the slot
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);
    assert!(session.get_hitl_request().is_none(), "no HITL prompt may pre-exist");

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: InternalExecStdinRequest| {
            session_for_cb.set_exec_stdin_request(Some(request));
        })),
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();
    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;

    // @step When the exec-stdin quiet detector fires for a live exec session and the agent session callback stores the request on the BackgroundSession
    // (quiet >= 3s → the 2s-cadence detector emits within ~4s)
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "detector must have fired and stored the request"
    );

    // @step Then an exec-stdin request StreamChunk with that request is pushed on the session chunk stream
    let chunks = drained_chunks(&mut chunks_rx);
    let request_chunk = chunks.iter().find_map(|c| match c {
        StreamChunk::ExecStdinRequest { request } => Some(request.clone()),
        _ => None,
    });
    assert!(
        request_chunk.is_some(),
        "a StreamChunk::ExecStdinRequest must ride the chunk stream; got: {chunks:?}"
    );
    let pushed = request_chunk.expect("just asserted Some");
    assert_eq!(pushed.exec_session_id, exec_live);
    assert_eq!(pushed.command, "sh -c 'sleep 45'");

    // @step And the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "the push chunk must not flip the status"
    );
    assert!(
        !chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::SessionStateChange { state: SessionState::Paused })),
        "no Paused state change chunk may accompany the exec-stdin request chunk"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Clearing the exec-stdin slot pushes an exec-stdin cleared chunk
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_clearing_the_exec_stdin_slot_pushes_an_exec_stdin_cleared_chunk() {
    // @step Given a stored exec-stdin request on a Running agent session
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: InternalExecStdinRequest| {
            session_for_cb.set_exec_stdin_request(Some(request));
        })),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored before the clear"
    );

    // @step When the exec-stdin slot transitions from Some to None
    let mut chunks_rx = manager.chunks_tx().subscribe();
    session.set_exec_stdin_request(None);

    // @step Then an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    tokio::time::sleep(Duration::from_millis(200)).await;
    let chunks = drained_chunks(&mut chunks_rx);
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "a StreamChunk::ExecStdinRequestCleared must ride the chunk stream; got: {chunks:?}"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Exec session child exit clears the stored request without a status flip
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_exec_session_child_exit_clears_the_stored_request_without_a_status_flip() {
    // @step Given a stored exec-stdin request on a Running agent session
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: InternalExecStdinRequest| {
            session_for_cb.set_exec_stdin_request(Some(request));
        })),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored before the child exits"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the underlying exec session child exits and the alive check runs
    close_exec(&tool, &exec_live).await;
    // Wait for the reaper to drop the store entry so the alive check
    // can observe the exit.
    for _ in 0..50 {
        if !codelet_tools::unified_exec::global_store()
            .contains(&exec_live)
            .await
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let handle: &dyn SessionManagerHandle = &*manager;
    let wire = handle.get_exec_stdin_request(&sid);
    assert!(
        wire.is_none(),
        "the alive check must clear the stale stored request"
    );

    // @step Then the stored request is cleared
    assert!(
        session.get_exec_stdin_request().is_none(),
        "the stored request must be gone after the child exits"
    );

    // @step And the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "child exit must not flip the agent session status"
    );

    // The cleared chunk must have ridden the chunk stream.
    let chunks = drained_chunks(&mut chunks_rx);
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "a cleared chunk must accompany the alive-check clear; got: {chunks:?}"
    );

    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Successful write_exec_stdin pushes a cleared chunk so the overlay unmounts
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_successful_write_exec_stdin_pushes_a_cleared_chunk() {
    // @step Given the exec-stdin composer overlay is visible for a live exec session
    // (backend analog: a live exec session with a stored exec-stdin request
    //  on a Running agent session)
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: InternalExecStdinRequest| {
            session_for_cb.set_exec_stdin_request(Some(request));
        })),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored (overlay visible) before the write"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the user presses Enter and the backend write to the exec session stdin succeeds
    let handle: &dyn SessionManagerHandle = &*manager;
    let write = handle.write_exec_stdin(&sid, &exec_live, "y");
    assert!(
        write.is_ok(),
        "write_exec_stdin must succeed, got: {:?}",
        write.err()
    );

    // @step Then an exec-stdin cleared StreamChunk is pushed
    tokio::time::sleep(Duration::from_millis(200)).await;
    let chunks = drained_chunks(&mut chunks_rx);
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "a cleared chunk must accompany the post-write clear; got: {chunks:?}"
    );

    // @step And the overlay is gone on the next frame
    // (backend analog: the stored request is cleared so the next probe
    //  / the TUI's next frame sees no pending request)
    assert!(
        session.get_exec_stdin_request().is_none(),
        "the stored request must be cleared after a successful write"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

