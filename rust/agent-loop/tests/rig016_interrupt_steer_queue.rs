//! RIG-016: steer, queue and interrupt input to a busy agent.
//!
//! Feature: spec/features/interrupt-steer-and-queue-input.feature
//!
//! Real agent loop and CLI stream loop over an offline OpenAI-compatible
//! wiremock endpoint; assertions read the provider requests it receives.

#[path = "rig016_interrupt_steer_queue/harness.rs"]
pub mod harness;

use codelet_rpc_types::{InputMode, StreamChunk};
use harness::{last_user, mentions, messages, Harness, Reply, TestResult};

fn queued_of(chunk: &StreamChunk) -> Option<Vec<String>> {
    match chunk {
        StreamChunk::Interrupted { queued_inputs }
        | StreamChunk::InterruptedWithReason { queued_inputs, .. } => Some(queued_inputs.clone()),
        _ => None,
    }
}

/// A running turn with `queued` follow-ups, then an interrupt; returns the
/// interrupted notice's queued inputs.
async fn interrupted_with(harness: &mut Harness, queued: &[&str]) -> Vec<String> {
    harness
        .session
        .send_input("long running task".into(), None)
        .expect("start the turn");
    harness.until_requests(1).await;
    for text in queued {
        harness
            .session
            .submit_input((*text).into(), None, InputMode::Queue)
            .expect("queue a follow-up");
    }
    harness.session.interrupt();
    let seen = harness.until(|chunk| queued_of(chunk).is_some()).await;
    seen.iter().rev().find_map(queued_of).unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupting_keeps_queued_inputs_parked() -> TestResult {
    // @step Given a session whose turn is running with two queued inputs
    let mut harness = Harness::new(vec![Reply::Text("never finishes", 4000)]).await;

    // @step When the user interrupts the turn
    let notice = interrupted_with(&mut harness, &["fix the tests", "update the docs"]).await;

    // @step Then the turn stops and the interrupted notice lists both queued inputs
    assert_eq!(notice, ["fix the tests", "update the docs"]);
    harness.until_done(1).await;

    // @step And neither queued input is sent
    // TEST-042: no waiting window. The turn has ended; a queued input that
    // was sent would have left the queue, and both are still parked.
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 1, "no follow-up turn after the interrupt");
    assert!(!mentions(&requests[0], "fix the tests"));
    assert_eq!(
        harness.session.queued_inputs(),
        ["fix the tests", "update the docs"]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn resuming_sends_the_parked_inputs() -> TestResult {
    // @step Given an interrupted session with a parked queued input
    let mut harness = Harness::new(vec![
        Reply::Text("never finishes", 4000),
        Reply::Text("resumed reply", 0),
    ])
    .await;
    let notice = interrupted_with(&mut harness, &["carry on with the migration"]).await;
    assert_eq!(notice, ["carry on with the migration"]);
    harness.until_done(1).await;

    // @step When the user resumes the session
    assert!(harness.session.resume_queued()?);

    // @step Then the parked input starts as a new turn
    harness.until_done(1).await;
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(last_user(&requests[1]), "carry on with the migration");
    assert!(harness.session.queued_inputs().is_empty());
    Ok(())
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_steered_input_reaches_the_next_provider_request_after_the_tool_result() -> TestResult {
    // BUG-447: this test drives tools without a session manager callback.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a session whose turn is running a tool call
    let mut harness = Harness::new(vec![
        Reply::Bash("sleep 2; echo tool-finished"),
        Reply::Text("steered reply", 0),
    ])
    .await;
    harness
        .session
        .send_input("deploy the service".into(), None)?;
    harness.until_requests(1).await;
    // TEST-042: wait until the tool call is running, not a fixed time.
    let session = harness.session.id;
    codelet_test_helpers::wait_until("the tool call is running", || {
        std::future::ready(codelet_tools::tool_pause::active_tool_call(session).is_some())
    })
    .await?;

    // @step When the user steers "use the staging database"
    harness
        .session
        .submit_input("use the staging database".into(), None, InputMode::Steer)?;
    harness.until_done(1).await;

    // @step Then the tool result is recorded before the steered input
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 2, "one turn, two provider requests");
    assert!(!mentions(&requests[0], "use the staging database"));
    let next = messages(&requests[1]);
    let tool = next.iter().position(|(role, _)| role == "tool");
    let steer = next
        .iter()
        .position(|(role, text)| role == "user" && text.contains("use the staging database"));
    assert!(
        matches!((tool, steer), (Some(tool), Some(steer)) if tool < steer),
        "tool result must precede the steer: {next:?}"
    );

    // @step And the next provider request of the same turn carries "use the staging database"
    assert_eq!(last_user(&requests[1]), "use the staging database");
    assert_eq!(
        next.iter()
            .filter(|(_, text)| text.contains("deploy the service"))
            .count(),
        1,
        "the steer joins the running turn rather than starting a new one"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_queued_input_starts_a_new_turn_after_the_running_turn_completes() -> TestResult {
    // @step Given a session whose turn is running
    let mut harness = Harness::new(vec![
        Reply::Text("parser refactored", 1500),
        Reply::Text("changelog updated", 0),
    ])
    .await;
    harness
        .session
        .send_input("refactor the parser".into(), None)?;
    harness.until_requests(1).await;

    // @step When the user queues "then update the changelog"
    harness
        .session
        .submit_input("then update the changelog".into(), None, InputMode::Queue)?;
    assert_eq!(
        harness.session.queued_inputs(),
        ["then update the changelog"]
    );

    // @step Then the running turn does not receive the queued input
    harness.until_done(1).await;
    let requests = harness.requests().await;
    assert!(!mentions(&requests[0], "then update the changelog"));

    // @step And when the turn completes a new turn starts with "then update the changelog"
    harness.until_done(1).await;
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(last_user(&requests[1]), "then update the changelog");
    assert!(harness.session.queued_inputs().is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn queued_inputs_start_in_submission_order() -> TestResult {
    // @step Given a session whose turn is running
    let mut harness = Harness::new(vec![
        Reply::Text("working", 1500),
        Reply::Text("first handled", 0),
        Reply::Text("second handled", 0),
    ])
    .await;
    harness.session.send_input("start the work".into(), None)?;
    harness.until_requests(1).await;

    // @step When the user queues "first follow-up" and then "second follow-up"
    for text in ["first follow-up", "second follow-up"] {
        harness
            .session
            .submit_input(text.into(), None, InputMode::Queue)?;
    }

    // @step Then the follow-ups start as separate turns in that order
    harness.until_done(3).await;
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(last_user(&requests[1]), "first follow-up");
    assert!(!mentions(&requests[1], "second follow-up"));
    assert_eq!(last_user(&requests[2]), "second follow-up");
    Ok(())
}
