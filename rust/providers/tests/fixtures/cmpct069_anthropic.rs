use rig::client::CompletionClient;
use rig::providers::anthropic;
use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub async fn model(
) -> Result<(MockServer, anthropic::completion::CompletionModel), Box<dyn std::error::Error>> {
    model_with_expected_requests(1).await
}

pub async fn model_with_expected_requests(
    expected: u64,
) -> Result<(MockServer, anthropic::completion::CompletionModel), Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    let events = [
        json!({"type":"message_start","message":{
            "id":"msg_settlement","role":"assistant","model":"test-model",
            "content":[],"stop_reason":null,"stop_sequence":null,
            "usage":{"input_tokens":600,"output_tokens":0,
                "cache_read_input_tokens":400,"cache_creation_input_tokens":50}
        }}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"done"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":100}}),
        json!({"type":"message_stop"}),
    ];
    let body: String = events
        .into_iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap_or("message")
            )
        })
        .collect();
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .expect(expected)
        .mount(&server)
        .await;
    let model = anthropic::Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    Ok((server, model))
}
