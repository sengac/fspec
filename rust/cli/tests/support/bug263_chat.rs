use super::{chat, runtime};
use codelet_cli::{interactive::StreamEvent, session::Session};
use codelet_core::RigAgent;
use codelet_providers::ProviderType;
use codelet_tools::DoneTool;
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::streaming::StreamedAssistantContent;
use runtime::{Output, RuntimeFixture, TestResult};
use serde_json::{json, Value};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub fn usage() -> Value {
    json!({"prompt_tokens":20,"completion_tokens":30,"total_tokens":50})
}

pub fn frame(value: Value) -> String {
    format!("data: {value}\n\n")
}

pub fn terminal(usage: Value, reason: Value) -> String {
    frame(json!({"choices":[{"index":0,"delta":{},"finish_reason":reason}],"usage":usage}))
}

pub fn done_response() -> Value {
    let mut response = chat::response();
    response["choices"][0]["message"]["tool_calls"] = json!([
        {"id":"call-32","type":"function","function":{"name":"Done",
            "arguments":json!({"summary":"ordered task complete","work_unit_id":"NONE"}).to_string()}},
        {"id":"call-33","type":"function","function":{"name":"WriteOrdered",
            "arguments":json!({"index":33,"work_unit_id":"NONE"}).to_string()}}
    ]);
    response
}

pub fn done_sse(usage: Value, tail: &str) -> String {
    let response = done_response();
    let calls: Vec<_> = response["choices"][0]["message"]["tool_calls"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, call)| json!({"index":index,"id":call["id"],"function":call["function"]}))
        .collect();
    frame(json!({"choices":[{"index":0,"delta":{"tool_calls":calls},"finish_reason":null}]}))
        + terminal(usage, json!("tool_calls")).as_str()
        + tail
}

pub fn model(
    url: &str,
    streaming: bool,
) -> TestResult<rig::providers::openai::completion::CompletionModel> {
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("synthetic-loopback")
            .base_url(url)
            .build()?;
    Ok(client
        .completion_model("local-model")
        .with_stream(streaming))
}

#[derive(Default, Debug)]
pub struct Observed {
    pub errors: Vec<String>,
    pub finals: Vec<(u64, Option<String>)>,
    pub calls: usize,
}

pub async fn consume(url: &str, streaming: bool) -> TestResult<Observed> {
    let model = model(url, streaming)?;
    let mut observed = Observed::default();
    let stream = model
        .stream(CompletionRequestBuilder::new(model.clone(), "test").build())
        .await;
    match stream {
        Err(error) => observed.errors.push(error.to_string()),
        Ok(mut stream) => {
            while let Some(event) = stream.next().await {
                match event {
                    Err(error) => observed.errors.push(error.to_string()),
                    Ok(StreamedAssistantContent::ToolCall(_)) => observed.calls += 1,
                    Ok(StreamedAssistantContent::Final(response)) => observed.finals.push((
                        response
                            .token_usage()
                            .ok_or("missing final usage")?
                            .total_tokens,
                        response.stop_reason().map(str::to_owned),
                    )),
                    _ => {}
                }
            }
        }
    }
    Ok(observed)
}

pub async fn server(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/chat/completions"))
        .respond_with(response)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    server
}

pub fn sse(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}

pub async fn run_done(response: ResponseTemplate, streaming: bool, valid: bool) -> TestResult {
    let fixture = RuntimeFixture::new()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let log = directory.path().join("trailing");
    let server = server(response).await;
    let manager = fixture.manager(ProviderType::OpenAI, json!({"maxOutputTokens":4096}))?;
    let session_id = fixture.session_id("openai")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    codelet_tools::set_continue_armed(session_id, true);
    let agent = rig::agent::AgentBuilder::new(model(&server.uri(), streaming)?)
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
    let accepted = events.iter().any(|event| matches!(event, StreamEvent::Status(text) if text.contains("done: ordered task complete")));
    assert_eq!(accepted, valid, "unexpected completion: {outcome:?}");
    assert!(!log.exists(), "trailing tool executed");
    assert_eq!(session.continue_nudges_used, 0);
    assert!(recorded.is_none());
    if valid {
        outcome?;
        assert!(events.iter().any(|event| matches!(event, StreamEvent::ToolResult(result) if result.content.contains("Completion recorded"))));
        let manifest = codelet_core::persistence::load_session(session_id)?;
        assert_eq!(manifest.token_usage.cumulative_billed_input, 20);
        assert_eq!(manifest.token_usage.cumulative_billed_output, 30);
        assert_eq!(session.token_tracker.input_tokens, 20);
        assert!(serde_json::to_string(&session.messages)?.contains("call-32"));
    } else {
        assert!(outcome.is_err(), "invalid response succeeded");
    }
    assert_eq!(requests.len(), 1);
    Ok(())
}
