#[path = "fixtures/prov151.rs"]
pub mod fixture;
use codelet_providers::codex::presentation::{CodexPresentationHook, CodexToolInputUpdate};
use fixture::*;
use futures::StreamExt;
use rig::agent::AgentBuilder;
use rig::streaming::StreamingPrompt;
use serde_json::json;
use std::sync::{Arc, Mutex};

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
    // Wait for the five input fragments while the stream stays pending; the
    // first request also builds admission's token counter, which takes a
    // few hundred milliseconds in an unoptimised build.
    // TEST-042: no waiting window, under the deadlock guard.
    let mut next = stream.next();
    let fragments =
        codelet_test_helpers::pending_until("the five input fragments", &mut next, || {
            updates.lock().is_ok_and(|updates| updates.len() >= 5)
        })
        .await;
    drop(next);
    assert!(
        fragments.is_ok(),
        "the stream yielded before the tools ran: {fragments:?}"
    );
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
