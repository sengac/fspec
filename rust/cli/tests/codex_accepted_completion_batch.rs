//! Feature: spec/features/codex-accepted-completion-batch.feature

#[path = "fixtures/interruption_tool.rs"]
mod interruption_tool;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::{create_session_with_provider, reset_stores_for_tests};
use codelet_core::RigAgent;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use codelet_tools::DoneTool;
use rig::message::{AssistantContent, Message, ToolResultContent, UserContent};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Notify;
use transport::TestResult;

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

fn outcomes(messages: &[Message]) -> Vec<(String, String, String)> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => Some(content.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|content| match content {
            UserContent::ToolResult(result) => Some((
                result.id.clone(),
                result.call_id.clone().unwrap_or_default(),
                result
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        ToolResultContent::Text(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

/// @step Given a Codex response commits an accepted done call followed by another tool call
/// @step When the streaming consumer accepts completion from the real done tool
/// @step Then the remaining tool never executes
/// @step And the accepted done result remains unchanged
/// @step And the remaining call and its accepted completion cancellation are published before done
/// @step And no user interruption is reported
/// @step And the next turn succeeds with both original invocation identities and outcomes
#[tokio::test]
#[serial_test::serial]
async fn accepted_completion_cancels_the_unexecuted_remainder_of_a_committed_batch() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let manifest = create_session_with_provider("accepted completion", directory.path(), "codex")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        manifest.id,
        directory.path().to_path_buf(),
    )?;
    let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
    let mut session = Session::from_provider_manager(manager);
    let history = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("Codex session missing")?
        .history
        .clone();
    let response = transport::event(json!({"type":"response.completed","response":{
        "id":"accepted-batch", "status":"completed", "end_turn":true,
        "output":[
            {"type":"function_call", "id":"done-item", "call_id":"done-call",
             "name":"Done", "arguments":json!({"work_unit_id":"NONE",
                "summary":"Task complete"}).to_string()},
            {"type":"function_call", "id":"inspect-item", "call_id":"inspect-call",
             "name":"inspect", "arguments":json!({"work_unit_id":"NONE",
                "path":"untouched.rs"}).to_string()}
        ], "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
    }}));
    let server = transport::Server::start(
        vec![response, transport::terminal("completed", None, None)],
        false,
    )
    .await?;
    let model = server.models()?.0.with_history(history);
    let interrupted = Arc::new(AtomicBool::new(false));
    let notify = Arc::new(Notify::new());
    let execution = interruption_tool::Execution {
        completed: Arc::new(AtomicUsize::new(0)),
        started: Arc::new(AtomicUsize::new(0)),
        interrupted: interrupted.clone(),
        notify: notify.clone(),
        interrupt_at: None,
    };
    let agent = rig::agent::AgentBuilder::new(model.clone())
        .preamble("Test instructions")
        .tool(DoneTool::new(manifest.id))
        .tool(interruption_tool::RecordingTool {
            name: "inspect",
            execution: execution.clone(),
        })
        .build();
    let output = RecordingOutput::default();
    codelet_tools::set_continue_armed(manifest.id, true);
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "Finish the task",
            &mut session,
            interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            notify,
            &output,
            manifest.id,
        ),
    )
    .await;
    codelet_tools::set_continue_armed(manifest.id, false);
    result??;
    assert_eq!(execution.started.load(Ordering::SeqCst), 0);
    assert_eq!(execution.completed.load(Ordering::SeqCst), 0);
    assert!(!interrupted.load(Ordering::Acquire));
    let recorded = outcomes(&session.messages);
    assert_eq!(
        recorded.len(),
        2,
        "committed calls need outcomes: {recorded:?}"
    );
    assert_eq!(
        (&*recorded[0].0, &*recorded[0].1),
        ("done-item", "done-call")
    );
    assert_eq!(
        recorded[0].2,
        "Completion recorded. The turn will finish with your summary."
    );
    assert_eq!(
        (&*recorded[1].0, &*recorded[1].1),
        ("inspect-item", "inspect-call")
    );
    assert!(recorded[1].2.contains("cancelled"));
    assert!(recorded[1].2.contains("accepted completion"));
    assert!(!recorded[1].2.contains("user interruption"));
    let calls: Vec<_> = session
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::Assistant { content, .. } => Some(content.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|content| match content {
            AssistantContent::ToolCall(call) => Some(call),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].id, "inspect-item");
    assert_eq!(calls[1].call_id.as_deref(), Some("inspect-call"));
    assert_eq!(calls[1].function.name, "inspect");
    assert_eq!(
        calls[1].function.arguments,
        json!({"work_unit_id":"NONE","path":"untouched.rs"})
    );
    {
        let events = output.0.lock().map_err(|_| "output poisoned")?;
        let pairs: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolCall(call) => Some(("call", call.id.as_str())),
                StreamEvent::ToolResult(result) => Some(("result", result.id.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("call", "done-item"),
                ("result", "done-item"),
                ("call", "inspect-item"),
                ("result", "inspect-item")
            ]
        );
        for event in events.iter() {
            if let StreamEvent::ToolResult(result) = event {
                let expected = recorded
                    .iter()
                    .find(|(id, _, _)| id == &result.id)
                    .ok_or("unexpected tool result")?;
                assert_eq!(result.call_id.as_deref(), Some(expected.1.as_str()));
                assert_eq!(result.content, expected.2);
                assert_eq!(result.is_error, result.id == "inspect-item");
            }
        }
        assert!(!events
            .iter()
            .any(|event| matches!(event, StreamEvent::Interrupted(_))));
        assert!(matches!(events.last(), Some(StreamEvent::Done(Some(reason))) if reason == "done"));
        assert!(events
            .iter()
            .any(|event| matches!(event, StreamEvent::Status(status)
            if status == "✓ done: Task complete")));
    }
    let next_output = RecordingOutput::default();
    let next_agent = rig::agent::AgentBuilder::new(model)
        .preamble("Test instructions")
        .build();
    tokio::time::timeout(
        Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(next_agent),
            "Continue the conversation",
            &mut session,
            interrupted,
            Arc::new(AtomicBool::new(false)),
            Arc::new(Notify::new()),
            &next_output,
            manifest.id,
        ),
    )
    .await??;
    assert!(next_output
        .0
        .lock()
        .map_err(|_| "output poisoned")?
        .iter()
        .any(|event| matches!(event, StreamEvent::Done(_))));
    let requests = server.requests.lock().map_err(|_| "requests poisoned")?;
    assert_eq!(requests.len(), 2);
    let initial_tools = requests[0]["tools"]
        .as_array()
        .ok_or("initial tools missing")?;
    assert_eq!(initial_tools.len(), 2);
    for tool in initial_tools {
        assert!(
            tool["parameters"]["required"]
                .as_array()
                .is_some_and(|required| required.contains(&json!("work_unit_id"))),
            "{tool}"
        );
        assert_eq!(
            tool["parameters"]["properties"]["work_unit_id"]["type"],
            "string"
        );
    }
    let input = requests[1]["input"]
        .as_array()
        .ok_or("next input missing")?;
    for (_, call_id, text) in recorded {
        let outputs: Vec<&Value> = input
            .iter()
            .filter(|item| item["type"] == "function_call_output" && item["call_id"] == call_id)
            .collect();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0]["output"], text);
    }
    Ok(())
}
