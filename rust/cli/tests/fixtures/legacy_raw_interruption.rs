use crate::transport;
use codelet_cli::{
    interactive::{run_agent_stream, StreamEvent, StreamOutput},
    session::Session,
};
use codelet_core::RigAgent;
use codelet_providers::{
    codex::{
        custom_tools::{CustomToolRegistry, ToolRegistration},
        session::CodexSession,
    },
    ProviderManager, ProviderType,
};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition},
    message::{AssistantContent, Message},
};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

pub async fn assert_legacy_raw_replay(root: &std::path::Path) -> transport::TestResult {
    for completed in [0, 1] {
        let manifest =
            codelet_core::persistence::create_session_with_provider("legacy raw", root, "codex")?;
        let mut session = Session::from_provider_manager(ProviderManager::for_testing(
            ProviderType::Codex,
            Some(272_000),
            Some(4096),
        ));
        let history = session
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("Codex history")?
            .history
            .clone();
        let patches: Vec<_> = (0..3)
            .map(|number| {
                format!("*** Begin Patch\n*** Add File: café-{number}.txt\n+你好\n*** End Patch")
            })
            .collect();
        let originals: Vec<Value> = patches.iter().enumerate().map(|(number, patch)| json!({
            "type":"custom_tool_call","id":format!("raw-item-{number}"),"call_id":format!("raw-call-{number}"),
            "namespace":"legacy","name":"patch_text","input":patch
        })).collect();
        let response = transport::event(json!({"type":"response.completed","response":{
            "id":"legacy-response","status":"completed","output":originals,"end_turn":true,
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}
        }}));
        let server = transport::Server::start(
            vec![response, transport::terminal("completed", None, None)],
            false,
        )
        .await?;
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_custom_tools(CustomToolRegistry::new(vec![ToolRegistration {
                namespace: Some("legacy".into()),
                name: "patch_text".into(),
                executor: "apply_patch".into(),
                grammar: None,
                raw_input: true,
            }])?);
        let definition = ToolDefinition {
            name: "apply_patch".into(),
            description: "Legacy normalization only, never executed".into(),
            parameters: json!({"type":"object","properties":{"patch":{"type":"string"}}}),
        };
        session.messages.push(Message::user("legacy raw history"));
        let mut stream = model
            .stream(
                CompletionRequestBuilder::new(model.clone(), "legacy raw history")
                    .tool(definition.clone())
                    .build(),
            )
            .await?;
        while let Some(event) = stream.next().await {
            event?;
        }
        let checkpoint = history.snapshot()?.ok_or("legacy checkpoint")?;
        for item in checkpoint
            .items
            .iter()
            .filter_map(|item| history.tool_call(item).transpose())
            .take(completed)
        {
            let call = item?;
            session.messages.push(Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(AssistantContent::ToolCall(call.clone())),
            });
            session.messages.push(Message::tool_result_with_call_id(
                &call.id,
                call.call_id,
                "authentic legacy result",
            ));
        }
        let output = RecordingOutput::default();
        let agent = rig::agent::AgentBuilder::new(model.clone()).build();
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "legacy raw history",
            &mut session,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            manifest.id,
        )
        .await?;
        assert_eq!(history.snapshot()?, Some(checkpoint));
        {
            let events = output
                .0
                .lock()
                .map_err(|_| "legacy event buffer poisoned")?;
            let interrupted = events
                .iter()
                .position(|event| matches!(event, StreamEvent::Interrupted(_)))
                .ok_or("legacy interruption")?;
            let calls: Vec<_> = events[..interrupted]
                .iter()
                .filter_map(|event| match event {
                    StreamEvent::ToolCall(call) => Some(call),
                    _ => None,
                })
                .collect();
            let results: Vec<_> = events[..interrupted]
                .iter()
                .filter_map(|event| match event {
                    StreamEvent::ToolResult(result) => Some(result),
                    _ => None,
                })
                .collect();
            assert_eq!(calls.len(), 3 - completed);
            assert_eq!(results.len(), 3 - completed);
            for (number, (call, result)) in calls.iter().zip(results).enumerate() {
                let index = number + completed;
                assert_eq!(call.id, format!("raw-item-{index}"));
                assert_eq!(call.call_id, Some(format!("raw-call-{index}")));
                assert_eq!(call.name, "apply_patch");
                assert_eq!(call.args, json!({"patch":patches[index]}));
                assert_eq!(result.id, call.id);
                assert_eq!(result.call_id, call.call_id);
                assert!(result.is_error && result.content.contains("cancelled"));
            }
        }
        let request = CompletionRequestBuilder::new(model.clone(), "continue")
            .messages(session.messages.clone())
            .tool(definition)
            .build();
        let mut stream = model.stream(request).await?;
        while let Some(event) = stream.next().await {
            event?;
        }
        let requests = server.requests.lock().map_err(|_| "legacy requests")?;
        assert_eq!(
            requests.len(),
            2,
            "legacy cancellation must not infer or execute a raw tool"
        );
        let input = requests[1]["input"]
            .as_array()
            .ok_or("legacy replay input")?;
        for (number, original) in originals.iter().enumerate() {
            let calls: Vec<_> = input
                .iter()
                .filter(|item| {
                    item["type"] == "custom_tool_call" && item["call_id"] == original["call_id"]
                })
                .collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(*calls[0], *original);
            let results: Vec<_> = input
                .iter()
                .filter(|item| {
                    item["type"] == "custom_tool_call_output"
                        && item["call_id"] == original["call_id"]
                })
                .collect();
            assert_eq!(results.len(), 1);
            if number < completed {
                assert_eq!(results[0]["output"], "authentic legacy result");
            } else {
                assert!(results[0]["output"]
                    .as_str()
                    .ok_or("legacy outcome")?
                    .contains("cancelled"));
            }
        }
    }
    Ok(())
}
