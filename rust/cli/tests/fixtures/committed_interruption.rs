use crate::{interruption_tool, transport};
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::codex::custom_tools::{CustomToolRegistry, ToolRegistration};
use codelet_providers::codex::session::CodexSession;
use codelet_providers::codex::CodexResponsesModel;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::Message;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use transport::TestResult;

pub fn expected(index: usize) -> (&'static str, Value) {
    match index {
        0 => ("inspect", json!({"path":"first.rs"})),
        1 => ("search", json!({"pattern":"needle"})),
        _ => (
            "apply_patch",
            json!({"patch":"*** Begin Patch\n*** End Patch"}),
        ),
    }
}

pub struct Batch {
    pub execution: interruption_tool::Execution,
    pub interrupted: Arc<AtomicBool>,
    pub notify: Arc<Notify>,
    model: CodexResponsesModel,
    _server: transport::Server,
}

impl Batch {
    pub async fn new(
        session: &Session,
        completed: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut calls: Vec<Value> = (0..2)
            .map(|index| {
                let (name, arguments) = expected(index);
                json!({"type":"function_call", "id":format!("item-{index}"),
                    "call_id":format!("call-{index}"), "name":name,
                    "arguments":arguments.to_string()})
            })
            .collect();
        calls[1]["namespace"] = json!("files");
        calls[1]["name"] = json!("inspect");
        calls.push(
            json!({"type":"custom_tool_call", "id":"item-2", "call_id":"call-2",
            "namespace":"editing", "name":"patch_text", "input":expected(2).1["patch"]}),
        );
        let response = transport::event(json!({"type":"response.completed","response":{
            "id":"batch-response", "status":"completed", "output":calls,"end_turn":true,
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
        }}));
        let server = transport::Server::start(
            vec![response, transport::terminal("completed", None, None)],
            false,
        )
        .await?;
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
                .with_custom_tools(CustomToolRegistry::new(vec![
                    ToolRegistration {
                        namespace: Some("editing".into()),
                        name: "patch_text".into(),
                        executor: "apply_patch".into(),
                        grammar: None,
                        raw_input: true,
                    },
                    ToolRegistration {
                        namespace: Some("files".into()),
                        name: "inspect".into(),
                        executor: "search".into(),
                        grammar: None,
                        raw_input: false,
                    },
                ])?);
        let interrupted = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());
        let execution = interruption_tool::Execution {
            completed: Arc::new(AtomicUsize::new(0)),
            started: Arc::new(AtomicUsize::new(0)),
            interrupted: interrupted.clone(),
            notify: notify.clone(),
            interrupt_at: Some(completed),
        };
        Ok(Self {
            execution,
            interrupted,
            notify,
            model,
            _server: server,
        })
    }

    pub async fn run<O: StreamOutput>(
        &self,
        session: &mut Session,
        session_id: uuid::Uuid,
        output: &O,
    ) -> TestResult {
        let agent = rig::agent::AgentBuilder::new(self.model.clone())
            .preamble("Test instructions")
            .tool(interruption_tool::RecordingTool {
                name: "inspect",
                execution: self.execution.clone(),
            })
            .tool(interruption_tool::RecordingTool {
                name: "search",
                execution: self.execution.clone(),
            })
            .tool(interruption_tool::RecordingTool {
                name: "apply_patch",
                execution: self.execution.clone(),
            })
            .build();
        tokio::time::timeout(
            Duration::from_secs(10),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "inspect",
                session,
                self.interrupted.clone(),
                Arc::new(AtomicBool::new(false)),
                self.notify.clone(),
                output,
                session_id,
            ),
        )
        .await??;
        Ok(())
    }

    pub async fn assert_replay(&self, messages: Vec<Message>) -> TestResult {
        let request = CompletionRequestBuilder::new(self.model.clone(), "continue")
            .messages(messages)
            .build();
        let mut stream = self.model.stream(request).await?;
        tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(chunk) = stream.next().await {
                chunk?;
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await??;
        Ok(())
    }
}

pub fn assert_pairs(events: &[StreamEvent], completed: usize) {
    let calls: Vec<_> = events
        .iter()
        .enumerate()
        .filter_map(|(position, event)| match event {
            StreamEvent::ToolCall(call) => Some((position, call)),
            _ => None,
        })
        .collect();
    let results: Vec<_> = events
        .iter()
        .enumerate()
        .filter_map(|(position, event)| match event {
            StreamEvent::ToolResult(result) => Some((position, result)),
            _ => None,
        })
        .collect();
    assert_eq!(
        calls.len(),
        3,
        "every committed call must be published once"
    );
    assert_eq!(results.len(), 3);
    for (index, ((call_position, call), (result_position, result))) in
        calls.iter().zip(&results).enumerate()
    {
        assert!(call_position < result_position);
        if let Some((next_call_position, _)) = calls.get(index + 1) {
            assert!(
                result_position < next_call_position,
                "preserve hook attribution"
            );
        }
        assert_eq!(call.id, format!("item-{index}"));
        assert_eq!(
            call.call_id.as_deref(),
            Some(format!("call-{index}").as_str())
        );
        assert_eq!((call.name.as_str(), call.args.clone()), expected(index));
        assert_eq!(result.id, call.id);
        assert_eq!(result.call_id, call.call_id);
        assert_eq!(result.is_error, index >= completed);
        if index < completed {
            assert_eq!(result.content, json!({"completed":true}).to_string());
        } else {
            assert!(result.content.contains("cancelled"));
        }
    }
    assert!(events
        .iter()
        .position(|event| matches!(event, StreamEvent::Interrupted(_)))
        .is_some_and(|interruption| results.iter().all(|(position, _)| *position < interruption)));
    assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
}
