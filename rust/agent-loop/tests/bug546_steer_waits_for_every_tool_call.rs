//! BUG-546: a steer is delivered only when the model's response has finished
//! and every tool call it requested has its real result.
//!
//! Feature: spec/features/interrupt-steer-and-queue-input.feature
//!
//! rig runs each tool call as soon as it streams, so the first call's result
//! can arrive while the same response still has calls to run. Delivering the
//! steer there replaced the stream: Codex then refused the replay ('Codex
//! replay has no recorded outcome for call ...') and the other providers
//! silently lost the remaining calls. Real agent loop and CLI stream loop
//! over an offline OpenAI-compatible endpoint. The first call blocks on a
//! gate file the test releases only after the steer is submitted, so no
//! timing guess is involved.

#[path = "rig016_interrupt_steer_queue/harness.rs"]
pub mod harness;

use codelet_rpc_types::{InputMode, StreamChunk};
use harness::{last_user, messages, Harness, Reply, TestResult};

/// The ids of every tool call the assistant messages of `body` carry.
fn tool_call_ids(body: &serde_json::Value) -> Vec<String> {
    body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|message| {
            message["tool_calls"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|call| call["id"].as_str().map(str::to_owned))
        .collect()
}

/// Scenario: A steer waits for every tool call of the response before it is delivered
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_steer_waits_for_every_tool_call_of_the_response() -> TestResult {
    // BUG-447: this test drives tools without a session manager callback.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a session whose model response requested two tool calls and the first is still running
    let gate_directory = tempfile::tempdir()?;
    let gate = gate_directory.path().join("release");
    let first: &'static str = Box::leak(
        format!(
            "while [ ! -f '{}' ]; do sleep 0.05; done; echo first-finished",
            gate.display()
        )
        .into_boxed_str(),
    );
    let mut harness = Harness::new(vec![
        Reply::BashPair(first, "echo second-finished"),
        Reply::Text("steered reply", 0),
    ])
    .await;
    harness
        .session
        .send_input("inspect the project".into(), None)?;
    harness
        .until(|chunk| matches!(chunk, StreamChunk::ToolCall { .. }))
        .await;

    // @step When the user steers "use the staging database"
    harness
        .session
        .submit_input("use the staging database".into(), None, InputMode::Steer)?;
    std::fs::write(&gate, "")?;
    harness.until_done(1).await;

    // @step Then both tool calls run and both real results are recorded
    let requests = harness.requests().await;
    assert_eq!(requests.len(), 2, "one turn, two provider requests");
    let next = messages(&requests[1]);
    let results: Vec<&str> = next
        .iter()
        .filter(|(role, _)| role == "tool")
        .map(|(_, text)| text.as_str())
        .collect();
    assert_eq!(results.len(), 2, "both results are recorded: {next:?}");
    assert!(results.iter().any(|text| text.contains("first-finished")));
    assert!(results.iter().any(|text| text.contains("second-finished")));

    // @step And the next provider request carries both tool calls and both results before "use the staging database"
    assert_eq!(
        tool_call_ids(&requests[1]),
        ["bug546-call-1", "bug546-call-2"],
        "no requested call is dropped"
    );
    let last_result = next.iter().rposition(|(role, _)| role == "tool");
    let steer = next
        .iter()
        .position(|(role, text)| role == "user" && text.contains("use the staging database"));
    assert!(
        matches!((last_result, steer), (Some(result), Some(steer)) if result < steer),
        "both results precede the steer: {next:?}"
    );
    assert_eq!(last_user(&requests[1]), "use the staging database");

    // @step And the turn continues in that request
    let inner = harness.session.inner.lock().await;
    let conversation = serde_json::to_string(&inner.messages)?;
    assert!(
        conversation.contains("steered reply"),
        "the steered request's reply ends the turn"
    );
    Ok(())
}
