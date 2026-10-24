//! Feature: spec/features/terminal-recovery-single-owner-locking.feature

use std::sync::Arc;
use std::time::Duration;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use serial_test::serial;
use tokio::sync::mpsc;

#[path = "support/terminal_recovery.rs"]
mod support;
use support::fixture;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_ordinary_terminal_errors_release_the_session_lock() {
    // @step Given a background session running the real outer agent loop
    let (_directory, session, input_rx) = fixture();
    let mut chunks = session.subscribe_to_stream();
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    // @step When provider construction returns an ordinary terminal error
    session
        .send_input("exercise terminal failure".into(), None)
        .expect("queue input");
    let error = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let StreamChunk::Error { error, .. } = chunks.recv().await.expect("stream open") {
                break error;
            }
        }
    })
    .await;
    // @step Then the error is published within the harness deadline
    assert!(error.is_ok(), "ordinary terminal error must not deadlock");
    assert!(error
        .expect("terminal result")
        .contains("Unsupported provider"));
    // @step And a later status operation can acquire the session lock
    assert!(
        tokio::time::timeout(Duration::from_secs(1), session.inner.lock())
            .await
            .is_ok()
    );
    task.abort();
    let _ = task.await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_failed_overflow_recovery_preserves_committed_context() {
    // @step Given a session with committed conversation and a failing DAG builder
    let (_directory, session, _input_rx) = fixture();
    session
        .inner
        .lock()
        .await
        .messages
        .push(rig::message::Message::user("committed evidence"));
    session
        .inner
        .lock()
        .await
        .messages
        .push(rig::message::Message::assistant("committed response"));
    let before = serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot");
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        session.id,
        Some(Arc::new(|_, _| {
            Box::pin(async { Err("selected DAG operation failed".into()) })
        })),
    );
    let mut chunks = session.subscribe_to_stream();
    // @step When terminal overflow recovery runs under the caller ownership boundary
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            session.clone(),
            &support::overflow_error(),
        ),
    )
    .await
    .expect("recovery must terminate");
    // @step Then recovery returns a typed failure within the harness deadline
    assert!(
        matches!(
            outcome,
            Err(codelet_agent_loop::terminal_overflow_recovery::TerminalRecoveryError::Failed(_))
        ),
        "expected typed failure: {outcome:?}"
    );
    // @step And committed context is unchanged without compaction completion
    assert_eq!(
        serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot"),
        before
    );
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(chunk, StreamChunk::CompactionComplete { .. }));
    }
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(session.id, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn scenario_cancellation_releases_recovery_ownership_without_replacing_context() {
    // @step Given a session whose admitted DAG builder is waiting
    let (_directory, session, _input_rx) = fixture();
    session
        .inner
        .lock()
        .await
        .messages
        .push(rig::message::Message::user("committed evidence"));
    session
        .inner
        .lock()
        .await
        .messages
        .push(rig::message::Message::assistant("cancellation response"));
    let before = serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot");
    let entered = Arc::new(tokio::sync::Notify::new());
    let builder_entered = entered.clone();
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        session.id,
        Some(Arc::new(move |_, _| {
            let entered = builder_entered.clone();
            Box::pin(async move {
                entered.notify_one();
                std::future::pending().await
            })
        })),
    );
    let mut chunks = session.subscribe_to_stream();
    let recovering = session.clone();
    let task = tokio::spawn(async move {
        codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
            recovering,
            &support::overflow_error(),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .expect("builder starts");
    // @step When the user cancels terminal recovery
    session.interrupt();
    // @step Then recovery returns a cancellation within the harness deadline
    let result = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("cancellation terminates")
        .expect("recovery task joins");
    assert!(
        matches!(
            result,
            Err(codelet_agent_loop::terminal_overflow_recovery::TerminalRecoveryError::Cancelled)
        ),
        "expected cancellation: {result:?}"
    );
    // @step And the committed conversation remains available without completion
    assert_eq!(
        serde_json::to_value(&session.inner.lock().await.messages).expect("snapshot"),
        before
    );
    assert!(!session
        .compaction_in_progress
        .load(std::sync::atomic::Ordering::SeqCst));
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(chunk, StreamChunk::CompactionComplete { .. }));
    }
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(session.id, None);
}
