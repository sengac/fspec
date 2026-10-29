use super::{
    fixture::{Harness, TestResult},
    retention,
};
use codelet_sessions::background_session::{format_incoming_message, IncomingMessage};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_six_supervisor_inputs_canceled_before_admission_remain_audit_only() -> TestResult
{
    // @step Given a genuine Codex conversation with a completed tool effect
    let mut harness = Harness::new()?;
    harness.seed().await?;
    // @step When six queued supervisor inputs are canceled before admission
    let mut originals = Vec::new();
    for index in 0..6 {
        let input = IncomingMessage::new(
            "synthetic-controller".into(),
            "operator".into(),
            format!("six-case-{index}"),
        )?;
        originals.push(format_incoming_message(&input));
        harness.session.receive_incoming_message(input)?;
    }
    assert_eq!(harness.session.pending_incoming_message_count(), 6);
    harness.session.interrupt();
    let mut events = harness.session.subscribe_to_stream();
    harness.start()?;
    let completion = retention::done(&mut events, 6).await;
    harness.stop().await;
    completion?;
    assert_eq!(harness.session.pending_incoming_message_count(), 0);
    // @step Then all six originals remain recoverable exactly once without executing
    retention::retained(&harness, &originals).await?;
    // @step And a fresh instruction completes on the unchanged runtime
    harness.fresh().await?;
    // @step And selected compaction accepts the same admitted history as runtime
    harness.compact().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_two_supervisor_inputs_submitted_during_interruption_remain_audit_only(
) -> TestResult {
    // @step Given a genuine Codex conversation with a completed tool effect
    let mut harness = Harness::new()?;
    harness.seed().await?;
    // @step When two supervisor inputs arrive while the session is interrupted
    harness.session.interrupt();
    let mut originals = Vec::new();
    for index in 0..2 {
        let input = IncomingMessage::new(
            "synthetic-controller".into(),
            "operator".into(),
            format!("two-case-{index}"),
        )?;
        originals.push(format_incoming_message(&input));
        harness.session.receive_incoming_message(input)?;
    }
    assert_eq!(harness.session.pending_incoming_message_count(), 2);
    let mut events = harness.session.subscribe_to_stream();
    harness.start()?;
    let completion = retention::done(&mut events, 2).await;
    harness.stop().await;
    completion?;
    assert_eq!(harness.session.pending_incoming_message_count(), 0);
    // @step Then both originals remain recoverable exactly once without executing
    retention::retained(&harness, &originals).await?;
    // @step And a fresh instruction completes on the unchanged runtime
    harness.fresh().await?;
    // @step And selected compaction accepts the same admitted history as runtime
    harness.compact().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_cancellation_during_a_prompt_hook_preserves_audit_without_conversation_membership(
) -> TestResult {
    // @step Given a genuine Codex conversation and a prompt held by a submission hook
    let mut harness = Harness::new()?;
    harness.seed().await?;
    retention::hook(&mut harness)?;
    let mut events = harness.session.subscribe_to_stream();
    harness
        .session
        .send_input("hook cancellation".into(), None)?;
    harness.start()?;
    retention::wait_for(&harness.directory.path().join("entered")).await?;
    // @step When the prompt is canceled before the hook releases
    harness.session.interrupt();
    std::fs::write(harness.directory.path().join("release"), "release")?;
    let completion = retention::done(&mut events, 1).await;
    harness.stop().await;
    completion?;
    // @step Then the original remains recoverable without executing
    retention::retained(&harness, &["hook cancellation".into()]).await?;
    // @step And a fresh instruction completes on the unchanged runtime
    harness.fresh().await?;
    // @step And selected compaction accepts the same admitted history as runtime
    harness.compact().await?;
    Ok(())
}
