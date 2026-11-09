//! Feature: spec/features/exec-stdin-detector-cooldown.feature
//!
//! BUG-171 rule [5] — a non-exit detector clear resets the re-fire
//! cooldown, a continuous quiet period still obeys the 30 second cooldown,
//! and a detector that never fired emits no clear. Split from
//! exec_stdin_push_chunk_bug171.rs (REFAC-060).

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

/// Scenario: A non-exit detector clear resets the per-exec-session re-fire cooldown
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_a_non_exit_detector_clear_resets_the_per_exec_session_re_fire_cooldown() {
    // @step Given a stored exec-stdin request on a Running agent session that fired the detector
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

    // Subscribe BEFORE the command starts so the FIRST fire's request
    // chunk is captured too (the re-fire assertion compares both).
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // out@0 → quiet (fire ≈4s) → mid@9 → quiet again → re-fire ≈13s.
    let (tool, exec_live) = run_still_running(
        agent,
        "sh -c 'printf out\\n; sleep 9; printf mid\\n; sleep 20'",
    )
    .await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the first fire must store a request"
    );

    // @step When the command produces output again and the detector clears the stored request
    // (mid@9 clears; the detector keeps watching — it may re-fire in the
    // next quiet window once its own quiet counter crosses the 3s threshold)

    // @step Then the detector may re-fire after a fresh quiet period without waiting out the previous 30 second window
    // (the clear must have RESET the cooldown — without the reset the
    //  30s window from the first fire would suppress the re-fire until
    //  ~t=34, well past this test's window; with the reset the re-fire
    //  lands ≈4-5s after mid@9)
    let chunks = wait_for_chunks(&mut chunks_rx, &|chunks| {
        chunks
            .iter()
            .filter(|c| matches!(c, StreamChunk::ExecStdinRequest { .. }))
            .count()
            >= 2
    })
    .await;

    // @step And a second exec-stdin request StreamChunk with a newer fire timestamp is pushed on the session chunk stream
    // (all exec-stdin chunks observed by the helper — they were consumed
    //  from the receiver, so this vec is the authoritative record)
    let requests: Vec<codelet_rpc_types::ExecStdinRequest> = chunks
        .iter()
        .filter_map(|c| match c {
            StreamChunk::ExecStdinRequest { request } => Some(request.clone()),
            _ => None,
        })
        .collect();
    let cleared = chunks
        .iter()
        .any(|c| matches!(c, StreamChunk::ExecStdinRequestCleared));
    assert!(
        cleared,
        "the non-exit clear chunk must have ridden the stream before the re-fire; chunks: {chunks:?}"
    );
    assert!(
        requests.len() >= 2,
        "the detector must re-fire after the non-exit clear reset the cooldown; requests: {requests:?}"
    );
    assert!(
        requests[1].ts_ms > requests[0].ts_ms,
        "the second request must carry a newer fire timestamp; got {requests:?}"
    );
    // TEST-042: the test waits for the re-fire however long it takes, so
    // the reset is told apart by the fire timestamps: without it the
    // cooldown holds the re-fire to at least 30s after the first fire.
    assert!(
        requests[1].ts_ms - requests[0].ts_ms < 30_000,
        "the re-fire must come before the 30s cooldown would have ended; got {requests:?}"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: A continuous quiet period still obeys the 30 second cooldown
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_a_continuous_quiet_period_still_obeyes_the_30_second_cooldown() {
    // @step Given a stored exec-stdin request on a Running agent session that fired the detector
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

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 40'").await;
    wait_for_stored_request(&session).await;
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(Some(_))),
        "the first fire must store a request"
    );
    let first_ts: i64 = session
        .get_exec_stdin_request()
        .ok()
        .flatten()
        .expect("just asserted Some")
        .ts_ms
        .min(i64::MAX as u64) as i64;

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the command stays quiet for at least 30 seconds without producing output or exiting
    // (the child runs 40s without output; the observation window below
    //  waits for the SECOND request chunk — the re-fire — which the
    //  cooldown must hold back until 30s after the first fire)

    // @step Then the detector does not emit a second request before the 30 second cooldown elapses
    // (the first ExecStdinRequest observed AFTER the first fire IS the
    //  re-fire; its fire timestamp must sit at/after the first fire +
    //  30s, landing on the first 2s tick past the cooldown)
    // TEST-042: the first fire's own chunk may still arrive after the
    // subscription (the request is stored before it is pushed); the re-fire
    // is the first request with a newer fire timestamp.
    let chunks = wait_for_chunks(&mut chunks_rx, &|chunks| {
        chunks.iter().any(
            |c| matches!(c, StreamChunk::ExecStdinRequest { request } if request.ts_ms > first_ts),
        )
    })
    .await;
    let second_ts = chunks
        .iter()
        .find_map(|c| match c {
            StreamChunk::ExecStdinRequest { request } if request.ts_ms > first_ts => {
                Some(request.ts_ms)
            }
            _ => None,
        })
        .expect("the detector must re-fire once the 30s cooldown elapses; chunks: {chunks:?}");
    let delta_ms = second_ts.saturating_sub(first_ts);
    assert!(
        delta_ms >= 29_000,
        "no second request may ride the stream before the 30s cooldown elapses; re-fire delta: {delta_ms}ms"
    );
    assert!(
        delta_ms <= 35_000,
        "the re-fire must land on the first tick past the cooldown (2s cadence); delta: {delta_ms}ms"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: A detector that never fired emits no clear
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_a_detector_that_never_fired_emits_no_clear() {
    // @step Given an agent session with exec-stdin callbacks registered and a live exec session that exits before the quiet threshold
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

    // Exits at t≈2 — before the 3s quiet threshold, so the detector
    // never fires for this session.
    let (tool, exec_live) = run_still_running(agent, "sh -c 'echo fast; sleep 2'").await;

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the detector ticks while the exec session is gone and the detector never fired
    // (the reaper removes the store entry within ~2s of the exit; the
    //  detector task observes the removal and stops without having fired)
    // TEST-042: wait for the reaper to drop the exited session; the
    // detector can only fire while the child runs, so nothing can be
    // pushed once the entry is gone.
    let exec = &exec_live;
    let removed = codelet_test_helpers::wait_until(
        "the reaper drops the exited exec session",
        || async move {
            !codelet_tools::unified_exec::global_store()
                .contains(exec)
                .await
        },
    )
    .await;
    assert!(removed.is_ok(), "{removed:?}");

    // @step Then no exec-stdin request or cleared StreamChunk is pushed on the session chunk stream
    let mut request_chunks = 0;
    let mut cleared_chunks = 0;
    while let Ok((_, chunk)) = chunks_rx.try_recv() {
        match chunk {
            StreamChunk::ExecStdinRequest { .. } => request_chunks += 1,
            StreamChunk::ExecStdinRequestCleared => cleared_chunks += 1,
            _ => {}
        }
    }
    assert_eq!(
        request_chunks, 0,
        "no request chunk may be pushed when the detector never fired"
    );
    assert_eq!(
        cleared_chunks, 0,
        "no cleared chunk may be pushed when the detector never fired (nothing to clear)"
    );

    // @step And the agent session slot remains empty
    assert!(
        matches!(session.get_exec_stdin_request(), Ok(None)),
        "the slot must remain empty — the detector never stored a request"
    );

    // The reaper already removed the exited session; nothing is left to close.
    drop(tool);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}
