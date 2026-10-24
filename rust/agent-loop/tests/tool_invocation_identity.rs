#![doc = "Feature: spec/features/codex-tool-invocation-identity-preservation.feature"]

use codelet_agent_loop::{persist::persist_user_message, BackgroundOutput};
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use codelet_core::persistence::{
    create_session_with_provider, get_session_message_envelopes, load_session,
    reset_stores_for_tests,
};
use codelet_core::SessionManagerHandle;
use codelet_providers::codex::session::CodexSession;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_test_helpers::setup_temp_directory;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::message::{AssistantContent, Message, UserContent};
use serde_json::{json, Value};

#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Environment {
    previous_key: Option<std::ffi::OsString>,
    previous_user_dir: Option<std::ffi::OsString>,
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (key, value) in [
            ("ANTHROPIC_API_KEY", &self.previous_key),
            ("FSPEC_USER_DIR", &self.previous_user_dir),
        ] {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        reset_stores_for_tests();
    }
}

/// @step Given a streamed assistant tool call and cancellation outcome have distinct identities
/// @step When their events are persisted and loaded into a fresh session
/// @step Then the restored call and outcome retain both original identities
/// @step And the next provider request replays the original call and cancellation exactly once
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn streamed_tool_identities_survive_persistence_and_cold_restore() -> TestResult {
    let directory = setup_temp_directory()?;
    let _environment = Environment {
        previous_key: std::env::var_os("ANTHROPIC_API_KEY"),
        previous_user_dir: std::env::var_os("FSPEC_USER_DIR"),
    };
    std::env::set_var("ANTHROPIC_API_KEY", "test-tool-identity");
    std::env::set_var("FSPEC_USER_DIR", directory.path());
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov101_models.json"),
    )?;
    let manifest = create_session_with_provider("identity", directory.path(), "anthropic")?;
    let session_id = SessionId::new(manifest.id.to_string());
    let manager = SessionManager::new();
    manager
        .create_session_from_manifest(&manifest, "anthropic/claude-opus-4-5")
        .await?;
    let session = manager.get_session(&session_id.value)?;
    persist_user_message(&manifest.id, "inspect")?;
    {
        let mut inner = session.inner.lock().await;
        inner.provider_session.ensure_provider("codex");
        inner.bind_provider_persistence(manifest.id)?;
        inner
            .provider_session
            .extension::<CodexSession>("codex")
            .ok_or("Codex session missing")?
            .history
            .commit(
                &json!({"model":"shared-model","input":[{"type":"message","role":"user",
                    "content":[{"type":"input_text","text":"inspect"}]}]}),
                &[
                    json!({"type":"function_call", "id":"fc_original", "call_id":"call_original",
                    "name":"inspect", "arguments":"{}"}),
                ],
            )?;
    }
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "fc_original".into(),
        call_id: Some("call_original".into()),
        name: "inspect".into(),
        args: json!({}),
    }));
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: "fc_original".into(),
        call_id: Some("call_original".into()),
        content: "Tool execution cancelled".into(),
        is_error: true,
    }));
    drop(output);
    drop(session);
    drop(manager);
    reset_stores_for_tests();
    let envelopes = get_session_message_envelopes(manifest.id)?;
    let blocks: Vec<Value> = envelopes
        .iter()
        .map(|encoded| serde_json::from_str::<Value>(encoded))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|envelope| {
            envelope["message"]["content"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|block| matches!(block["type"].as_str(), Some("tool_use" | "tool_result")))
        .collect();
    assert_eq!(blocks.len(), 2);
    for block in &blocks {
        assert_eq!(block["call_id"], "call_original");
    }
    assert_eq!(blocks[0]["id"], "fc_original");
    assert_eq!(blocks[1]["tool_use_id"], "fc_original");
    assert_eq!(blocks[1]["is_error"], true);
    let cold_manager = SessionManager::new();
    cold_manager
        .create_session_from_manifest(&load_session(manifest.id)?, "anthropic/claude-opus-4-5")
        .await?;
    let restored = cold_manager.get_session(&session_id.value)?;
    restored
        .inner
        .lock()
        .await
        .provider_session
        .ensure_provider("codex");
    cold_manager.restore_session_messages(&session_id, envelopes)?;
    let inner = restored.inner.lock().await;
    let mut identities = Vec::new();
    for message in &inner.messages {
        match message {
            Message::Assistant { content, .. } => {
                for block in content.iter() {
                    if let AssistantContent::ToolCall(call) = block {
                        identities.push((call.id.as_str(), call.call_id.as_deref()));
                    }
                }
            }
            Message::User { content } => {
                for block in content.iter() {
                    if let UserContent::ToolResult(result) = block {
                        identities.push((result.id.as_str(), result.call_id.as_deref()));
                    }
                }
            }
        }
    }
    assert_eq!(identities, vec![("fc_original", Some("call_original")); 2]);
    let history = inner
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("restored Codex session missing")?
        .history
        .clone();
    let messages = inner.messages.clone();
    drop(inner);
    let server =
        transport::Server::start(vec![transport::terminal("completed", None, None)], false).await?;
    let model = server.models()?.0.with_history(history);
    let request = CompletionRequestBuilder::new(model.clone(), "continue")
        .messages(messages)
        .build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    let requests = server.requests.lock().map_err(|_| "requests poisoned")?;
    let input = requests.first().ok_or("continuation missing")?["input"]
        .as_array()
        .ok_or("input missing")?;
    for kind in ["function_call", "function_call_output"] {
        let items: Vec<_> = input.iter().filter(|item| item["type"] == kind).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["call_id"], "call_original");
        if kind == "function_call" {
            assert_eq!(items[0]["id"], "fc_original");
        } else {
            assert_eq!(items[0]["output"], "Tool execution cancelled");
        }
    }
    Ok(())
}
