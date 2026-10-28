use codelet_providers::codex::{
    diagnostics::{CodexRejection, RejectionClass},
    history::CodexHistory,
    CodexResponsesModel, NativeCapabilities,
};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};

#[path = "fixtures/prov151.rs"]
pub mod fixture;
use fixture::{event, Server, TestResult};

fn message() -> Value {
    fixture::previous::baseline::response()["output"][0].clone()
}

fn metadata(output: Option<Vec<Value>>, end_turn: bool) -> String {
    let baseline = fixture::previous::baseline::response();
    let mut response = json!({"id":baseline["id"],"object":baseline["object"],
        "created_at":baseline["created_at"],"model":baseline["model"],
        "usage":baseline["usage"],"end_turn":end_turn});
    if let Some(output) = output {
        response["output"] = json!(output);
    }
    event(json!({"type":"response.completed","response":response}))
}

fn done(index: u64, item: Value) -> String {
    event(json!({"type":"response.output_item.done","output_index":index,"item":item}))
}

#[tokio::test]
async fn metadata_only_completion_accepts_omitted_status_and_output() -> TestResult {
    for completion in [false, true] {
        let server = Server::start(
            vec![[done(0, message()), metadata(None, true)].concat()],
            false,
        )
        .await?;
        let history = CodexHistory::default();
        let model = server
            .models()?
            .0
            .with_history(history.clone())
            .with_streaming_completions();
        if completion {
            let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
            let response = model.completion(request).await?;
            assert_eq!(response.usage.total_tokens, 20);
            assert_eq!(response.raw_response.output.len(), 1);
        } else {
            let observed = fixture::previous::collect(model).await?;
            assert!(observed.errors.is_empty(), "{:?}", observed.errors);
            assert_eq!(observed.text, "Hello");
            assert_eq!(observed.finals, [20]);
            assert_eq!(observed.usage, [20]);
        }
        let checkpoint = history.snapshot()?.ok_or("missing checkpoint")?;
        assert_eq!(checkpoint.items.len(), 2);
        assert_eq!(
            serde_json::to_value(&checkpoint.items[1])?["id"],
            "msg_test"
        );
    }
    Ok(())
}

#[tokio::test]
async fn invalid_terminal_output_and_missing_success_usage_still_reject() -> TestResult {
    for (output, usage) in [(vec![Value::Null], true), (vec![message()], false)] {
        let mut response = fixture::previous::baseline::response();
        response["output"] = json!(output);
        if !usage {
            response
                .as_object_mut()
                .ok_or("response object")?
                .remove("usage");
        }
        let body = [
            done(0, message()),
            event(json!({"type":"response.completed","response":response})),
        ]
        .concat();
        let server = Server::start(vec![body], false).await?;
        let history = CodexHistory::default();
        let observed =
            fixture::previous::collect(server.models()?.0.with_history(history.clone())).await?;
        assert_eq!(observed.errors.len(), 1);
        assert!(observed.finals.is_empty());
        assert!(history.snapshot()?.is_none());
    }
    Ok(())
}

#[tokio::test]
async fn metadata_only_terminal_accepts_streamed_text_without_completed_items() -> TestResult {
    let observed =
        fixture::previous::run([fixture::previous::text(), metadata(None, true)].concat()).await?;
    assert!(observed.errors.is_empty(), "{:?}", observed.errors);
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.finals, [20]);
    assert_eq!(observed.usage, [20]);
    Ok(())
}

#[tokio::test]
async fn completed_items_merge_in_wire_order_into_partial_terminal_checkpoints() -> TestResult {
    let reasoning = json!({"type":"reasoning","id":"rs_test","summary":[],
        "encrypted_content":"opaque-reasoning"});
    let call = fixture::item("fc_test", "call_test", "read_file", "{ \"path\": \"a\" }");
    for output in [
        None,
        Some(vec![]),
        Some(vec![message()]),
        Some(vec![call.clone()]),
        Some(vec![
            json!({"type":"function_call","id":"fc_test","arguments":""}),
        ]),
        Some(vec![reasoning.clone(), message(), call.clone()]),
    ] {
        let server = Server::start(
            vec![[
                done(2, call.clone()),
                done(0, reasoning.clone()),
                done(1, message()),
                done(2, call.clone()),
                metadata(output, true),
            ]
            .concat()],
            false,
        )
        .await?;
        let history = CodexHistory::default();
        let observed =
            fixture::previous::collect(server.models()?.0.with_history(history.clone())).await?;
        assert!(observed.errors.is_empty(), "{:?}", observed.errors);
        assert_eq!(observed.tools, 1);
        assert_eq!(observed.text, "Hello");
        let saved = history.snapshot()?.ok_or("missing checkpoint")?;
        let items = serde_json::to_value(saved.items)?;
        assert_eq!(items.as_array().ok_or("checkpoint items")?.len(), 4);
        assert_eq!(items[1], reasoning);
        assert_eq!(items[2]["id"], "msg_test");
        assert_eq!(items[3]["call_id"], "call_test");
        assert_eq!(items[3]["arguments"], call["arguments"]);
    }
    Ok(())
}

#[tokio::test]
async fn completed_items_supply_replay_when_terminal_output_is_omitted() -> TestResult {
    let server = Server::start(
        vec![
            [done(0, message()), metadata(None, false)].concat(),
            metadata(Some(vec![message()]), true),
        ],
        false,
    )
    .await?;
    let observed = fixture::previous::collect(server.models()?.0).await?;
    assert!(observed.errors.is_empty(), "{:?}", observed.errors);
    assert_eq!(observed.text, "HelloHello");
    assert_eq!(observed.finals, [40]);
    assert_eq!(observed.usage, [20, 20]);
    let requests = server.requests.lock().map_err(|_| "requests lock")?;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1]["input"][1], message());
    Ok(())
}

async fn stream_error(
    model: CodexResponsesModel,
) -> Result<CompletionError, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => return Ok(error),
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_)) => {
                return Err("failed response dispatched a tool or succeeded".into());
            }
            _ => {}
        }
    }
    Err("failed response did not report an error".into())
}

#[tokio::test]
async fn failed_responses_without_usage_preserve_typed_backend_rejections() -> TestResult {
    for completion in [false, true] {
        for (code, class) in [
            ("bad_request", RejectionClass::Other),
            (
                "context_length_exceeded",
                RejectionClass::ContextLengthExceeded,
            ),
        ] {
            let failure = event(json!({"type":"response.failed","response":{
                "id":"resp_failed","status":"failed","output":[],
                "error":{"code":code,"message":"backend rejected test-key"}}}));
            let server = Server::start(
                vec![[
                    fixture::done(fixture::item("fc_test", "call_test", "read_file", "{}")),
                    failure,
                ]
                .concat()],
                false,
            )
            .await?;
            let history = CodexHistory::default();
            let model = server
                .models()?
                .0
                .with_history(history.clone())
                .with_streaming_completions();
            let error = if completion {
                let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
                model
                    .completion(request)
                    .await
                    .err()
                    .ok_or("failure completed")?
            } else {
                stream_error(model).await?
            };
            let CompletionError::HttpError(rig::http_client::Error::Instance(instance)) = &error
            else {
                return Err(format!("missing typed rejection: {error}").into());
            };
            let rejection = instance
                .downcast_ref::<CodexRejection>()
                .ok_or("rejection type")?;
            assert_eq!(rejection.class, class);
            assert_eq!(rejection.code.as_deref(), Some(code));
            assert!(rejection
                .message
                .as_deref()
                .is_some_and(|text| text.contains("backend rejected")));
            assert!(!error.to_string().contains("test-key"));
            assert!(history.snapshot()?.is_none());
        }
    }
    Ok(())
}

#[tokio::test]
async fn hosted_search_notices_are_reasoning_not_assistant_text() -> TestResult {
    let search = json!({"type":"web_search_call","id":"ws_test","status":"completed"});
    let mut answer = message();
    answer["content"][0]["annotations"] = json!([{"type":"url_citation",
        "url":"https://rust-lang.org","title":"Rust","start_index":0,"end_index":5}]);
    let body = [
        event(json!({"type":"response.web_search_call.searching","item_id":"ws_test"})),
        done(0, search.clone()),
        metadata(Some(vec![search, answer]), true),
    ]
    .concat();
    let server = Server::start(vec![body], false).await?;
    let model = server
        .models()?
        .0
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        });
    let request = CompletionRequestBuilder::new(model.clone(), "Hello").build();
    let mut stream = model.stream(request).await?;
    let mut text = String::new();
    let mut notices = String::new();
    let mut finals = 0;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Text(part) => text.push_str(&part.text),
            StreamedAssistantContent::Reasoning(reasoning) => {
                notices.push_str(&reasoning.reasoning.join(""))
            }
            StreamedAssistantContent::Final(_) => finals += 1,
            _ => {}
        }
    }
    assert_eq!(text, "Hello");
    assert_eq!(finals, 1);
    assert!(notices.contains("ws_test: searching"));
    assert_eq!(notices.matches("ws_test: completed").count(), 1);
    assert!(notices.contains("https://rust-lang.org"));
    let projected = serde_json::to_string(&stream.choice)?;
    assert!(!projected.contains("Codex hosted search"));
    assert!(!projected.contains("Codex source"));
    Ok(())
}
