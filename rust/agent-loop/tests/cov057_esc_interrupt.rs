//! Feature: spec/features/immediate-esc-interrupt-during-tool-execution.feature
//!
//! COV-057: the TUI/RPC Esc path is `BackgroundSession::interrupt`, which sets
//! the interrupt flag and signals `interrupt_notify`. These tests drive the
//! real `run_agent_stream` driver (the Notify select arm in
//! `rust/cli/src/interactive/stream_loop/poll.rs`) against a provider whose
//! response never finishes, so only the Notify can end the turn: without it
//! the loop stays blocked until the 600s stall timeout. The slow-tool
//! scenario interrupts a running 5-second tool the same way.

#[path = "cov057_esc_interrupt/fixtures.rs"]
mod fixtures;
#[path = "support/bug220.rs"]
pub mod support;

use codelet_cli::interactive::run_agent_stream;
use codelet_core::RigAgent;
use codelet_providers::stub_model::StubModel;
use fixtures::{HangingModel, SlowTool};
use rig::message::{AssistantContent, Message};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use support::{fixture, TestResult};
use tokio::sync::Notify;

struct Interrupted {
    chunks: Vec<Value>,
    history: Vec<Message>,
    ended_on_flag_alone: bool,
}

/// Runs one turn against `HangingModel` and presses Esc
/// (`BackgroundSession::interrupt`) once the driver is blocked on the
/// provider. With `probe_flag_alone`, the interrupt flag is first set WITHOUT
/// the Notify for 500ms to show the blocked select does not wake on it.
async fn press_esc_while_blocked(
    partial: Option<&'static str>,
    probe_flag_alone: bool,
) -> Result<Interrupted, Box<dyn std::error::Error>> {
    let (_directory, session, output) = fixture().await?;
    let model = HangingModel {
        partial,
        reached: Arc::new(Notify::new()),
    };
    let reached = Arc::clone(&model.reached);
    let agent = rig::agent::AgentBuilder::new(model).build();
    let finished = AtomicBool::new(false);
    let mut inner = session.inner.lock().await;
    let run = async {
        let result = run_agent_stream(
            RigAgent::with_default_depth(agent),
            "Explain the change",
            &mut inner,
            session.is_interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            session.interrupt_notify.clone(),
            output.as_ref(),
            session.id,
        )
        .await;
        finished.store(true, Ordering::Release);
        result
    };
    let esc = async {
        reached.notified().await;
        let mut ended_on_flag_alone = false;
        if probe_flag_alone {
            session.is_interrupted.store(true, Ordering::Release);
            // TEST-042: no waiting window. Hand the turn to the run a few
            // times; the flag alone must not end it.
            for _ in 0..16 {
                tokio::task::yield_now().await;
            }
            ended_on_flag_alone = finished.load(Ordering::Acquire);
        }
        session.interrupt();
        ended_on_flag_alone
    };
    let (result, ended_on_flag_alone) =
        codelet_test_helpers::deadlock_guard("the awaited block", async { tokio::join!(run, esc) })
            .await?;
    result?;
    let history = inner.messages.clone();
    drop(inner);
    let chunks = session
        .get_buffered_output(10_000)
        .iter()
        .map(codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value)
        .collect();
    Ok(Interrupted {
        chunks,
        history,
        ended_on_flag_alone,
    })
}

/// Index of the single user interruption event, asserting it is followed by
/// the turn's `done`.
fn user_interruption(chunks: &[Value]) -> usize {
    let interrupted: Vec<_> = chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| chunk["type"] == "interrupted")
        .collect();
    assert_eq!(interrupted.len(), 1, "{chunks:?}");
    let (index, event) = interrupted[0];
    assert_eq!(event["reason"], "user", "{event}");
    assert!(
        chunks[index + 1..]
            .iter()
            .any(|chunk| chunk["type"] == "done"),
        "interruption must settle with done: {chunks:?}"
    );
    index
}

/// BUG-524: the turn against a provider that never completes ended (inside
/// the hang guard) on the user interrupt and not on an error such as the
/// stall timeout.
fn ended_on_the_interrupt(chunks: &[Value]) {
    assert!(
        !chunks.iter().any(|chunk| chunk["type"] == "error"),
        "the turn must end on the interrupt, not an error: {chunks:?}"
    );
    user_interruption(chunks);
}

fn streamed_text(chunks: &[Value]) -> String {
    chunks
        .iter()
        .filter(|chunk| chunk["type"] == "text")
        .filter_map(|chunk| chunk["text"].as_str())
        .collect()
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_interrupt_agent_during_text_streaming() -> TestResult {
    // @step Given the agent is streaming a text response
    let partial = "Partial answer: the change";
    // @step When I press ESC
    let run = press_esc_while_blocked(Some(partial), false).await?;
    // @step Then the agent should stop on the interrupt without waiting for the stalled stream
    ended_on_the_interrupt(&run.chunks);
    // @step And I should see an interrupted event with reason "user"
    let interrupted = user_interruption(&run.chunks);
    // @step And partial text should be preserved in conversation history
    assert_eq!(streamed_text(&run.chunks[..interrupted]), partial);
    match run.history.last() {
        Some(Message::Assistant { content, .. }) => {
            let text: String = content
                .iter()
                .filter_map(|item| match item {
                    AssistantContent::Text(text) => Some(text.text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, partial);
        }
        other => return Err(format!("partial reply missing from history: {other:?}").into()),
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_interrupt_agent_waiting_for_api_response() -> TestResult {
    // @step Given the agent is waiting for an API response
    let partial = None;
    // @step When I press ESC
    let run = press_esc_while_blocked(partial, true).await?;
    // @step Then the tokio select should wake via Notify
    assert!(
        !run.ended_on_flag_alone,
        "the blocked select must not end on the interrupt flag alone"
    );
    // @step And the stream loop should break on the interrupt without waiting for the stalled response
    ended_on_the_interrupt(&run.chunks);
    assert!(streamed_text(&run.chunks).is_empty());
    assert!(
        matches!(run.history.last(), Some(Message::User { .. })),
        "no assistant reply may be invented: {:?}",
        run.history
    );
    // @step And I should see an interrupted event with reason "user"
    user_interruption(&run.chunks);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_interrupt_agent_during_slow_tool_execution() -> TestResult {
    // @step Given the agent is executing a tool call that takes 5 seconds
    let (directory, session, output) = fixture().await?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.id, directory.path().into())?;
    let started = Arc::new(Notify::new());
    let call = serde_json::json!({"id":"slow-check","name":"SlowCheck","arguments":{"work_unit_id":"NONE"}});
    let agent = rig::agent::AgentBuilder::new(StubModel::new())
        .additional_params(serde_json::json!({ "stub_tool_call": call }))
        .tool(SlowTool(Arc::clone(&started)))
        .build();
    let mut inner = session.inner.lock().await;
    let run = run_agent_stream(
        RigAgent::with_default_depth(agent),
        "Run the slow check",
        &mut inner,
        session.is_interrupted.clone(),
        Arc::new(AtomicBool::new(false)),
        session.interrupt_notify.clone(),
        output.as_ref(),
        session.id,
    );
    let esc = async {
        started.notified().await;
        // @step When I press ESC after 1 second
        // TEST-042: Esc comes once the tool is running, not after a fixed time.
        session.interrupt();
    };
    let (result, ()) =
        codelet_test_helpers::deadlock_guard("the awaited block", async { tokio::join!(run, esc) })
            .await?;
    result?;
    // @step Then the agent should stop before the tool finishes
    // The tool still had 4 seconds to run: its result never reaching history
    // shows Esc ended it (BUG-524: not a wall-clock bound).
    let history = serde_json::to_string(&inner.messages)?;
    assert!(
        history.contains("cancelled by user interruption"),
        "{history}"
    );
    assert!(!history.contains("slow check finished"), "{history}");
    let chunks: Vec<Value> = session
        .get_buffered_output(10_000)
        .iter()
        .map(codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value)
        .collect();
    // @step And I should see an interrupted event with reason "user"
    user_interruption(&chunks);
    // @step And partial response should be preserved in conversation history
    assert!(
        history.contains("slow-check"),
        "tool call missing: {history}"
    );
    Ok(())
}
