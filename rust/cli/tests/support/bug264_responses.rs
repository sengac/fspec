use super::{chat, runtime};
use codelet_cli::{interactive::StreamEvent, session::Session};
use codelet_core::RigAgent;
use codelet_providers::ProviderType;
use codelet_tools::DoneTool;
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::streaming::StreamedAssistantContent;
use runtime::{Output, RuntimeFixture, TestResult};
use serde_json::{json, Value};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub fn usage() -> Value {
    json!({"input_tokens":20,"output_tokens":30,"total_tokens":50})
}

pub fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}

pub fn terminal(status: &str, usage: Value) -> String {
    event(
        json!({"type":format!("response.{status}"),"sequence_number":3,"response":{
            "id":"local-response","object":"response","created_at":0,"model":"local-model",
            "status":status,"usage":usage,"output":[],
            "error":if status == "failed" { json!({"code":"invalid_schema","message":"rejected tool schema"}) } else { Value::Null },
            "incomplete_details":if status == "incomplete" { json!({"reason":"max_output_tokens"}) } else { Value::Null }
        }}),
    )
}

pub fn queued_tools() -> String {
    let mut body = String::new();
    for (index, name, arguments) in [
        (
            0,
            "Done",
            json!({"summary":"responses task complete","work_unit_id":"NONE"}),
        ),
        (1, "WriteOrdered", json!({"index":33,"work_unit_id":"NONE"})),
    ] {
        body.push_str(&event(json!({"type":"response.output_item.done","output_index":index,
            "sequence_number":index,"item":{"type":"function_call","id":format!("item-{index}"),
                "call_id":format!("call-{index}"),"name":name,"arguments":arguments.to_string(),"status":"completed"}})));
    }
    body
}

pub async fn server(body: String) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    server
}

pub fn model(
    url: &str,
) -> TestResult<rig::providers::openai::responses_api::ResponsesCompletionModel> {
    let client: rig::providers::openai::Client = rig::providers::openai::Client::builder()
        .api_key("synthetic-loopback")
        .base_url(url)
        .build()?;
    Ok(client.completion_model("local-model"))
}

#[derive(Default, Debug)]
pub struct Observed {
    pub errors: Vec<CompletionError>,
    pub finals: Vec<(u64, Option<String>)>,
    pub calls: usize,
}

pub async fn consume(body: String) -> TestResult<Observed> {
    let server = server(body).await;
    let model = model(&server.uri())?;
    let mut stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "test").build())
        .await?;
    let mut observed = Observed::default();
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => observed.errors.push(error),
            Ok(StreamedAssistantContent::ToolCall(_)) => observed.calls += 1,
            Ok(StreamedAssistantContent::Final(response)) => observed.finals.push((
                response.token_usage().ok_or("missing usage")?.total_tokens,
                response.stop_reason().map(str::to_owned),
            )),
            _ => {}
        }
    }
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("missing capture")?
            .len(),
        1
    );
    Ok(observed)
}

pub async fn run_done(body: String, valid: bool) -> TestResult {
    let fixture = RuntimeFixture::new()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let log = directory.path().join("trailing");
    let server = server(body).await;
    let manager = fixture.manager(ProviderType::OpenAI, json!({"maxOutputTokens":4096}))?;
    let session_id = fixture.session_id("openai")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    codelet_tools::set_continue_armed(session_id, true);
    let agent = rig::agent::AgentBuilder::new(model(&server.uri())?)
        .tool(DoneTool::new(session_id))
        .tool(chat::Write(log.clone()))
        .build();
    let mut session = Session::from_provider_manager(manager);
    session.continue_enabled = true;
    let output = Output::default();
    let outcome = runtime::run(RigAgent::new(agent, 2), &mut session, session_id, &output).await;
    let recorded = codelet_tools::take_done_acceptance(session_id);
    codelet_tools::set_continue_armed(session_id, false);
    let requests = server.received_requests().await.ok_or("missing capture")?;
    let events = output.0.lock().map_err(|error| error.to_string())?;
    assert!(!events.iter().any(|event| matches!(event, StreamEvent::ToolResult(result) if result.content.contains("invalid work_unit_id"))));
    let accepted = events.iter().any(|event| matches!(event, StreamEvent::Status(text) if text.contains("done: responses task complete")));
    assert_eq!(accepted, valid, "{outcome:?}");
    assert!(!log.exists());
    assert_eq!(session.continue_nudges_used, 0);
    assert!(recorded.is_none());
    if valid {
        outcome?;
        let manifest = codelet_core::persistence::load_session(session_id)?;
        assert_eq!(manifest.token_usage.cumulative_billed_input, 20);
        assert_eq!(manifest.token_usage.cumulative_billed_output, 30);
        let history = serde_json::to_string(&session.messages)?;
        assert!(history.contains("item-0") && history.contains("call-0"));
        assert!(events.iter().any(|event| matches!(event, StreamEvent::ToolResult(result) if result.id == "item-0" && result.call_id.as_deref() == Some("call-0") && result.content.contains("Completion recorded"))));
    } else {
        assert!(outcome.is_err());
    }
    assert_eq!(requests.len(), 1);
    Ok(())
}
