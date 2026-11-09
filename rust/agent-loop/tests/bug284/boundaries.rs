use super::{
    fixture::{Harness, TestResult},
    retention,
};
use codelet_core::persistence::input_receipts::{self, ExecutionOutcome, InputDisposition};
use codelet_core::persistence::*;

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
    codelet_test_helpers::deadlock_guard("the awaited block", async {
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
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?;
    // @step When the prompt is canceled before persistence releases
    harness.session.interrupt();
    let heartbeat = codelet_test_helpers::deadlock_guard(
        "tokio::spawn(async { 284 })",
        tokio::spawn(async { 284 }),
    )
    .await??;
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
    let mut events = session.subscribe_to_stream();
    session.send_input("runtime lock cancellation".into(), None)?;
    let submission = codelet_core::persistence::input_receipts::list(session.id)?
        .last()
        .ok_or("acknowledged submission has no receipt")?
        .id;
    harness.start()?;
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            if get_message(submission)?.is_some() {
                return Ok::<_, String>(());
            }
            codelet_test_helpers::yield_now().await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_interrupted_execution_cannot_revoke_an_already_admitted_input() -> TestResult {
    // @step Given a submitted input has won durable admission and runtime installation
    let mut harness = Harness::new()?;
    harness.seed().await?;
    let mut events = harness.session.subscribe_to_stream();
    harness.start()?;
    let (submission, release) =
        retention::hold_admission(&harness, "admitted before interruption").await?;
    // @step When execution is interrupted before provider dispatch
    harness.session.interrupt();
    release.send(())?;
    let completion = retention::done(&mut events, 1).await;
    harness.stop().await;
    completion?;
    // @step Then the admitted input remains in runtime and cold replay exactly once
    let receipt = input_receipts::get(harness.session.id, submission)?.ok_or("receipt")?;
    assert_eq!(receipt.disposition, InputDisposition::Admitted);
    assert_eq!(
        receipt.execution,
        Some(ExecutionOutcome::InterruptedBeforeDispatch)
    );
    let manifest = load_session(harness.session.id)?;
    let members = manifest.messages.iter();
    assert_eq!(members.filter(|m| m.message_id == submission).count(), 1);
    let runtime = serde_json::to_string(&harness.session.inner.lock().await.messages)?;
    assert_eq!(runtime.matches("admitted before interruption").count(), 1);
    retention::equal_replay(&harness).await?;
    // @step And a fresh subsequent input may continue without replaying the old input as new work
    let wire = harness.fresh().await?;
    assert_eq!(wire.matches("admitted before interruption").count(), 1);
    // @step And truthful durable cancellation evidence is model visible before subsequent inference
    let disposition = codelet_agent_loop::input_admission::disposition_text(
        submission,
        &ExecutionOutcome::InterruptedBeforeDispatch,
    )?;
    assert!(
        wire.contains(&disposition),
        "disposition was not model visible"
    );
    let full = get_session_messages_full(&load_session(harness.session.id)?)?;
    assert!(full.iter().any(|m| m.content == disposition));
    harness.compact().await?;
    Ok(())
}
