use super::environment::TestResult;
use codelet_providers::{
    copilot::{CopilotAuthJson, CopilotDeploymentType, CopilotProvider},
    session::ProviderSession,
    ProviderAdapter,
};
use futures::StreamExt;
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, CompletionRequestBuilder},
    message::Message,
};
use serde_json::{json, Value};
use std::time::Duration;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub type Model = codelet_providers::copilot::CopilotCompletionModel;

pub async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(|request: &wiremock::Request| {
        let body: Value = request.body_json().expect("synthetic request is JSON");
        let prompt = body["input"].as_array().and_then(|items| items.last())
            .map(|item| item["content"].to_string()).unwrap_or_default();
        let failed = prompt.contains("failure");
        let slow = prompt.contains("slow");
        let text = if prompt.contains("new") { "settled new" } else { "settled seed" };
        let opaque = if prompt.contains("new") { "synthetic-opaque-new" } else { "synthetic-opaque-seed" };
        let response = json!({"id":"fixture-response","object":"response","created_at":1,"model":"gpt-5",
            "status":if failed {"failed"} else {"completed"},
            "error":if failed {json!({"code":"synthetic_failure","message":"synthetic request failed"})} else {Value::Null},
            "output":[{"type":"message","id":"fixture-message","status":"completed","role":"assistant",
                "content":[{"type":"output_text","text":text,"annotations":[]}]}],
            "reasoning_opaque":opaque,"usage":{"input_tokens":4,"output_tokens":2,"total_tokens":6}});
        let event = json!({"type":if failed {"response.failed"} else {"response.completed"},"sequence_number":1,"response":response});
        let delta = json!({"type":"response.output_text.delta","sequence_number":0,"output_index":0,"content_index":0,"item_id":"fixture-message","delta":text});
        let template = ResponseTemplate::new(200).insert_header("content-type","text/event-stream")
            .set_body_string(format!("data: {delta}\n\ndata: {event}\n\n"));
        if slow { template.set_delay(Duration::from_millis(500)) } else { template }
    }).mount(&server).await;
    server
}

pub fn model(server: &MockServer, session: &ProviderSession) -> TestResult<Model> {
    model_for_deployment(server, session, CopilotDeploymentType::GitHubCom)
}

pub fn model_for_deployment(
    server: &MockServer,
    session: &ProviderSession,
    deployment: CopilotDeploymentType,
) -> TestResult<Model> {
    assert!(server.uri().starts_with("http://127.0.0.1:"));
    let provider = CopilotProvider::from_auth(
        deployment,
        CopilotAuthJson {
            github_oauth_token: "synthetic-logical-account".into(),
            copilot_token: Some("synthetic-short-token".into()),
            copilot_token_expires_at: Some(u64::MAX),
            endpoints_api: Some(server.uri()),
            enterprise_url: None,
        },
        "gpt-5",
    )?
    .with_session(session);
    Ok(provider.client().completion_model("gpt-5"))
}

pub async fn complete(
    model: &Model,
    history: Vec<Message>,
    prompt: &str,
) -> TestResult<Vec<Message>> {
    let request = CompletionRequestBuilder::new(model.clone(), prompt)
        .messages(history)
        .build();
    let mut history: Vec<_> = request.chat_history.iter().cloned().collect();
    let mut stream = model.stream(request).await?;
    while let Some(item) = stream.next().await {
        item?;
    }
    assert!(stream.response.is_some(), "missing validated final");
    let raw = serde_json::to_string(&stream.response)?;
    assert!(!raw.contains("synthetic-opaque"));
    history.push(Message::Assistant {
        id: None,
        content: stream.choice,
    });
    Ok(history)
}

pub async fn last_request(server: &MockServer) -> TestResult<Value> {
    let requests = server.received_requests().await.ok_or("missing requests")?;
    let request = requests.last().ok_or("missing request")?;
    assert_eq!(request.url.path(), "/responses");
    Ok(request.body_json()?)
}

pub async fn wait_requests(server: &MockServer, count: usize) -> TestResult {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if server
                .received_requests()
                .await
                .is_some_and(|requests| requests.len() >= count)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}
