#![doc = "Feature: spec/features/agent-manager-await-idle-completion-races.feature"]

#[path = "../src/agent_manager_wait.rs"]
mod agent_manager_wait;

use codelet_rpc_types::{SessionState, StreamChunk};
use codelet_tools::agent_manager::AwaitOutcome;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn completion_during_initial_status_inspection_is_not_missed() -> TestResult {
    let (sender, _) = broadcast::channel(4);
    let receiver = agent_manager_wait::subscribe_before_check(
        || sender.subscribe(),
        || {
            let _ = sender.send(StreamChunk::session_state_change(SessionState::Idle));
            false
        },
    )
    .ok_or("running status must retain the subscription")?;
    let outcome = tokio::time::timeout(
        Duration::from_secs(1),
        agent_manager_wait::wait(receiver, || Some(false)),
    )
    .await?;
    assert!(matches!(outcome, AwaitOutcome::Idle));
    Ok(())
}

#[tokio::test]
async fn lagged_completion_resolves_from_authoritative_idle_status() -> TestResult {
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::session_state_change(SessionState::Idle))?;
    sender.send(StreamChunk::done())?;
    let outcome = tokio::time::timeout(
        Duration::from_secs(1),
        agent_manager_wait::wait(receiver, || Some(true)),
    )
    .await?;
    assert!(matches!(outcome, AwaitOutcome::Idle));
    Ok(())
}

#[tokio::test]
async fn lag_while_running_does_not_finish_the_wait() -> TestResult {
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::done())?;
    sender.send(StreamChunk::done())?;
    let checked = AtomicBool::new(false);
    let waiter = agent_manager_wait::wait(receiver, || {
        checked.store(true, Ordering::SeqCst);
        Some(false)
    });
    tokio::pin!(waiter);
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut waiter)
        .await
        .is_err());
    assert!(checked.load(Ordering::SeqCst));
    sender.send(StreamChunk::session_state_change(SessionState::Idle))?;
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), waiter).await?,
        AwaitOutcome::Idle
    ));
    Ok(())
}

#[tokio::test]
async fn destroyed_subordinate_resolves_after_stream_lag() -> TestResult {
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::done())?;
    sender.send(StreamChunk::done())?;
    let outcome = tokio::time::timeout(
        Duration::from_secs(1),
        agent_manager_wait::wait(receiver, || None),
    )
    .await?;
    assert!(matches!(outcome, AwaitOutcome::Destroyed));
    Ok(())
}

#[test]
fn already_idle_subordinate_needs_no_stream_event() {
    let (sender, _) = broadcast::channel::<StreamChunk>(1);
    assert!(agent_manager_wait::subscribe_before_check(|| sender.subscribe(), || true).is_none());
}

#[tokio::test]
async fn closed_stream_resolves_as_destroyed() {
    let (sender, receiver) = broadcast::channel(1);
    drop(sender);
    assert!(matches!(
        agent_manager_wait::wait(receiver, || Some(false)).await,
        AwaitOutcome::Destroyed
    ));
}
