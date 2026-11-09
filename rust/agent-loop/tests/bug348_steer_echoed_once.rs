//! BUG-348 — a steer the running turn never reached is echoed once.
//!
//! Feature: spec/features/steers-a-turn-never-reached-are-echoed-once.feature
//!
//! Real agent loop and CLI stream loop over the offline RIG-016 wiremock
//! harness. The steer is echoed when it is submitted. When the turn ends
//! without draining it, continue_with_inbox starts it as the next turn,
//! and that turn must not echo it a second time.

#[path = "rig016_interrupt_steer_queue/harness.rs"]
pub mod harness;

use codelet_rpc_types::{InputMode, StreamChunk};
use harness::{last_user, Harness, Reply, TestResult};

const STEER: &str = "use the staging database";

/// Scenario: A steer the running turn never reached is echoed once when it starts the next turn
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_steer_the_running_turn_never_reached_is_echoed_once_when_it_starts_the_next_turn(
) -> TestResult {
    // @step Given a session whose turn is streaming a text reply with no tool call
    let mut harness = Harness::new(vec![
        Reply::Text("service deployed", 1500),
        Reply::Text("staging configured", 0),
    ])
    .await;
    harness
        .session
        .send_input("deploy the service".into(), None)?;
    harness.until_requests(1).await;

    // @step When the user steers "use the staging database" and the turn completes without reaching it
    harness
        .session
        .submit_input(STEER.into(), None, InputMode::Steer)?;
    let mut seen = harness
        .until(|chunk| matches!(chunk, StreamChunk::Done))
        .await;

    // @step Then the next turn starts with "use the staging database"
    seen.extend(
        harness
            .until(|chunk| matches!(chunk, StreamChunk::Done))
            .await,
    );
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 2, "the steer starts a second turn");
    assert_eq!(last_user(&requests[1]), STEER);

    // @step And the session emitted exactly one user input echo for "use the staging database"
    let echoes = seen
        .iter()
        .filter(|chunk| matches!(chunk, StreamChunk::UserInput { text } if text == STEER))
        .count();
    assert_eq!(echoes, 1, "each steer is echoed once, got {seen:?}");
    Ok(())
}
