//! Feature: spec/features/prompt-admission-cancellation.feature
#![cfg(feature = "test-support")]

#[path = "support/prompt_admission.rs"]
pub mod support;

use std::sync::atomic::Ordering;
use support::{done, fixture, originals, StubGuard, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_queued_cancellation_survives_a_later_user_submission() -> TestResult {
    let (_directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    let mut events = session.subscribe_to_stream();
    // @step Given an accepted prompt waiting in the input queue
    session.send_input("cancelled original".into(), None)?;
    // @step When I cancel it and submit a genuinely new prompt
    session.interrupt();
    session.send_input("genuinely new".into(), None)?;
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    let completion = done(&mut events, 2).await;
    worker.abort();
    let _ = worker.await;
    completion?;
    // @step Then only the new prompt reaches the provider once
    assert_eq!(polls.load(Ordering::Acquire), 1);
    let messages = session.inner.lock().await.messages.clone();
    let history = serde_json::to_string(&messages)?;
    assert!(!history.contains("cancelled original"));
    assert!(history.contains("genuinely new"));
    // @step And the cancelled original remains recoverable without successful completion
    let archived = originals(session.id)?;
    assert_eq!(
        archived
            .iter()
            .filter(|record| record["message"]["content"][0]["text"] == "cancelled original")
            .count(),
        1
    );
    assert!(session
        .get_buffered_output(100)
        .iter()
        .any(|chunk| matches!(
            chunk,
            codelet_rpc_types::StreamChunk::InterruptedWithReason {
                reason: codelet_rpc_types::InterruptionReason::User,
                ..
            }
        )));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancellation_during_prompt_hooks_prevents_dispatch() -> TestResult {
    // @step Given an accepted prompt held by a submission hook
    let (directory, mut session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    support::block_hook(&mut session, directory.path())?;
    let mut events = session.subscribe_to_stream();
    session.send_input("hook original".into(), None)?;
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    support::wait_for(&directory.path().join("entered")).await?;
    // @step When I cancel it before the hook releases
    session.interrupt();
    std::fs::write(directory.path().join("release"), "release")?;
    let completion = done(&mut events, 1).await;
    worker.abort();
    let _ = worker.await;
    completion?;
    // @step Then no provider or tool is dispatched for that prompt
    assert_eq!(polls.load(Ordering::Acquire), 0);
    assert!(session.inner.lock().await.messages.is_empty());
    // @step And the original input remains recoverable with truthful interruption
    assert_eq!(originals(session.id)?.len(), 1);
    assert!(session
        .get_buffered_output(100)
        .iter()
        .any(|chunk| matches!(
            chunk,
            codelet_rpc_types::StreamChunk::InterruptedWithReason {
                reason: codelet_rpc_types::InterruptionReason::User,
                ..
            }
        )));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancellation_during_persistence_survives_startup() -> TestResult {
    // @step Given an accepted prompt held at its durable persistence boundary
    let (directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    codelet_core::persistence::get_message(uuid::Uuid::nil())?;
    let lease = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory.path().join("messages.lock"))?;
    lease.lock()?;
    let journal = directory
        .path()
        .join("persistence-recovery")
        .join(session.id.to_string());
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (heartbeat_tx, heartbeat_rx) = std::sync::mpsc::channel();
    let cancelled_session = session.clone();
    let controller = std::thread::spawn(move || -> Result<bool, String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if std::fs::read_dir(&journal).is_ok_and(|entries| {
                entries.flatten().any(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "json")
                })
            }) {
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err("persistence did not reach journal barrier".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // @step When I cancel it before storage releases
        cancelled_session.interrupt();
        ready_tx.send(()).map_err(|_| "barrier receiver closed")?;
        let responsive = heartbeat_rx
            .recv_timeout(std::time::Duration::from_millis(250))
            .is_ok();
        lease.unlock().map_err(|error| error.to_string())?;
        Ok(responsive)
    });
    let mut events = session.subscribe_to_stream();
    session.send_input("storage original".into(), None)?;
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    ready_rx.await?;
    let heartbeat = tokio::spawn(async move {
        let _ = heartbeat_tx.send(());
    });
    let completion = done(&mut events, 1).await;
    worker.abort();
    let _ = worker.await;
    completion?;
    // @step Then unrelated asynchronous work remains responsive
    heartbeat.await?;
    assert!(
        controller.join().map_err(|_| "controller failed")??,
        "persistence blocked the only async worker"
    );
    // @step And no provider or tool is dispatched for that prompt
    assert_eq!(polls.load(Ordering::Acquire), 0);
    assert!(session.inner.lock().await.messages.is_empty());
    // @step And cold resume retains the original input exactly once
    let archived = originals(session.id)?;
    assert_eq!(archived.len(), 1);
    assert_eq!(
        archived[0]["message"]["content"][0]["text"],
        "storage original"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_failed_persistence_retains_cancelled_input_for_recovery() -> TestResult {
    // @step Given an accepted prompt whose archive is unavailable
    let (directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    std::fs::create_dir_all(directory.path().join("messages/messages.jsonl"))?;
    let mut events = session.subscribe_to_stream();
    session.send_input("failed storage original".into(), None)?;
    // @step When I cancel it before admission finishes
    session.interrupt();
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    let completion = done(&mut events, 1).await;
    worker.abort();
    let _ = worker.await;
    completion?;
    // @step Then inference is refused without successful completion
    assert_eq!(polls.load(Ordering::Acquire), 0);
    assert!(session.inner.lock().await.messages.is_empty());
    assert!(session
        .get_buffered_output(100)
        .iter()
        .any(|chunk| matches!(chunk, codelet_rpc_types::StreamChunk::Error { .. })));
    // @step And explicit storage recovery retains the original input exactly once
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    codelet_agent_loop::persist::retry_pending_persistence(&session.id)?;
    codelet_agent_loop::persist::retry_pending_persistence(&session.id)?;
    let archived = originals(session.id)?;
    assert_eq!(archived.len(), 1);
    assert_eq!(
        archived[0]["message"]["content"][0]["text"],
        "failed storage original"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_supervisor_and_recovery_inputs_retain_cancellation_ownership() -> TestResult {
    use codelet_sessions::background_session::IncomingMessage;
    // @step Given queued supervisor input and staged recovery continuations
    let (_directory, session, receiver, polls) = fixture()?;
    let _stub = StubGuard;
    session.receive_incoming_message(IncomingMessage::new(
        "local supervisor".into(),
        "supervisor".into(),
        "stale supervisor".into(),
    )?)?;
    let epoch = session.compaction_epoch();
    session.send_compaction_continuation(epoch)?;
    session.interrupt_loop_detector("stale detector retry".into());
    let detector_epoch = session.compaction_epoch();
    // @step When I cancel them before admission and submit new user input
    session.interrupt();
    session.receive_incoming_message(IncomingMessage::new(
        "local supervisor".into(),
        "supervisor".into(),
        "stale supervisor queued during interruption".into(),
    )?)?;
    session.send_input("fresh after recovery".into(), None)?;
    let mut events = session.subscribe_to_stream();
    let worker = tokio::spawn(codelet_agent_loop::agent_loop(
        session.clone(),
        receiver,
        tokio::sync::mpsc::channel(1).1,
    ));
    let completion = done(&mut events, 4).await;
    worker.abort();
    let _ = worker.await;
    completion?;
    // @step Then stale supervisor and recovery work never reaches inference
    assert!(!session.admit_loop_retry(detector_epoch));
    assert!(session.compaction_cancelled(epoch));
    let history = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert!(!history.contains("stale supervisor"));
    // @step And only the genuinely new user input may start normally
    assert_eq!(polls.load(Ordering::Acquire), 1);
    assert!(history.contains("fresh after recovery"));
    Ok(())
}
