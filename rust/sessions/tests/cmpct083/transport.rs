use super::fixture::TestResult;
use serde_json::{json, Value};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn server(row: &str) -> MockServer {
    let server = MockServer::start().await;
    let body = match row {
        "anthropic" => {
            let events = [
                (
                    "message_start",
                    json!({"type":"message_start","message":{"id":"msg_gate","type":"message","role":"assistant","model":"claude-sonnet-4","content":[],"usage":{"input_tokens":10,"output_tokens":0}}}),
                ),
                (
                    "content_block_start",
                    json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
                ),
                (
                    "content_block_delta",
                    json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ready"}}),
                ),
                (
                    "content_block_stop",
                    json!({"type":"content_block_stop","index":0}),
                ),
                (
                    "message_delta",
                    json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}),
                ),
                ("message_stop", json!({"type":"message_stop"})),
            ];
            events
                .into_iter()
                .map(|(kind, value)| format!("event: {kind}\ndata: {value}\n\n"))
                .collect()
        }
        "gemini" => format!(
            "data: {}\n\n",
            json!({"candidates":[{"content":{"role":"model","parts":[{"text":"ready"}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":1,"totalTokenCount":11}})
        ),
        "codex" => {
            let response = json!({"id":"resp_gate","object":"response","created_at":1,"status":"completed","model":"shared-model",
                "output":[{"type":"message","id":"msg_gate","role":"assistant","status":"completed","content":[{"type":"output_text","text":"ready","annotations":[]}]}],
                "usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11,"input_tokens_details":{"cached_tokens":0},"output_tokens_details":{"reasoning_tokens":0}}});
            format!(
                "data: {}\n\ndata: {}\n\n",
                json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"sequence_number":1,"delta":"ready"}),
                json!({"type":"response.completed","sequence_number":2,"response":response})
            )
        }
        "custom" => format!(
            "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
            json!({"kind":"text_delta","text":"ready"}),
            json!({"kind":"usage","input_tokens":10,"output_tokens":1})
        ),
        _ => format!(
            "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
            json!({"id":"gate","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"ready"},"finish_reason":"stop"}]}),
            json!({"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}})
        ),
    };
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    server
}

pub async fn submission(row: &str, content: &str) -> MockServer {
    let server = MockServer::start().await;
    let arguments = json!({"content":content}).to_string();
    let body = match row {
        "anthropic" => [
            ("message_start", json!({"type":"message_start","message":{"id":"msg_submit","type":"message","role":"assistant","model":"claude-sonnet-4","content":[],"usage":{"input_tokens":10,"output_tokens":0}}})),
            ("content_block_start", json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"summary-call","name":"inject_summary","input":{}}})),
            ("content_block_delta", json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":arguments}})),
            ("content_block_stop", json!({"type":"content_block_stop","index":0})),
            ("message_delta", json!({"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":1}})),
            ("message_stop", json!({"type":"message_stop"})),
        ].into_iter().map(|(kind, value)| format!("event: {kind}\ndata: {value}\n\n")).collect(),
        "gemini" => format!("data: {}\n\n", json!({"candidates":[{"content":{"role":"model","parts":[{"functionCall":{"name":"inject_summary","args":{"content":content}}}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":1,"totalTokenCount":11}})),
        "codex" => {
            let item = json!({"type":"function_call","id":"fc_summary","call_id":"summary-call","name":"inject_summary","arguments":arguments,"status":"completed"});
            [json!({"type":"response.output_item.done","output_index":0,"sequence_number":1,"item":item}),
                json!({"type":"response.completed","sequence_number":2,"response":{"id":"resp_submit","status":"completed","end_turn":true,"output":[item],"usage":{"input_tokens":10,"output_tokens":1,"total_tokens":11}}})]
                .into_iter().map(|value| format!("data: {value}\n\n")).collect()
        }
        "custom" => format!("data: {}\n\ndata: {}\n\n", json!({"kind":"tool_call","id":"summary-call","name":"inject_summary","arguments":arguments,"complete":true}), json!({"kind":"stop","reason":"tool_use"})),
        _ => [
            json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"summary-call","type":"function","function":{"name":"inject_summary","arguments":arguments}}]},"finish_reason":null}]}),
            json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
        ].into_iter().map(|value| format!("data: {value}\n\n")).chain(std::iter::once("data: [DONE]\n\n".into())).collect(),
    };
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    server
}

pub async fn only(server: &MockServer) -> TestResult<Value> {
    let requests = server
        .received_requests()
        .await
        .ok_or("capture unavailable")?;
    assert_eq!(
        requests.len(),
        1,
        "ordinary selected send must not hide preceding physical requests"
    );
    Ok(requests.first().ok_or("no physical request")?.body_json()?)
}

pub async fn builder_response(content: &str, delay: std::time::Duration) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(delay).set_body_json(json!({
            "id":"builder-gate", "object":"chat.completion", "created":1, "model":"o3",
            "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}],
            "usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}
        })))
        .mount(&server)
        .await;
    server
}
