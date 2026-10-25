//! Feature: spec/features/ordered-chat-completion-tools.feature

#[path = "support/bug262_chat.rs"]
mod chat;
#[path = "support/prov161_runtime.rs"]
mod runtime;

use codelet_cli::{interactive::StreamEvent, session::Session};
use codelet_core::RigAgent;
use codelet_providers::ProviderType;
use codelet_tools::DoneTool;
use rig::client::CompletionClient;
use runtime::{Output, RuntimeFixture, TestResult};
use serde_json::json;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

async fn run_case(terminal: Option<&str>) -> TestResult {
    let fixture = RuntimeFixture::new()?;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let log = directory.path().join("executed");
    let server = MockServer::start().await;
    let response = match terminal {
        Some(reason) => ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(chat::sse(reason)),
        None => ResponseTemplate::new(200).set_body_json(chat::response()),
    };
    Mock::given(path("/chat/completions"))
        .respond_with(response)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let manager = fixture.manager(ProviderType::OpenAI, json!({"maxOutputTokens":4096}))?;
    let session_id = fixture.session_id("openai")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    codelet_tools::set_continue_armed(session_id, true);
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("synthetic-loopback")
            .base_url(server.uri())
            .build()?;
    let agent = rig::agent::AgentBuilder::new(
        client
            .completion_model("local-model")
            .with_stream(terminal.is_some()),
    )
    .tool(chat::Write(log.clone()))
    .tool(DoneTool::new(session_id))
    .build();
    let mut session = Session::from_provider_manager(manager);
    let output = Output::default();
    let result = runtime::run(RigAgent::new(agent, 2), &mut session, session_id, &output).await;
    codelet_tools::set_continue_armed(session_id, false);
    if let Err(error) = result {
        let events = output.0.lock().map_err(|error| error.to_string())?;
        let results: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolResult(result) => Some((&result.id, &result.content)),
                _ => None,
            })
            .take(35)
            .collect();
        return Err(format!("{error}; tool results: {results:?}").into());
    }
    {
        let events = output.0.lock().map_err(|error| error.to_string())?;
        let results: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert!(
            results
                .iter()
                .all(|result| !result.content.contains("invalid work_unit_id")),
            "{results:?}"
        );
        assert!(
            results
                .iter()
                .any(|result| result.id == "call-32"
                    && result.content.contains("Completion recorded")),
            "Done was not accepted: {results:?}"
        );
        assert!(events.iter().any(|event| matches!(event, StreamEvent::Status(text) if text.contains("done: ordered task complete"))), "accepted Done did not terminate the turn");
    }
    let expected = (0..32)
        .map(|index| format!("{index}\n"))
        .collect::<String>();
    let executed = match std::fs::read_to_string(log) {
        Ok(executed) => executed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    assert_eq!(
        executed, expected,
        "accepted Done skipped or reordered earlier effects"
    );
    let history = serde_json::to_string(&session.messages)?;
    for index in 0..32 {
        assert!(
            history.contains(&format!("call-{index}")),
            "missing call-{index}"
        );
    }
    assert!(!history.contains("call-33"), "trailing tool executed");
    assert!(output.0.lock().map_err(|error| error.to_string())?.iter().any(
        |event| matches!(event, StreamEvent::Status(text) if text.contains("done: ordered task complete"))
    ));
    assert!(codelet_tools::take_done_acceptance(session_id).is_none());
    server.verify().await;
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("missing requests")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn tool_finish_flush_preserves_provider_index_order_before_done() -> TestResult {
    // @step Given reverse arriving fragmented indexed work and native Done followed by trailing work
    // @step When a loopback Chat stream finishes with tool_calls
    // @step Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work
    run_case(Some("tool_calls")).await
}

#[tokio::test]
#[serial_test::serial]
async fn eof_flush_preserves_provider_index_order_before_done() -> TestResult {
    // @step Given reverse arriving fragmented indexed work and native Done followed by trailing work
    // @step When a loopback Chat stream finishes with stop and reaches EOF
    // @step Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work
    run_case(Some("stop")).await
}

#[tokio::test]
#[serial_test::serial]
async fn json_response_vector_retains_streaming_ordering_parity() -> TestResult {
    // @step Given indexed work and native Done followed by trailing work in a JSON response vector
    // @step When a loopback nonstreaming Chat request is adapted to a stream
    // @step Then earlier work executes in index order with intact identities and arguments before accepted Done cancels trailing work
    run_case(None).await
}
