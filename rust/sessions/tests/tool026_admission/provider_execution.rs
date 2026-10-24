use crate::{
    fixture::Fixture,
    support::{args, Calls, Probe, Result},
};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_tools::tool_attribution::configure_agent_tools;
use futures::StreamExt;
use rig::{
    agent::{Agent, MultiTurnStreamItem},
    client::CompletionClient,
    completion::CompletionModel,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn verify(fixture: &Fixture) -> Result {
    for streaming in [false, true] {
        let endpoint = MockServer::start().await;
        let openai: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-tool026")
                .base_url(endpoint.uri())
                .build()?;
        execute(
            openai.agent("fixture-openai").build(),
            fixture,
            &endpoint,
            "openai",
            streaming,
        )
        .await?;
        let endpoint = MockServer::start().await;
        let anthropic: rig::providers::anthropic::Client =
            rig::providers::anthropic::Client::builder()
                .api_key("offline-tool026")
                .base_url(endpoint.uri())
                .build()?;
        execute(
            anthropic.agent("fixture-claude").max_tokens(1024).build(),
            fixture,
            &endpoint,
            "anthropic",
            streaming,
        )
        .await?;
    }
    Ok(())
}

async fn execute<M: CompletionModel + 'static>(
    agent: Agent<M>,
    fixture: &Fixture,
    endpoint: &MockServer,
    provider: &str,
    streaming: bool,
) -> Result {
    fixture
        .manager
        .set_session_focus(&fixture.first, Some("AUTH-001"), "provider-fixture")?;
    let other = fixture.manager.get_work_unit_focus(&fixture.second)?;
    let calls: Calls = Arc::new(Mutex::new(Vec::new()));
    agent
        .tool_server_handle
        .add_tool(Probe {
            calls: calls.clone(),
            gate: None,
            callback: None,
        })
        .await?;
    configure_agent_tools(&agent, &fixture.first.value).await?;
    let mut valid: Value = serde_json::from_str(&args("NONE"))?;
    let original = valid.clone();
    valid
        .as_object_mut()
        .ok_or("business arguments")?
        .remove("work_unit_id");
    let invalid = valid.clone();
    let count = AtomicUsize::new(0);
    let boundaries = Arc::new(Mutex::new(Vec::new()));
    let observed = boundaries.clone();
    let effects = calls.clone();
    let manager = fixture.manager.clone();
    let session = fixture.first.clone();
    let protocol = provider.to_owned();
    let original_call = original.clone();
    let invalid_call = invalid.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            let index = count.fetch_add(1, Ordering::SeqCst);
            let executed = effects
                .lock()
                .map(|calls| calls.len())
                .unwrap_or(usize::MAX);
            let focus = manager
                .get_work_unit_focus(&session)
                .ok()
                .and_then(|focus| focus.context.map(|card| card.id));
            if let Ok(mut boundaries) = observed.lock() {
                boundaries.push((executed, focus));
            }
            let arguments = if index == 0 {
                &invalid_call
            } else {
                &original_call
            };
            response(&protocol, streaming, index, arguments)
        })
        .mount(endpoint)
        .await;
    let agent = codelet_core::RigAgent::new(agent, 4);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        if streaming {
            let stream = agent.prompt_streaming("Exercise attributed tools").await;
            futures::pin_mut!(stream);
            let mut finished = false;
            while let Some(item) = stream.next().await {
                if let MultiTurnStreamItem::FinalResponse(response) = item? {
                    assert_eq!(response.response(), "finished");
                    finished = true;
                }
            }
            assert!(finished, "{provider}: no final response");
        } else {
            assert_eq!(agent.prompt("Exercise attributed tools").await?, "finished");
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    assert_eq!(
        *boundaries.lock().map_err(|_| "boundaries")?,
        vec![
            (0, Some("AUTH-001".into())),
            (0, Some("AUTH-001".into())),
            (1, None)
        ],
        "{provider}/{streaming}"
    );
    assert!(fixture.focus(&fixture.first)?.is_none());
    assert_eq!(fixture.manager.get_work_unit_focus(&fixture.second)?, other);
    {
        let recorded = calls.lock().map_err(|_| "calls")?;
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, valid);
        assert_eq!(
            recorded[0]
                .1
                .as_ref()
                .ok_or("receipt")?
                .session_id
                .to_string(),
            fixture.first.value
        );
    }
    let requests = endpoint.received_requests().await.ok_or("HTTP requests")?;
    let bodies = requests
        .iter()
        .map(wiremock::Request::body_json::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let parameters = if provider == "openai" {
        &bodies[0]["tools"][0]["function"]["parameters"]
    } else {
        &bodies[0]["tools"][0]["input_schema"]
    };
    assert!(parameters["required"]
        .as_array()
        .is_some_and(|required| required.contains(&json!("work_unit_id"))));
    let rejected = history(&bodies[1], provider, "call_invalid", &invalid)?;
    assert!(rejected.contains("work_unit_id"), "{rejected}");
    let admitted = history(&bodies[2], provider, "call_valid", &original)?;
    assert_eq!(serde_json::from_str::<Value>(&admitted)?, valid);
    Ok(())
}

fn history(body: &Value, provider: &str, id: &str, expected: &Value) -> Result<String> {
    let messages = body["messages"].as_array().ok_or("request messages")?;
    if provider == "openai" {
        let call = messages
            .iter()
            .filter_map(|message| message["tool_calls"].as_array())
            .flatten()
            .find(|call| call["id"] == id)
            .ok_or("historical OpenAI call")?;
        assert_eq!(
            serde_json::from_str::<Value>(
                call["function"]["arguments"]
                    .as_str()
                    .ok_or("raw arguments")?
            )?,
            *expected
        );
        let result = messages
            .iter()
            .find(|message| message["role"] == "tool" && message["tool_call_id"] == id)
            .ok_or("correlated OpenAI result")?;
        return Ok(result["content"]
            .as_str()
            .ok_or("OpenAI result content")?
            .to_owned());
    }
    let blocks: Vec<_> = messages
        .iter()
        .filter_map(|message| message["content"].as_array())
        .flatten()
        .collect();
    let call = blocks
        .iter()
        .find(|block| block["type"] == "tool_use" && block["id"] == id)
        .ok_or("historical Anthropic call")?;
    assert_eq!(call["input"], *expected);
    let result = blocks
        .iter()
        .find(|block| block["type"] == "tool_result" && block["tool_use_id"] == id)
        .ok_or("correlated Anthropic result")?;
    Ok(result["content"]
        .as_str()
        .or_else(|| result["content"][0]["text"].as_str())
        .ok_or("Anthropic result content")?
        .to_owned())
}

fn response(provider: &str, streaming: bool, index: usize, arguments: &Value) -> ResponseTemplate {
    let id = if index == 0 {
        "call_invalid"
    } else {
        "call_valid"
    };
    if provider == "anthropic" {
        let content = if index < 2 {
            json!({"type":"tool_use","id":id,"name":"probe","input":arguments})
        } else {
            json!({"type":"text","text":"finished"})
        };
        let stop = if index < 2 { "tool_use" } else { "end_turn" };
        if streaming {
            return anthropic_stream(content, stop);
        }
        return ResponseTemplate::new(200).set_body_json(
            json!({"id":"fixture-message","type":"message",
            "role":"assistant","model":"fixture-claude","content":[content],"stop_reason":stop,
            "stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":5}}),
        );
    }
    let message = if index < 2 {
        json!({"role":"assistant","content":null,"tool_calls":[{
        "id":id,"index":0,"type":"function","function":{"name":"probe","arguments":arguments.to_string()}}]})
    } else {
        json!({"role":"assistant","content":"finished"})
    };
    let stop = if index < 2 { "tool_calls" } else { "stop" };
    if streaming {
        let chunk = json!({"id":"fixture-message","object":"chat.completion.chunk","created":1,
            "model":"fixture-openai","choices":[{"index":0,"delta":message,"finish_reason":stop}],
            "usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}});
        return ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"));
    }
    ResponseTemplate::new(200).set_body_json(json!({"id":"fixture-message","object":"chat.completion",
        "created":1,"model":"fixture-openai","choices":[{"index":0,"message":message,"finish_reason":stop}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}))
}

fn anthropic_stream(mut content: Value, stop: &str) -> ResponseTemplate {
    let delta = if content["type"] == "tool_use" {
        let input = content["input"].to_string();
        content["input"] = json!({});
        json!({"type":"input_json_delta","partial_json":input})
    } else {
        content["text"] = json!("");
        json!({"type":"text_delta","text":"finished"})
    };
    let events = [
        json!({"type":"message_start","message":{"id":"fixture-message","type":"message",
            "role":"assistant","model":"fixture-claude","content":[],"stop_reason":null,
            "stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}),
        json!({"type":"content_block_start","index":0,"content_block":content}),
        json!({"type":"content_block_delta","index":0,"delta":delta}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":stop,"stop_sequence":null},"usage":{"output_tokens":5}}),
        json!({"type":"message_stop"}),
    ];
    let body: String = events
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap_or("message")
            )
        })
        .collect();
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}
