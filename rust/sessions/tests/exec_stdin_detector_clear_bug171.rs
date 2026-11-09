//! Feature: spec/features/exec-stdin-detector-clear.feature
//!
//! BUG-171 rule [4] — once the exec-stdin detector has fired it observes
//! the END of the prompt condition (output resumes, child exits, exec
//! session leaves the store) and pushes a clear through the same
//! callback. Split from exec_stdin_push_chunk_bug171.rs (REFAC-060).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::ExecStdinRequest as InternalExecStdinRequest;
use serial_test::serial;
use uuid::Uuid;

mod exec_stdin_detector_bug171_support;
mod exec_stdin_push_chunk_bug171_support;
use exec_stdin_detector_bug171_support::{wait_for_chunks, wait_for_stored_request};
use exec_stdin_push_chunk_bug171_support::{
    close_exec, data_dir, fresh_session, run_still_running,
};

/// Scenario: Detector clear on output resumption pushes an exec-stdin cleared chunk
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_output_resumption_pushes_an_exec_stdin_cleared_chunk() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
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
        .expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(
            move |request: Option<InternalExecStdinRequest>| {
                if let Err(error) = session_for_cb.set_exec_stdin_request(request) {
                    tracing::error!(%error, "cannot record the exec-stdin request");
                }
            },
        )),
    );

    // Prints once at t≈0, then goes quiet (detector fires at ~4s), then
    // produces output again at t≈10.
    let (_tool, _exec_live) =
        run_still_running(agent, "sh -c 'printf out\\n; sleep 10; printf resume\\n'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the request must be stored before the output resumes"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the command produces output again so the session is no longer quiet
    // (the 'resume' line lands at t≈10; the 2s-cadence detector observes
    //  quiet < 3s on its next tick)

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    // (worst case: output at t≈10, detector tick + clear + reaper margin)
    let chunks = wait_for_chunks(&mut chunks_rx, &|chunks| {
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared))
    })
    .await;
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "the detector must clear the stored request when output resumes; chunks: {chunks:?}"
    );

    // @step And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    // (the previous assertion IS the chunk-stream observation)
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(None)),
        "the stored request must be gone after the non-exit clear"
    );

    // @step And the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "the detector clear must not flip the status"
    );

    // BUG-415: the command has exited and its exec session was reaped; nothing to close.
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Detector clear on child exit emits a clear and the stored request is gone
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_child_exit_emits_a_clear_and_the_stored_request_is_gone() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
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
        .expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(
            move |request: Option<InternalExecStdinRequest>| {
                if let Err(error) = session_for_cb.set_exec_stdin_request(request) {
                    tracing::error!(%error, "cannot record the exec-stdin request");
                }
            },
        )),
    );

    let (_tool, _exec_live) = run_still_running(agent, "sh -c 'printf out\\n; sleep 10'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the request must be stored before the child exits"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the child exits and the reaper removes the exec session from the store
    // (the child exits at t≈10 on its own; the reaper drops the store entry)

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    let chunks = wait_for_chunks(&mut chunks_rx, &|chunks| {
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared))
    })
    .await;
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "the detector must clear the stored request when the child exits; chunks: {chunks:?}"
    );

    // @step And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    // (the previous assertion IS the chunk-stream observation)
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(None)),
        "the stored request must be gone after the exit clear"
    );

    // @step And the stored request is cleared and the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "child exit must not flip the agent session status"
    );

    // BUG-415: the command has exited and its exec session was reaped; nothing to close.
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Detector clear on session removal from the store
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_session_removal_from_the_store() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
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
        .expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(
            move |request: Option<InternalExecStdinRequest>| {
                if let Err(error) = session_for_cb.set_exec_stdin_request(request) {
                    tracing::error!(%error, "cannot record the exec-stdin request");
                }
            },
        )),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'printf out\\n; sleep 45'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the request must be stored before the store removal"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the exec session is removed from the store while the agent session stays Running
    // (close drops the store entry + kills the child; the agent session
    //  itself is untouched and stays Running)
    close_exec(&tool, &exec_live).await;

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    let chunks = wait_for_chunks(&mut chunks_rx, &|chunks| {
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared))
    })
    .await;
    assert!(
        chunks
            .iter()
            .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared)),
        "the detector must clear the stored request when the exec session leaves the store; chunks: {chunks:?}"
    );

    // @step And an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    // (the previous assertion IS the chunk-stream observation)
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "store removal must not flip the agent session status"
    );

    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}
