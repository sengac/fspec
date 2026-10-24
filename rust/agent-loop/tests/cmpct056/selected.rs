use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use rig::client::CompletionClient;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

struct QuietOutput;

impl StreamOutput for QuietOutput {
    fn emit(&self, _: StreamEvent) {}
}

pub async fn install(session: &mut Session, session_id: Uuid) {
    let server = MockServer::start().await;
    let chunk = serde_json::json!({
        "id":"selected-target", "object":"chat.completion.chunk", "created":1,
        "model":"o3", "choices":[{"index":0,
        "delta":{"role":"assistant", "content":"ready"}, "finish_reason":"stop"}],
        "usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}
    });
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
        )
        .mount(&server)
        .await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("test-selected-key")
            .base_url(server.uri())
            .build()
            .expect("local selected client");
    let agent = RigAgent::with_default_depth(
        rig::agent::AgentBuilder::new(client.completion_model("o3"))
            .preamble("selected target instructions")
            .max_tokens(4096)
            .build(),
    );
    codelet_cli::interactive::run_agent_stream(
        agent,
        "selected target prompt",
        session,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(tokio::sync::Notify::new()),
        &QuietOutput,
        session_id,
    )
    .await
    .expect("install selected serializer through actual runtime stream");
}
