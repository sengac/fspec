#![doc = "Feature: spec/features/agent-manager-await-idle-completion-races.feature"]

// AMGR-021: this file tests the idle wait only; the request ending
// (`WaitEnd::Request`) is read by the handler, not here.
#[allow(dead_code)]
#[path = "../src/agent_manager_wait.rs"]
mod agent_manager_wait;

use codelet_rpc_types::{SessionState, StreamChunk};
use codelet_tools::agent_manager::AwaitOutcome;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn completion_during_initial_status_inspection_is_not_missed() -> TestResult {
    // @step Given a running subordinate with no timeout on its idle wait
    let (sender, _) = broadcast::channel(4);
    // @step When it completes while the waiter inspects its initial status
    let receiver = agent_manager_wait::subscribe_before_check(
        || sender.subscribe(),
        || {
            let _ = sender.send(StreamChunk::session_state_change(SessionState::Idle));
            false
        },
    )
    .ok_or("running status must retain the subscription")?;
    // @step Then the waiter observes completion without requiring another event
    let outcome = codelet_test_helpers::deadlock_guard(
        "agent_manager_wait::wait(receiver, || Some(false), |_| None, || None)",
        agent_manager_wait::wait(receiver, || Some(false), |_| None, || None),
    )
    .await?;
    assert!(matches!(
        outcome,
        agent_manager_wait::WaitEnd::Outcome(AwaitOutcome::Idle)
    ));
    Ok(())
}

#[tokio::test]
async fn lagged_completion_resolves_from_authoritative_idle_status() -> TestResult {
    // @step Given the subordinate's idle event has been overwritten by later output
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::session_state_change(SessionState::Idle))?;
    sender.send(StreamChunk::done())?;
    // @step When the waiter detects stream lag and the subordinate is idle
    let outcome = codelet_test_helpers::deadlock_guard(
        "agent_manager_wait::wait(receiver, || Some(true), |_| None, || None)",
        agent_manager_wait::wait(receiver, || Some(true), |_| None, || None),
    )
    .await?;
    // @step Then the wait resolves as idle without another state change
    assert!(matches!(
        outcome,
        agent_manager_wait::WaitEnd::Outcome(AwaitOutcome::Idle)
    ));
    Ok(())
}

#[tokio::test]
async fn lag_while_running_does_not_finish_the_wait() -> TestResult {
    // @step Given stream lag while the subordinate is still running
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::done())?;
    sender.send(StreamChunk::done())?;
    // @step When the waiter recovers and the subordinate subsequently becomes idle
    let checked = AtomicBool::new(false);
    let waiter = agent_manager_wait::wait(
        receiver,
        || {
            checked.store(true, Ordering::SeqCst);
            Some(false)
        },
        |_| None,
        || None,
    );
    tokio::pin!(waiter);
    // TEST-042: polled once, no waiting window; the lag is already queued.
    assert!(futures::FutureExt::now_or_never(&mut waiter).is_none());
    assert!(checked.load(Ordering::SeqCst));
    sender.send(StreamChunk::session_state_change(SessionState::Idle))?;
    // @step Then the wait resolves only after completion
    assert!(matches!(
        codelet_test_helpers::deadlock_guard("waiter", waiter).await?,
        agent_manager_wait::WaitEnd::Outcome(AwaitOutcome::Idle)
    ));
    Ok(())
}

#[tokio::test]
async fn destroyed_subordinate_resolves_after_stream_lag() -> TestResult {
    // @step Given stream lag and a subordinate that no longer exists
    let (sender, receiver) = broadcast::channel(1);
    sender.send(StreamChunk::done())?;
    sender.send(StreamChunk::done())?;
    // @step When the waiter rechecks authoritative status
    let outcome = codelet_test_helpers::deadlock_guard(
        "agent_manager_wait::wait(receiver, || None)",
        agent_manager_wait::wait(receiver, || None, |_| None, || None),
    )
    .await?;
    // @step Then the wait resolves as destroyed
    assert!(matches!(
        outcome,
        agent_manager_wait::WaitEnd::Outcome(AwaitOutcome::Destroyed)
    ));
    Ok(())
}

#[test]
fn already_idle_subordinate_needs_no_stream_event() {
    // @step Given a subordinate whose authoritative status is already idle
    let (sender, _) = broadcast::channel::<StreamChunk>(1);
    // @step When its idle wait subscribes and inspects status
    // @step Then the waiter resolves immediately
    assert!(agent_manager_wait::subscribe_before_check(|| sender.subscribe(), || true).is_none());
}

#[tokio::test]
async fn closed_stream_resolves_as_destroyed() {
    // @step Given a running subordinate with a subscribed idle waiter
    let (sender, receiver) = broadcast::channel(1);
    // @step When its stream closes before completion
    drop(sender);
    // @step Then the wait resolves as destroyed
    assert!(matches!(
        agent_manager_wait::wait(receiver, || Some(false), |_| None, || None).await,
        agent_manager_wait::WaitEnd::Outcome(AwaitOutcome::Destroyed)
    ));
}
