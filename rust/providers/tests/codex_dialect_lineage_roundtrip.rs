//! Feature: spec/features/codex-streamed-lineage-roundtrip.feature

use codelet_providers::codex::{
    custom_tools::{CustomToolRegistry, ToolRegistration},
    history::CodexHistory,
};
use codelet_test_helpers::setup_temp_directory;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder},
    message::{AssistantContent, Message},
};
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn dialect(raw: bool) -> Result<(CustomToolRegistry, Value, Value), Box<dyn std::error::Error>> {
    let registry = CustomToolRegistry::new(vec![ToolRegistration {
        namespace: Some("edit".into()),
        name: "patch".into(),
        executor: "apply_patch".into(),
        grammar: None,
        raw_input: raw,
    }])?;
    let arguments = json!({"patch":"*** Begin Patch\n你好\n*** End Patch"});
    let mut call = json!({"type":"function_call","id":"item1","call_id":"call1",
        "namespace":"edit","name":"patch","arguments":arguments.to_string()});
    if raw {
        call["type"] = json!("custom_tool_call");
        call["input"] = arguments["patch"].clone();
        call.as_object_mut()
            .ok_or("call object missing")?
            .remove("arguments");
    }
    Ok((registry, call, arguments))
}

fn initial() -> Value {
    json!({"model":"shared-model","input":[{"role":"user",
        "content":[{"type":"input_text","text":"patch"}]}]})
}

fn messages(executor: &str, identity: &str, arguments: Value) -> Vec<Message> {
    vec![
        Message::user("patch"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                "item1",
                identity.into(),
                executor,
                arguments,
            )),
        },
        Message::tool_result_with_call_id("item1", Some(identity.into()), "applied"),
    ]
}

#[tokio::test]
async fn scenario_registered_tool_dialect_projections_retain_their_original_checkpoint_identity(
) -> TestResult {
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    for raw in [false, true] {
        let (registry, call, arguments) = dialect(raw)?;
        let history = CodexHistory::default();
        history.commit(&initial(), std::slice::from_ref(&call))?;
        std::fs::write(&path, serde_json::to_vec(&history.snapshot()?)?)?;
        let restored = CodexHistory::default();
        restored.restore(serde_json::from_slice(&std::fs::read(&path)?)?)?;
        for history in [history, restored] {
            let server = fixture::Server::start(vec![], false).await?;
            let model = server
                .models()?
                .0
                .with_history(history.clone())
                .with_custom_tools(registry.clone());
            for turn in 0..2 {
                let mut conversation = messages("apply_patch", "call1", arguments.clone());
                if turn > 0 {
                    conversation.push(Message::user("continue"));
                    conversation.push(Message::assistant("Done"));
                    conversation.push(Message::user("next"));
                }
                let request = CompletionRequestBuilder::new(model.clone(), "continue")
                    .messages(conversation)
                    .build();
                let body: Value = serde_json::from_slice(&model.prepare_request(request)?)?;
                assert_eq!(body["input"][1], call);
                assert_eq!(
                    body["input"][2]["type"],
                    if raw {
                        "custom_tool_call_output"
                    } else {
                        "function_call_output"
                    }
                );
                assert_eq!(body["input"][2]["call_id"], "call1");
                assert_eq!(body["input"][2]["output"], "applied");
                if turn == 0 {
                    history.commit(
                        &body,
                        &[json!({"type":"message","role":"assistant",
                        "content":[{"type":"output_text","text":"Done"}]})],
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn scenario_changed_tool_dialect_ancestry_remains_rejected() -> TestResult {
    for raw in [false, true] {
        let (registry, call, arguments) = dialect(raw)?;
        for (executor, identity, arguments) in [
            ("apply_patch", "call1", json!({"patch":"changed"})),
            ("another_executor", "call1", arguments.clone()),
            ("apply_patch", "another_call", arguments),
        ] {
            let history = CodexHistory::default();
            history.commit(&initial(), std::slice::from_ref(&call))?;
            let saved = history.snapshot()?;
            let server = fixture::Server::start(vec![], false).await?;
            let model = server
                .models()?
                .0
                .with_history(history.clone())
                .with_custom_tools(registry.clone());
            let request = CompletionRequestBuilder::new(model.clone(), "continue")
                .messages(messages(executor, identity, arguments))
                .build();
            assert!(model.stream(request).await.is_err());
            assert_eq!(history.snapshot()?, saved);
            assert!(server
                .requests
                .lock()
                .map_err(|_| "requests lock")?
                .is_empty());
        }
    }
    Ok(())
}
