//! Feature: spec/features/codex-interruption-outcome-recording.feature

#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

#[path = "fixtures/interruption_tool.rs"]
mod interruption_tool;

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::codex::custom_tools::{CustomToolRegistry, ToolRegistration};
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{Message, ToolResultContent, UserContent};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::Notify;
use transport::TestResult;

struct InterruptingOutput {
    interrupted: Arc<AtomicBool>,
    stop_at: usize,
    events: Mutex<Vec<StreamEvent>>,
}

impl StreamOutput for InterruptingOutput {
    fn emit(&self, event: StreamEvent) {
        let mut events = self.events.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(event, StreamEvent::ToolCall(_))
            && events
                .iter()
                .filter(|event| matches!(event, StreamEvent::ToolCall(_)))
                .count()
                + 1
                == self.stop_at
        {
            self.interrupted.store(true, Ordering::Release);
        }
        events.push(event);
    }
}

fn results(messages: &[Message]) -> Vec<(String, String)> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => Some(content.iter()),
            _ => None,
        })
        .flatten()
        .filter_map(|content| match content {
            UserContent::ToolResult(result) => {
                let text = result
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        ToolResultContent::Text(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                Some((result.call_id.clone().unwrap_or(result.id.clone()), text))
            }
            _ => None,
        })
        .collect()
}

/// @step Given a Codex response commits three <dialect> tool calls
/// @step And the first <completed> tools finish before interruption
/// @step When I interrupt the next tool <timing>
/// @step Then every unfinished committed call has exactly one recorded cancellation
/// @step And completed tool results remain unchanged
/// @step And cancellation results are published before interruption and completion
/// @step And the next request succeeds with the original invocation identities
#[tokio::test]
#[serial_test::serial]
async fn interrupted_batches_retain_completed_results_and_cancel_every_remaining_call() -> TestResult
{
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    for (custom, completed, executing) in [
        (false, 0, false),
        (false, 1, false),
        (true, 0, false),
        (true, 1, false),
        (false, 0, true),
        (false, 1, true),
        (true, 0, true),
        (true, 1, true),
    ] {
        let manifest = codelet_core::persistence::create_session_with_provider(
            "interrupted tools",
            directory.path(),
            "codex",
        )?;
        let calls: Vec<Value> = (1..=3)
            .map(|number| {
                if custom {
                    json!({"type":"custom_tool_call","id":format!("item-{number}"),
                        "call_id":format!("call-{number}"),"name":"apply_patch","input":"patch"})
                } else {
                    json!({"type":"function_call","id":format!("item-{number}"),
                        "call_id":format!("call-{number}"),"name":"inspect","arguments":"{}"})
                }
            })
            .collect();
        let response = transport::event(json!({"type":"response.completed","response":{
            "id":"response-1","status":"completed","output":calls,"end_turn":true,
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
        }}));
        let server = transport::Server::start(
            vec![response, transport::terminal("completed", None, None)],
            false,
        )
        .await?;
        let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
        let mut session = Session::from_provider_manager(manager);
        let history = session
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("Codex session missing")?
            .history
            .clone();
        let model =
            server
                .models()?
                .0
                .with_history(history)
                .with_custom_tools(CustomToolRegistry::new(vec![ToolRegistration {
                    namespace: None,
                    name: "apply_patch".into(),
                    executor: "apply_patch".into(),
                    grammar: None,
                    raw_input: true,
                }])?);
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupt_notify = Arc::new(Notify::new());
        let execution = interruption_tool::Execution {
            completed: Arc::new(AtomicUsize::new(0)),
            started: Arc::new(AtomicUsize::new(0)),
            interrupted: interrupted.clone(),
            notify: interrupt_notify.clone(),
            interrupt_at: executing.then_some(completed),
        };
        let agent = rig::agent::AgentBuilder::new(model.clone())
            .preamble("Test instructions")
            .tool(interruption_tool::RecordingTool {
                name: "inspect",
                execution: execution.clone(),
            })
            .tool(interruption_tool::RecordingTool {
                name: "apply_patch",
                execution: execution.clone(),
            })
            .build();
        let output = InterruptingOutput {
            interrupted: interrupted.clone(),
            stop_at: if executing { usize::MAX } else { completed + 1 },
            events: Mutex::new(Vec::new()),
        };
        tokio::time::timeout(
            Duration::from_secs(10),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "inspect",
                &mut session,
                interrupted,
                Arc::new(AtomicBool::new(false)),
                interrupt_notify,
                &output,
                manifest.id,
            ),
        )
        .await??;
        assert_eq!(execution.completed.load(Ordering::SeqCst), completed);
        assert_eq!(
            execution.started.load(Ordering::SeqCst),
            completed + usize::from(executing)
        );
        let outcomes = results(&session.messages);
        assert_eq!(outcomes.len(), 3, "completed={completed}: {outcomes:?}");
        for (index, (identity, text)) in outcomes.iter().enumerate() {
            assert_eq!(identity, &format!("call-{}", index + 1));
            assert_eq!(text.contains("cancelled"), index >= completed);
            if index < completed {
                assert_eq!(
                    serde_json::from_str::<Value>(text)?,
                    json!({"completed":true})
                );
            }
        }
        {
            let events = output.events.lock().map_err(|_| "output poisoned")?;
            let interruption = events
                .iter()
                .position(|event| matches!(event, StreamEvent::Interrupted(_)))
                .ok_or("interruption missing")?;
            assert_eq!(
                events[..interruption]
                    .iter()
                    .filter(|event| matches!(event, StreamEvent::ToolResult(_)))
                    .count(),
                3
            );
            assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
        }
        let request = CompletionRequestBuilder::new(model.clone(), "continue")
            .messages(session.messages.clone())
            .build();
        let mut stream = model.stream(request).await?;
        while let Some(event) = stream.next().await {
            event?;
        }
        let requests = server.requests.lock().map_err(|_| "requests poisoned")?;
        let input = requests.get(1).ok_or("continuation missing")?["input"]
            .as_array()
            .ok_or("input missing")?;
        for (identity, text) in outcomes {
            let kind = if custom {
                "custom_tool_call_output"
            } else {
                "function_call_output"
            };
            let recorded: Vec<_> = input
                .iter()
                .filter(|item| item["type"] == kind && item["call_id"] == identity)
                .collect();
            assert_eq!(recorded.len(), 1);
            assert_eq!(recorded[0]["output"], text);
        }
    }
    Ok(())
}

/// @step Given a Codex session has no committed tool call
/// @step When I interrupt its streaming consumer
/// @step Then no tool cancellation is recorded or published
#[tokio::test]
#[serial_test::serial]
async fn interrupting_before_any_committed_tool_call_creates_no_cancellation() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let server = transport::Server::start(Vec::new(), false).await?;
    let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
    let mut session = Session::from_provider_manager(manager);
    let interrupted = Arc::new(AtomicBool::new(true));
    let output = InterruptingOutput {
        interrupted: interrupted.clone(),
        stop_at: 1,
        events: Mutex::new(Vec::new()),
    };
    let agent = rig::agent::AgentBuilder::new(server.models()?.0).build();
    run_agent_stream(
        RigAgent::with_default_depth(agent),
        "inspect",
        &mut session,
        interrupted,
        Arc::new(AtomicBool::new(false)),
        Arc::new(Notify::new()),
        &output,
        uuid::Uuid::new_v4(),
    )
    .await?;
    assert!(results(&session.messages).is_empty());
    assert!(!output
        .events
        .lock()
        .map_err(|_| "output poisoned")?
        .iter()
        .any(|event| matches!(event, StreamEvent::ToolResult(_))));
    assert!(server
        .requests
        .lock()
        .map_err(|_| "requests poisoned")?
        .is_empty());
    Ok(())
}
