use super::{
    fixture::{Harness, TestResult},
    retention,
};
use codelet_core::persistence::*;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancellation_during_original_persistence_preserves_audit_without_conversation_membership(
) -> TestResult {
    // @step Given a genuine Codex conversation and a prompt held at durable original persistence
    let mut harness = Harness::new()?;
    harness.seed().await?;
    let lease = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(harness.directory.path().join("messages.lock"))?;
    lease.lock()?;
    let journal = harness
        .directory
        .path()
        .join("persistence-recovery")
        .join(harness.session.id.to_string());
    let mut events = harness.session.subscribe_to_stream();
    harness
        .session
        .send_input("persistence cancellation".into(), None)?;
    harness.start()?;
    tokio::time::timeout(Duration::from_secs(10), async {
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
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    // @step When the prompt is canceled before persistence releases
    harness.session.interrupt();
    let heartbeat =
        tokio::time::timeout(Duration::from_millis(250), tokio::spawn(async { 284 })).await??;
    lease.unlock()?;
    let completion = retention::done(&mut events, 1).await;
    harness.stop().await;
    completion?;
    // @step Then asynchronous work remains responsive and the original remains recoverable
    assert_eq!(heartbeat, 284);
    retention::retained(&harness, &["persistence cancellation".into()]).await?;
    // @step And a fresh instruction completes on the unchanged runtime
    harness.fresh().await?;
    // @step And selected compaction accepts the same admitted history as runtime
    harness.compact().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancellation_while_awaiting_the_runtime_lock_does_not_admit_the_input(
) -> TestResult {
    // @step Given a genuine Codex conversation and a submitted prompt waiting for runtime ownership
    let mut harness = Harness::new()?;
    harness.seed().await?;
    let session = harness.session.clone();
    let ownership = session.inner.lock().await;
    let before = load_session(session.id)?.messages.len();
    let mut events = session.subscribe_to_stream();
    session.send_input("runtime lock cancellation".into(), None)?;
    harness.start()?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if load_session(session.id)?.messages.len() == before + 1 {
                return Ok::<_, String>(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await??;
    // @step When the prompt is canceled before runtime ownership becomes available
    session.interrupt();
    drop(ownership);
    let completion = retention::done(&mut events, 1).await;
    harness.stop().await;
    completion?;
    // @step Then the original remains recoverable without executing
    retention::retained(&harness, &["runtime lock cancellation".into()]).await?;
    // @step And cold conversation replay equals the admitted runtime history
    retention::equal_replay(&harness).await?;
    Ok(())
}
