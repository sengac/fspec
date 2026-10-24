//! Feature: spec/features/codex-recovery-boundaries.feature

#[path = "../../providers/tests/fixtures/prov148.rs"]
mod transport;

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use rig::message::Message;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, PoisonError};
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

/// @step Given a restored checkpoint with completed custom and namespaced calls
/// @step And their outcomes exist in the checkpoint or conversation
/// @step When I interrupt before the first request prepares a tool registry
/// @step Then interruption and completion are published without new tool outcomes or inference
/// @step And existing call identities and outcomes remain unchanged
#[tokio::test]
#[serial_test::serial]
async fn interrupting_restored_completed_dialect_calls_does_not_require_a_registry() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    for checkpoint_outcomes in [false, true] {
        let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
        let mut session = Session::from_provider_manager(manager);
        let history = session
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("Codex session missing")?
            .history
            .clone();
        let mut items = vec![
            json!({"type":"custom_tool_call", "id":"patch-item", "call_id":"patch-call",
                "namespace":"functions", "name":"patch", "input":"original patch"}),
            json!({"type":"function_call", "id":"inspect-item", "call_id":"inspect-call",
                "namespace":"functions", "name":"read", "arguments":"{}"}),
        ];
        for (item, call) in [
            ("patch-item", "patch-call"),
            ("inspect-item", "inspect-call"),
        ] {
            if checkpoint_outcomes {
                items.push(json!({"type":if call == "patch-call" {
                    "custom_tool_call_output"
                } else { "function_call_output" }, "call_id":call,"output":"already complete"}));
            } else {
                session.messages.push(Message::tool_result_with_call_id(
                    item,
                    Some(call.into()),
                    "already complete",
                ));
            }
        }
        history.restore(Some(serde_json::from_value(json!({
            "version":1, "generation":0, "protocol":"full-current-dag",
            "model":"shared-model", "items":items
        }))?))?;
        let before_checkpoint = history.snapshot()?;
        let before_messages = session.messages.clone();
        let server = transport::Server::start(Vec::new(), false).await?;
        let model = server.models()?.0.with_history(history.clone());
        let agent = rig::agent::AgentBuilder::new(model).build();
        let output = RecordingOutput::default();
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "new turn",
            &mut session,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(Notify::new()),
            &output,
            uuid::Uuid::new_v4(),
        )
        .await?;
        assert_eq!(history.snapshot()?, before_checkpoint);
        assert_eq!(
            &session.messages[..before_messages.len()],
            before_messages.as_slice()
        );
        assert_eq!(session.messages.len(), before_messages.len() + 1);
        let events = output.0.lock().map_err(|_| "events poisoned")?;
        assert!(events
            .iter()
            .any(|event| matches!(event, StreamEvent::Interrupted(_))));
        assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, StreamEvent::ToolCall(_) | StreamEvent::ToolResult(_))));
        assert!(server
            .requests
            .lock()
            .map_err(|_| "requests poisoned")?
            .is_empty());
    }
    Ok(())
}
