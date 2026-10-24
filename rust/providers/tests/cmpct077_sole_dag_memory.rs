//! Feature: spec/features/codex-sole-dag-memory-policy.feature
#[path = "fixtures/prov147.rs"]
mod baseline;
#[path = "fixtures/cmpct077.rs"]
mod fixture;

use codelet_providers::codex::{history::CodexHistory, NativeCapabilities};
use fixture::{
    consume_request, model, request, terminal, unsupported, PersistedHistory, TestResult,
};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::{json, Value};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn scenario_native_memory_activation_fails_before_sending_a_request() -> TestResult {
    // @step Given a Codex model and explicit native memory configuration
    let server = MockServer::start().await;
    let persisted = PersistedHistory::new()?;
    let history = persisted.history.clone();
    let saved = json!({"model":"shared-model","input":[{"role":"user","content":"prior"}]});
    history.commit(&saved, &[])?;
    let prior = history.snapshot()?;
    let durable_prior = persisted.bytes()?;
    let model = model(&server, history.clone())?;
    for (streaming, configuration) in [false, true].into_iter().flat_map(|streaming| {
        [
            json!(null),
            json!([]),
            json!([{"type":"compaction","compact_threshold":1}]),
        ]
        .map(|configuration| (streaming, configuration))
    }) {
        let request = CompletionRequestBuilder::new(model.clone(), "Hello")
            .additional_params(json!({"context_management":configuration}))
            .build();
        // @step When streaming and direct completion are requested
        let error = if streaming {
            model
                .stream(request)
                .await
                .err()
                .ok_or("activation reached stream")?
        } else {
            model
                .completion(request)
                .await
                .err()
                .ok_or("activation completed")?
        };
        // @step Then typed unsupported errors leave the checkpoint unchanged and send no request
        unsupported(&error);
        assert_eq!(history.snapshot()?, prior);
        assert_eq!(persisted.bytes()?, durable_prior);
    }
    assert!(server
        .received_requests()
        .await
        .ok_or("requests missing")?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn scenario_unsolicited_native_memory_output_never_commits_or_completes() -> TestResult {
    // @step Given a committed Codex checkpoint and responses containing native compaction items
    for family in [
        "compaction",
        "compaction_summary",
        "context_compaction",
        "compaction_trigger",
    ] {
        for route in [
            "direct",
            "stream-terminal",
            "stream-added",
            "stream-done",
            "stream-completion",
        ] {
            let server = MockServer::start().await;
            let persisted = PersistedHistory::new()?;
            let history = persisted.history.clone();
            let model = model(&server, history.clone())?;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_json(baseline::response()))
                .expect(1)
                .mount(&server)
                .await;
            model.completion(request(&model)).await?;
            let prior = history.snapshot()?;
            let durable_prior = persisted.bytes()?;
            server.reset().await;
            let item = json!({"type":family,"encrypted_content":"private-native"});
            let raw = terminal(vec![item.clone()]);
            let response = if route == "direct" {
                ResponseTemplate::new(200).set_body_json(raw)
            } else {
                let event = match route {
                    "stream-added" => json!({"type":"response.output_item.added","item":item}),
                    "stream-done" => json!({"type":"response.output_item.done","item":item}),
                    _ => json!({"type":"response.completed","response":raw}),
                };
                let tool = json!({"type":"response.output_item.done","output_index":0,"item":{
                    "type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","status":"completed","arguments":"{}"}});
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {tool}\n\ndata: {event}\n\n"))
            };
            Mock::given(method("POST"))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
            let model = if route == "stream-completion" {
                model.with_streaming_completions()
            } else {
                model
            };
            // @step When streaming events and direct responses are consumed
            let continuation = CompletionRequestBuilder::new(model.clone(), "next")
                .messages(vec![
                    rig::message::Message::user("Hello"),
                    rig::message::Message::assistant("Hello"),
                ])
                .build();
            let error = consume_request(
                &model,
                route.starts_with("stream-") && route != "stream-completion",
                continuation,
            )
            .await?;
            // @step Then typed unsupported errors preserve the checkpoint without tools or completion
            unsupported(&error);
            assert_eq!(history.snapshot()?, prior, "{route} {family}");
            assert_eq!(persisted.bytes()?, durable_prior, "{route} {family}");
            server.verify().await;
            assert_eq!(server.received_requests().await.ok_or("requests")?.len(), 1);
        }
    }
    Ok(())
}

#[test]
fn scenario_native_checkpoint_and_trigger_input_cannot_bypass_the_dag_policy() -> TestResult {
    // @step Given a committed checkpoint and attempted native trigger or native checkpoint replay
    let persisted = PersistedHistory::new()?;
    let history = persisted.history.clone();
    let original = json!({"model":"shared-model","input":[{"role":"user","content":"Hello"}]});
    history.commit(&original, &[])?;
    let prior = history.snapshot()?;
    let durable_prior = persisted.bytes()?;
    let mut triggered = original;
    triggered["input"]
        .as_array_mut()
        .ok_or("input")?
        .push(json!({"type":"compaction_trigger"}));
    let unchanged = triggered.clone();
    // @step When history preparation and commit are attempted
    let error = history
        .prepare(&mut triggered)
        .err()
        .ok_or("trigger accepted")?;
    // @step Then typed unsupported errors retain the original checkpoint and input
    unsupported(&error);
    assert_eq!(triggered, unchanged);
    assert_eq!(history.snapshot()?, prior);
    assert_eq!(persisted.bytes()?, durable_prior);
    unsupported(
        &history
            .commit(
                &triggered,
                &[json!({"type":"compaction","encrypted_content":"private-native"})],
            )
            .err()
            .ok_or("native commit accepted")?,
    );
    assert_eq!(history.snapshot()?, prior);
    assert_eq!(persisted.bytes()?, durable_prior);
    for (model_name, saved) in ["shared-model", "different-model"]
        .into_iter()
        .flat_map(|name| {
            [
                json!({"version":1,"model":"shared-model","items":[{
                "type":"compaction","encrypted_content":"private-native"}],"compacted_users":[]}),
                json!({"version":1,"model":"shared-model","items":[],"compacted_users":[{
                "type":"message","role":"user","content":"old"}]}),
            ]
            .map(|saved| (name, saved))
        })
    {
        history.restore(Some(serde_json::from_value(saved)?))?;
        let native = history.snapshot()?;
        let mut incoming =
            json!({"model":model_name,"input":[{"role":"user","content":"replacement"}]});
        let unchanged = incoming.clone();
        unsupported(
            &history
                .prepare(&mut incoming)
                .err()
                .ok_or("native checkpoint replayed")?,
        );
        assert_eq!(history.snapshot()?, native);
        assert_eq!(incoming, unchanged);
        assert_eq!(persisted.bytes()?, durable_prior);
    }
    Ok(())
}

#[tokio::test]
async fn scenario_encrypted_reasoning_and_hosted_search_remain_independent_of_native_memory(
) -> TestResult {
    // @step Given a Codex model with hosted search enabled and encrypted reasoning output
    let server = MockServer::start().await;
    let reasoning = json!({"type":"reasoning","id":"rs_1","summary":[],"encrypted_content":"private-native-reasoning"});
    let search = json!({"type":"web_search_call","id":"ws_1","status":"completed","action":{"type":"search","query":"Rust"}});
    let output = vec![
        reasoning.clone(),
        search.clone(),
        baseline::response()["output"][0].clone(),
    ];
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!(
                    "data: {}\n\n",
                    json!({"type":"response.completed","response":terminal(output)})
                )),
        )
        .expect(2)
        .mount(&server)
        .await;
    let history = CodexHistory::default();
    let model = model(&server, history.clone())?.with_native_capabilities(NativeCapabilities {
        hosted_search: true,
    });
    // @step When a successful response is followed by another user request
    for next in [false, true] {
        let request = if next {
            CompletionRequestBuilder::new(model.clone(), "next")
                .messages(vec![
                    rig::message::Message::user("Hello"),
                    rig::message::Message::assistant("Hello"),
                ])
                .build()
        } else {
            request(&model)
        };
        let mut stream = model.stream(request).await?;
        while let Some(event) = stream.next().await {
            let event = event?;
            assert!(!format!("{event:?}").contains("private-native"));
        }
    }
    // @step Then supported items replay without native memory activation or opaque disclosure
    let requests = server.received_requests().await.ok_or("requests")?;
    let second: Value = serde_json::from_slice(&requests[1].body)?;
    let input = second["input"].as_array().ok_or("input")?;
    assert!(input.contains(&reasoning));
    assert!(input.contains(&search));
    for request in requests {
        let body: Value = serde_json::from_slice(&request.body)?;
        assert!(body.get("context_management").is_none());
        assert!(!body.to_string().contains("compaction_trigger"));
    }
    assert!(!format!("{:?}", history.snapshot()?).contains("private-native"));
    server.verify().await;
    Ok(())
}
