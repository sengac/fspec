#[path = "fixtures/prov151.rs"]
pub mod fixture;
use codelet_providers::codex::presentation::{CodexPresentationHook, CodexToolInputUpdate};
use fixture::*;
use futures::StreamExt;
use rig::agent::AgentBuilder;
use rig::streaming::StreamingPrompt;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn input_fragments_reach_presentation_before_execution() -> TestResult {
    // @step Given interleaved Codex names and fragmented arguments on an open stream
    // @step When a presentation consumer observes the agent
    // @step Then each input update retains its item identity and no partial call executes
    let updates = Arc::new(Mutex::new(Vec::<CodexToolInputUpdate>::new()));
    let sink_updates = updates.clone();
    let hook = CodexPresentationHook::new(
        Gate::default(),
        Some(Arc::new(move |update| {
            sink_updates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(update);
        })),
    );
    let body = [
        event(json!({"type":"response.output_item.added","item":item("fc_a","call_a","read_file","")})),
        event(json!({"type":"response.output_item.added","item":item("fc_b","call_b","read_file","")})),
        event(json!({"type":"response.function_call_arguments.delta","item_id":"fc_a","delta":"{\"path\":\""})),
        event(json!({"type":"response.function_call_arguments.delta","item_id":"fc_b","delta":"{\"path\":\"b\"}"})),
        event(json!({"type":"response.function_call_arguments.delta","item_id":"fc_a","delta":"a\"}"})),
    ].concat();
    let server = Server::start(vec![body], true).await?;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let agent = AgentBuilder::new(server.models()?.0)
        .tool(ReadTool(calls.clone()))
        .build();
    let mut stream = agent
        .stream_prompt("read")
        .with_hook(hook)
        .await
        .filter(|event| {
            futures::future::ready(!matches!(
                event,
                Ok(rig::agent::MultiTurnStreamItem::StreamAssistantItem(
                    rig::streaming::StreamedAssistantContent::ResponseAttemptStarted
                        | rig::streaming::StreamedAssistantContent::ResponseAttemptAborted
                ))
            ))
        });
    let pending = tokio::time::timeout(Duration::from_millis(150), stream.next()).await;
    assert!(pending.is_err());
    assert!(calls.lock().map_err(|_| "calls lock poisoned")?.is_empty());
    let updates = updates.lock().map_err(|_| "updates lock poisoned")?;
    assert_eq!(updates.len(), 5);
    assert_eq!(updates[0].item_id, "fc_a");
    assert_eq!(updates[0].name.as_deref(), Some("read_file"));
    assert_eq!(updates[1].item_id, "fc_b");
    assert_eq!(updates[3].arguments_delta, "{\"path\":\"b\"}");
    assert_eq!(updates[4].item_id, "fc_a");
    assert_eq!(updates[4].arguments_delta, "a\"}");
    Ok(())
}
