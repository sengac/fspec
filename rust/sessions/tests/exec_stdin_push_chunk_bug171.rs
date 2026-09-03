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
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
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

// ─────────────────────────────────────────────────────────────────────────
// BUG-171 rules [4] + [5] — the detector observes the END of the prompt
// condition and pushes a clear through the same callback; a non-exit
// clear resets the re-fire cooldown.
// ─────────────────────────────────────────────────────────────────────────

/// Poll until the detector has stored a request (worst case ~4-5s:
/// 2s tick + 3s quiet threshold).
async fn wait_for_stored_request(
    session: &Arc<codelet_sessions::background_session::BackgroundSession>,
) {
    for _ in 0..100 {
        if session.get_exec_stdin_request().is_some() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Poll the chunk stream until `predicate` holds or the timeout elapses.
///
/// Returns EVERY chunk observed (in arrival order) — callers must
/// assert on the returned vec, NOT re-drain the receiver afterwards
/// (the chunks are consumed here).
///
/// Each `recv` is bounded (200ms) so the helper itself can never hang
/// when no chunks are ever emitted (the red-phase case): `recv()`
/// blocks forever on an idle broadcast channel, so an unbounded recv
/// would defeat the deadline loop.
async fn wait_for_chunks(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
    timeout: Duration,
    predicate: &dyn Fn(&[StreamChunk]) -> bool,
) -> Vec<StreamChunk> {
    let deadline = std::time::Instant::now() + timeout;
    let mut seen = Vec::new();
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            break;
        }
        let remaining = deadline - now;
        let recv_timeout = Duration::from_millis(200).min(remaining);
        match tokio::time::timeout(recv_timeout, rx.recv()).await {
            Ok(Ok((_, chunk))) => {
                seen.push(chunk);
                if predicate(&seen) {
                    return seen;
                }
            }
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => break,
            Err(_elapsed) => continue,
        }
    }
    seen
}

/// Scenario: Detector clear on output resumption pushes an exec-stdin cleared chunk
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_output_resumption_pushes_an_exec_stdin_cleared_chunk() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    // Prints once at t≈0, then goes quiet (detector fires at ~4s), then
    // produces output again at t≈10.
    let (tool, exec_live) =
        run_still_running(agent, "sh -c 'printf out\\n; sleep 10; printf resume\\n'").await;
    wait_for_stored_request(&session).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored before the output resumes"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the command produces output again so the session is no longer quiet
    // (the 'resume' line lands at t≈10; the 2s-cadence detector observes
    //  quiet < 3s on its next tick)

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    // (worst case: output at t≈10, detector tick + clear + reaper margin)
    let chunks =
        wait_for_chunks(&mut chunks_rx, Duration::from_secs(10), &|chunks| {
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
        session.get_exec_stdin_request().is_none(),
        "the stored request must be gone after the non-exit clear"
    );

    // @step And the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "the detector clear must not flip the status"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Detector clear on child exit emits a clear and the stored request is gone
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_child_exit_emits_a_clear_and_the_stored_request_is_gone() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    let (tool, exec_live) =
        run_still_running(agent, "sh -c 'printf out\\n; sleep 10'").await;
    wait_for_stored_request(&session).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored before the child exits"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the child exits and the reaper removes the exec session from the store
    // (the child exits at t≈10 on its own; the reaper drops the store entry)

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    let chunks =
        wait_for_chunks(&mut chunks_rx, Duration::from_secs(10), &|chunks| {
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
        session.get_exec_stdin_request().is_none(),
        "the stored request must be gone after the exit clear"
    );

    // @step And the stored request is cleared and the agent session status remains running
    assert_eq!(
        session.get_status(),
        SessionStatus::Running,
        "child exit must not flip the agent session status"
    );

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: Detector clear on session removal from the store
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_detector_clear_on_session_removal_from_the_store() {
    // @step Given a stored exec-stdin request on a Running agent session for a live exec session
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    let (tool, exec_live) =
        run_still_running(agent, "sh -c 'printf out\\n; sleep 45'").await;
    wait_for_stored_request(&session).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the request must be stored before the store removal"
    );

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the exec session is removed from the store while the agent session stays Running
    // (close drops the store entry + kills the child; the agent session
    //  itself is untouched and stays Running)
    close_exec(&tool, &exec_live).await;

    // @step Then the detector emits a clear to the agent-session callback within one detector tick
    let chunks =
        wait_for_chunks(&mut chunks_rx, Duration::from_secs(6), &|chunks| {
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

/// Scenario: A non-exit detector clear resets the per-exec-session re-fire cooldown
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_a_non_exit_detector_clear_resets_the_per_exec_session_re_fire_cooldown() {
    // @step Given a stored exec-stdin request on a Running agent session that fired the detector
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    // Subscribe BEFORE the command starts so the FIRST fire's request
    // chunk is captured too (the re-fire assertion compares both).
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // out@0 → quiet (fire ≈4s) → mid@9 → quiet again → re-fire ≈13s.
    let (tool, exec_live) =
        run_still_running(agent, "sh -c 'printf out\\n; sleep 9; printf mid\\n; sleep 20'").await;
    wait_for_stored_request(&session).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
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
    let chunks = wait_for_chunks(
        &mut chunks_rx,
        Duration::from_secs(15),
        &|chunks| {
            chunks
                .iter()
                .filter(|c| matches!(c, StreamChunk::ExecStdinRequest { .. }))
                .count()
                >= 2
        },
    )
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

    close_exec(&tool, &exec_live).await;
    codelet_tools::unified_exec::set_exec_stdin_request_callback(agent, None);
}

/// Scenario: A continuous quiet period still obeys the 30 second cooldown
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_a_continuous_quiet_period_still_obeyes_the_30_second_cooldown() {
    // @step Given a stored exec-stdin request on a Running agent session that fired the detector
    let _ = data_dir();
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    let (tool, exec_live) = run_still_running(agent, "sh -c 'sleep 40'").await;
    wait_for_stored_request(&session).await;
    assert!(
        session.get_exec_stdin_request().is_some(),
        "the first fire must store a request"
    );
    let first_ts: i64 = session
        .get_exec_stdin_request()
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
    let chunks =
        wait_for_chunks(&mut chunks_rx, Duration::from_secs(36), &|chunks| {
            chunks
                .iter()
                .any(|c| matches!(c, StreamChunk::ExecStdinRequest { .. }))
        })
        .await;
    let second_ts = chunks
        .iter()
        .find_map(|c| match c {
            StreamChunk::ExecStdinRequest { request } => Some(request.ts_ms),
            _ => None,
        })
        .expect(
            "the detector must re-fire once the 30s cooldown elapses; chunks: {chunks:?}",
        );
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
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager);
    let agent = Uuid::parse_str(&sid.value).expect("session key must be a UUID");
    let session = manager.get_session(&agent.to_string()).expect("session must exist");
    session.set_status(SessionStatus::Running);

    let session_for_cb = Arc::clone(&session);
    codelet_tools::unified_exec::set_exec_stdin_request_callback(
        agent,
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
        })),
    );

    // Exits at t≈2 — before the 3s quiet threshold, so the detector
    // never fires for this session.
    let (tool, exec_live) =
        run_still_running(agent, "sh -c 'echo fast; sleep 2'").await;

    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step When the detector ticks while the exec session is gone and the detector never fired
    // (the reaper removes the store entry within ~2s of the exit; the
    //  detector task observes the removal and stops without having fired)
    tokio::time::sleep(Duration::from_secs(6)).await;

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
        session.get_exec_stdin_request().is_none(),
        "the slot must remain empty — the detector never stored a request"
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
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
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
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
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
        Some(Arc::new(move |request: Option<InternalExecStdinRequest>| {
            session_for_cb.set_exec_stdin_request(request);
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

