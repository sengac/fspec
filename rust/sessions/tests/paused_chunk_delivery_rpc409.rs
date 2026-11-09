//! Feature: spec/features/paused-chunk-delivery-during-blocking-waits.feature
//!
//! RPC-409: chunks emitted immediately before a blocking wait
//! (`wait_for_pause_response` / `wait_for_fspec_response` /
//! `wait_for_hitl_response`) must reach broadcast subscribers WHILE the
//! wait is still pending. Before the fix, the emitting task blocked its
//! tokio worker in a std mpsc recv right after `broadcast::Sender::send`
//! woke the parked subscriber into that worker's non-stealable LIFO
//! slot — stranding delivery until the wait resolved (the inline pause
//! prompt never appeared). The tests mirror the production shape: BOTH
//! broadcast subscribers parked at emit time, handler on a spawned task.
//!
//! HANG-SAFETY (why these tests are structured observe → unblock →
//! assert): each scenario spawns a task that blocks its tokio worker in
//! a synchronous `std::mpsc::recv`. If an assertion panics while that
//! task is still parked, the test unwinds, the multi-thread runtime is
//! dropped, and runtime shutdown joins its worker threads — a worker
//! parked in a sync recv never wakes, so the test HANGS forever instead
//! of failing. Therefore every scenario:
//!   1. captures outcomes WITHOUT panicking,
//!   2. UNCONDITIONALLY sends the unblocking response,
//!   3. joins the handler with a hard timeout,
//!   4. only THEN runs assertions.
//!
//! Never reorder these — an assert before the unblock reintroduces the
//! hang-on-failure mode.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_providers::ProviderManager;
use codelet_rpc_types::{SessionId, SessionState, SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::request_user_input::HitlResponse;
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use serial_test::serial;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

type ChunksRx = broadcast::Receiver<(SessionId, StreamChunk)>;
type StatusRx = broadcast::Receiver<(SessionId, SessionStatus)>;

/// Build a real BackgroundSession wired to fresh manager-style
/// broadcast channels (fake env credential — no LLM is ever called).
fn fresh_session() -> (Arc<BackgroundSession>, ChunksRx, StatusRx) {
    std::env::set_var("ANTHROPIC_API_KEY", "rpc409-fake-key");
    let tmp = codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_common::set_data_directory(tmp).expect("set the data directory");

    let provider_manager =
        ProviderManager::with_provider("claude").expect("claude provider with fake env key");
    let inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

    let (chunks_tx, chunks_rx) = broadcast::channel::<(SessionId, StreamChunk)>(1024);
    let (status_tx, status_rx) = broadcast::channel::<(SessionId, SessionStatus)>(1024);
    // RPC-436: manager-owned session-cleared lifecycle event channel —
    // the test never clears the session, so a local unused sender is fine.
    let (session_cleared_tx, _session_cleared_rx) = broadcast::channel::<SessionId>(1024);
    let (input_tx, _input_rx) = mpsc::channel(8);

    let session = Arc::new(BackgroundSession::new(
        Uuid::new_v4(),
        "rpc409".to_string(),
        "/tmp".to_string(),
        None,
        None,
        inner,
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        status_tx,
        session_cleared_tx,
    ));
    (session, chunks_rx, status_rx)
}

/// Park a status watcher exactly like the TUI's bootstrap status task —
/// the RPC-409 stranding manifests with BOTH broadcast subscribers
/// parked at emit time (the production shape).
fn spawn_parked_status_watcher(mut rx: StatusRx) {
    tokio::spawn(async move { while rx.recv().await.is_ok() {} });
}

/// Spawn a dedicated subscriber task that parks on `rx.recv()` and
/// reports the first chunk matching `pred` on the returned channel.
/// The stranding only manifests when the subscriber's waker is already
/// registered at send time, so callers wait for the task to start and
/// yield to it BEFORE triggering the emit — see [`park_subscribers`].
fn spawn_parked_subscriber(
    mut rx: ChunksRx,
    pred: impl Fn(&StreamChunk) -> bool + Send + 'static,
) -> (
    mpsc::UnboundedReceiver<StreamChunk>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let (notify_tx, notify_rx) = mpsc::unbounded_channel();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let _ = started_tx.send(());
        loop {
            match rx.recv().await {
                Ok((_, chunk)) if pred(&chunk) => {
                    let _ = notify_tx.send(chunk);
                    break;
                }
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    (notify_rx, started_rx)
}

/// Let the spawned watcher task reach `recv().await` so its waker is
/// registered before the handler emits and blocks. TEST-042: no sleep —
/// wait for the task to start, then yield to the runtime so it parks.
async fn park_subscribers(started: tokio::sync::oneshot::Receiver<()>) {
    let started = codelet_test_helpers::deadlock_guard("the subscriber task starts", started).await;
    assert!(matches!(started, Ok(Ok(()))), "{started:?}");
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
}

/// Join a spawned handler under the deadlock guard. Returns `None` when
/// the guard expires or the task panics instead of awaiting forever —
/// callers assert on the `Option` so a stuck handler FAILS the test
/// rather than hanging it. Only call this AFTER the unblocking response
/// was sent.
async fn join_within<T>(handle: tokio::task::JoinHandle<T>) -> Option<T> {
    codelet_test_helpers::deadlock_guard("the handler task", handle)
        .await
        .ok()
        .and_then(std::result::Result::ok)
}

fn is_paused_chunk(chunk: &StreamChunk) -> bool {
    matches!(
        chunk,
        StreamChunk::SessionStateChange {
            state: SessionState::Paused,
        }
    )
}

// Scenario: Paused chunk reaches subscribers while the pause wait is still pending
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn paused_chunk_reaches_subscribers_while_the_pause_wait_is_still_pending() {
    // @step Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    let (session, chunks_rx, status_rx) = fresh_session();
    spawn_parked_status_watcher(status_rx);
    session.set_status(SessionStatus::Running);
    let (mut paused_rx, started) = spawn_parked_subscriber(chunks_rx, is_paused_chunk);
    park_subscribers(started).await;

    // @step When a tokio task runs the agent-loop pause handler which emits SessionStateChange Paused and blocks in wait_for_pause_response
    let handler_session = Arc::clone(&session);
    let handler = tokio::spawn(async move {
        // Verbatim shape of agent_loop.rs:501-516.
        handler_session.set_pause_state(Some(PauseState {
            kind: PauseKind::Triple,
            tool_name: "Read".to_string(),
            message: "Environment files often contain secrets".to_string(),
            details: Some("/tmp/.env".to_string()),
            tool_call_id: None,
        }));
        handler_session.set_status(SessionStatus::Paused);
        let response = handler_session.wait_for_pause_response();
        handler_session.set_status(SessionStatus::Running);
        response
    });

    // @step Then the subscriber receives the Paused chunk within 1 second while the pause is still pending
    // Observe WITHOUT panicking — the handler is parked in a sync recv
    // and a panic here would hang runtime shutdown (see module docs).
    let got =
        codelet_test_helpers::deadlock_guard("the chunk on paused_rx", paused_rx.recv()).await;
    let delivered_while_pending = matches!(&got, Ok(Some(_)));
    let wait_still_pending = !handler.is_finished();

    // @step And sending a pause response afterwards unblocks the handler with that response
    // UNCONDITIONAL unblock BEFORE any assertion — never reorder.
    // AMGR-020: the reply names the pending request (the Paused chunk came
    // after set_pause_state stored it).
    let answered = match session.pending_pause_route() {
        Some(route) => session.answer_pause(
            &route.request_id,
            &codelet_rpc_types::RequestHolder::User,
            codelet_sessions::background_session::PauseReply::Triple(
                codelet_rpc_types::ApprovalChoice::Deny,
            ),
        ),
        None => Err("no pending pause to answer".to_string()),
    };
    let response = join_within(handler).await;
    assert_eq!(answered, Ok(()));

    assert!(
        delivered_while_pending,
        "SessionStateChange{{Paused}} must be delivered while the pause wait is pending — \
         it was stranded on the blocked worker's LIFO slot (RPC-409)"
    );
    assert!(
        wait_still_pending,
        "the pause wait must still be pending when the chunk arrives"
    );
    assert_eq!(
        response,
        Some(PauseResponse::Denied),
        "handler must unblock with the sent response"
    );
}

// Scenario: Fspec request chunk reaches subscribers while the fspec wait is still pending
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn fspec_request_chunk_reaches_subscribers_while_the_fspec_wait_is_still_pending() {
    // @step Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    let (session, chunks_rx, status_rx) = fresh_session();
    spawn_parked_status_watcher(status_rx);
    let (mut request_rx, started) = spawn_parked_subscriber(chunks_rx, |c| {
        matches!(c, StreamChunk::FspecCommandRequest { .. })
    });
    park_subscribers(started).await;

    // @step When a tokio task emits an FspecCommandRequest chunk and blocks in wait_for_fspec_response
    let handler_session = Arc::clone(&session);
    let handler = tokio::spawn(async move {
        handler_session.handle_output(StreamChunk::fspec_command_request(
            codelet_rpc_types::FspecRequest {
                command: "list-work-units".to_string(),
                args_json: "{}".to_string(),
                project_root: "/tmp".to_string(),
                tool_call_id: "rpc409-test".to_string(),
            },
        ));
        handler_session.wait_for_fspec_response()
    });

    // @step Then the subscriber receives the FspecCommandRequest chunk within 1 second while the wait is still pending
    // Observe WITHOUT panicking (see module docs), unblock, THEN assert.
    let got =
        codelet_test_helpers::deadlock_guard("the chunk on request_rx", request_rx.recv()).await;
    let delivered_while_pending = matches!(&got, Ok(Some(_)));
    let wait_still_pending = !handler.is_finished();

    // @step And sending an fspec result afterwards unblocks the waiter with that result
    // UNCONDITIONAL unblock BEFORE any assertion — never reorder.
    session.send_fspec_result(codelet_rpc_types::FspecResult {
        success: true,
        data: "ok".to_string(),
        error: None,
        system_reminder: None,
        tool_call_id: "rpc409-test".to_string(),
    });
    let result = join_within(handler).await;

    assert!(
        delivered_while_pending,
        "FspecCommandRequest must be delivered while wait_for_fspec_response is pending (RPC-409)"
    );
    assert!(
        wait_still_pending,
        "the fspec wait must still be pending when the chunk arrives"
    );
    assert!(
        result.as_ref().is_some_and(|r| r.success && r.data == "ok"),
        "waiter must unblock with the sent result, got {result:?}"
    );
}

// Scenario: Paused chunk reaches subscribers while the HITL wait is still pending
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn paused_chunk_reaches_subscribers_while_the_hitl_wait_is_still_pending() {
    // @step Given a BackgroundSession on a multi-thread tokio runtime with a chunks broadcast subscriber
    let (session, chunks_rx, status_rx) = fresh_session();
    spawn_parked_status_watcher(status_rx);
    let (mut paused_rx, started) = spawn_parked_subscriber(chunks_rx, is_paused_chunk);
    park_subscribers(started).await;

    // @step When a tokio task emits SessionStateChange Paused and blocks in wait_for_hitl_response
    // AMGR-020: the question is stored before the wait, as the handler does.
    session.set_hitl_request(Some(codelet_tools::request_user_input::HitlRequest {
        questions: Vec::new(),
    }));
    let handler_session = Arc::clone(&session);
    let handler = tokio::spawn(async move {
        // Shape of the BUG-117 HITL handler: set status Paused, block.
        handler_session.set_status(SessionStatus::Paused);
        handler_session.wait_for_hitl_response()
    });

    // @step Then the subscriber receives the Paused chunk within 1 second while the wait is still pending
    // Observe WITHOUT panicking (see module docs), unblock, THEN assert.
    let got =
        codelet_test_helpers::deadlock_guard("the chunk on paused_rx", paused_rx.recv()).await;
    let delivered_while_pending = matches!(&got, Ok(Some(_)));
    let wait_still_pending = !handler.is_finished();

    // @step And sending a HITL response afterwards unblocks the waiter with that response
    // UNCONDITIONAL unblock BEFORE any assertion — never reorder.
    let answered = match session.pending_question_route() {
        Some(route) => session.answer_question(
            &route.request_id,
            &codelet_rpc_types::RequestHolder::User,
            HitlResponse::Cancelled { cancelled: false },
        ),
        None => Err("no pending question to answer".to_string()),
    };
    let response = join_within(handler).await;
    assert_eq!(answered, Ok(()));

    assert!(
        delivered_while_pending,
        "SessionStateChange{{Paused}} must be delivered while wait_for_hitl_response is pending (RPC-409)"
    );
    assert!(
        wait_still_pending,
        "the HITL wait must still be pending when the chunk arrives"
    );
    assert!(
        matches!(response, Some(HitlResponse::Cancelled { cancelled: false })),
        "waiter must unblock with the sent response, got {response:?}"
    );
}

// Scenario: Waits fall back to a direct blocking recv when called off-runtime
#[test]
#[serial]
fn waits_fall_back_to_a_direct_blocking_recv_when_called_off_runtime() {
    // @step Given a BackgroundSession and a plain OS thread outside any tokio runtime context
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build runtime");
    let (session, _chunks_rx, _status_rx) = rt.block_on(async { fresh_session() });
    drop(rt);

    // @step When the thread calls wait_for_pause_response and a pause response is sent from another thread
    // Report the result over a channel instead of an unbounded join():
    // the deadlock guard below bounds the wait, and if the waiter panics the
    // sender is dropped, which surfaces as a recv error (= test failure).
    session.set_pause_state(Some(PauseState {
        kind: PauseKind::Triple,
        tool_name: "Read".to_string(),
        message: "Environment files often contain secrets".to_string(),
        details: Some("/tmp/.env".to_string()),
        tool_call_id: None,
    }));
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let waiter_session = Arc::clone(&session);
    std::thread::spawn(move || {
        let _ = done_tx.send(waiter_session.wait_for_pause_response());
    });
    // TEST-042: no sleep; the response is sent whether or not the waiter
    // has started waiting yet.
    let route = session.pending_pause_route().expect("the pause is pending");
    session
        .answer_pause(
            &route.request_id,
            &codelet_rpc_types::RequestHolder::User,
            codelet_sessions::background_session::PauseReply::Triple(
                codelet_rpc_types::ApprovalChoice::Approve,
            ),
        )
        .expect("the pause is answered");

    // @step Then the waiter returns that response without panicking
    // Bounded wait: if the waiter never unblocks this FAILS in 5s
    // instead of hanging forever. A leaked parked thread cannot keep
    // the process alive once the test harness exits.
    let response =
        codelet_test_helpers::deadlock_guard_blocking("the off-runtime waiter", move || {
            done_rx.recv()
        });
    assert_eq!(
        response.ok().and_then(Result::ok),
        Some(PauseResponse::AllowOnce),
        "off-runtime waiter must return the sent response without panicking"
    );
}
