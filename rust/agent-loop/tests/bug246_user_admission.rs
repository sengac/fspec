#![doc = "Feature: spec/features/persistence-user-admission.feature"]

use codelet_agent_loop::{agent_loop, persist::retry_pending_persistence};
use codelet_rpc_types::StreamChunk;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn user_persistence_failure_prevents_inference_and_retains_the_input() -> TestResult {
    // @step Given a real session with an unavailable message archive
    let (directory, session, input_rx) = fixture()?;
    let mut events = session.subscribe_to_stream();
    // @step When user input is submitted to the agent loop
    let worker = tokio::spawn(agent_loop(
        session.clone(),
        input_rx,
        tokio::sync::mpsc::channel(1).1,
    ));
    session.send_input("recover this original".into(), None)?;
    // @step Then a scoped persistence error is visible and no provider request starts
    let message = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let event = events.recv().await?;
            if let StreamChunk::Error { error } = event {
                break Ok::<_, tokio::sync::broadcast::error::RecvError>(error);
            }
        }
    })
    .await??;
    assert!(
        message.contains("persistence"),
        "expected scoped persistence error, received {message}"
    );
    assert!(session.inner.lock().await.messages.is_empty());
    worker.abort();
    let _ = worker.await;
    // @step And repairing storage and explicit recovery archives the original input once
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    retry_pending_persistence(&session.id)?;
    retry_pending_persistence(&session.id)?;
    let records = read(session.id)?;
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0]["message"]["content"][0]["text"],
        "recover this original"
    );
    Ok(())
}
