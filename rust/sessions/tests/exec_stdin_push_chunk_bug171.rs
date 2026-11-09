//! Feature: spec/features/exec-stdin-push-emission.feature
//!
//! BUG-171 — exec-stdin slot transitions push StreamChunks (request on
//! store, cleared on child exit / successful write) with no status flip.
//! Harness: `exec_stdin_push_chunk_bug171_support` (REFAC-060).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, SessionState, SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::ExecStdinRequest as InternalExecStdinRequest;
use serial_test::serial;
use tokio::sync::broadcast;
use uuid::Uuid;

mod exec_stdin_push_chunk_bug171_support;
use exec_stdin_push_chunk_bug171_support::{
    close_exec, data_dir, fresh_session, run_still_running,
};

/// Collect every chunk delivered on the manager's chunk broadcast since
/// `rx` was created.
fn drained_chunks(rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();
    while let Ok((_, chunk)) = rx.try_recv() {
        chunks.push(chunk);
    }
    chunks
}

/// TEST-042: wait for the quiet detector to store its request (it fires on
/// its own cadence; the test waits on the stored state, not a fixed time).
async fn wait_for_stored_request(
    session: &codelet_sessions::background_session::BackgroundSession,
) {
    codelet_test_helpers::wait_until("the detector stores the exec-stdin request", || {
        std::future::ready(matches!(session.get_exec_stdin_request(), Ok(Some(_))))
    })
    .await
    .expect("the detector stores the exec-stdin request");
}

/// TEST-042: every chunk up to and including the first that `wanted`
/// accepts (the test waits on the chunk, not a fixed time).
async fn chunks_until(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
    what: &str,
    wanted: impl Fn(&StreamChunk) -> bool,
) -> Vec<StreamChunk> {
    codelet_test_helpers::deadlock_guard(what, async {
        let mut chunks = Vec::new();
        loop {
            match rx.recv().await {
                Ok((_, chunk)) => {
                    let found = wanted(&chunk);
                    chunks.push(chunk);
                    if found {
                        return chunks;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return chunks,
            }
        }
    })
    .await
    .expect("the chunk stream delivers the awaited chunk")
}

async fn chunks_until_cleared(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
) -> Vec<StreamChunk> {
    chunks_until(rx, "the exec-stdin cleared chunk", |chunk| {
        matches!(chunk, StreamChunk::ExecStdinRequestCleared)
    })
    .await
}

/// Scenario: Detector fire while session stays Running pushes an exec-stdin request chunk
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_fire_while_session_stays_running_pushes_an_exec_stdin_request_chunk() {
    // @step Given the agent session is Running and focused with no HITL prompt in the slot
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
    assert!(
        session.get_hitl_request().is_none(),
        "no HITL prompt may pre-exist"
    );

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

    let mut chunks_rx = manager.chunks_tx().subscribe();
    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;

    // @step When the exec-stdin quiet detector fires for a live exec session and the agent session callback stores the request on the BackgroundSession
    // (quiet >= 3s → the 2s-cadence detector emits within ~4s)
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "detector must have fired and stored the request"
    );

    // @step Then an exec-stdin request StreamChunk with that request is pushed on the session chunk stream
    let chunks = chunks_until(&mut chunks_rx, "the exec-stdin request chunk", |chunk| {
        matches!(chunk, StreamChunk::ExecStdinRequest { .. })
    })
    .await;
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
        !chunks.iter().any(|c| matches!(
            c,
            StreamChunk::SessionStateChange {
                state: SessionState::Paused
            }
        )),
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

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the request must be stored before the clear"
    );

    // @step When the exec-stdin slot transitions from Some to None
    let mut chunks_rx = manager.chunks_tx().subscribe();
    assert!(session.set_exec_stdin_request(None).is_ok());

    // @step Then an exec-stdin cleared StreamChunk is pushed on the session chunk stream
    let chunks = chunks_until_cleared(&mut chunks_rx).await;
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

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the request must be stored before the child exits"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the underlying exec session child exits and the alive check runs
    close_exec(&tool, &exec_live).await;
    // Wait for the reaper to drop the store entry so the alive check
    // can observe the exit.
    let exec = &exec_live;
    codelet_test_helpers::wait_until("the reaper drops the exec session", || async move {
        !codelet_tools::unified_exec::global_store()
            .contains(exec)
            .await
    })
    .await
    .expect("the reaper drops the exec session");
    let handle: &dyn SessionManagerHandle = &*manager;
    let wire = handle
        .get_exec_stdin_request(&sid)
        .expect("get_exec_stdin_request reads the session");
    assert!(
        wire.is_none(),
        "the alive check must clear the stale stored request"
    );

    // @step Then the stored request is cleared
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(None)),
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

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 45'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
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
    let chunks = chunks_until_cleared(&mut chunks_rx).await;
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
        matches!(session.get_exec_stdin_request(), Ok(None)),
        "the stored request must be cleared after a successful write"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}
