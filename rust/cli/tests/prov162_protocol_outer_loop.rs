//! Feature: spec/features/codex-protocol-outer-loop-composition.feature
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/prov161_runtime.rs"]
mod runtime;

use codelet_cli::{
    interactive::{ContinueStateReason, StreamEvent},
    session::Session,
};
use codelet_core::RigAgent;
use codelet_providers::{codex::session::CodexSession, ProviderType};
use environment::{Environment, TestResult};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial_test::serial]
async fn typed_protocol_tools_and_outer_nudges_retain_separate_accounting() -> TestResult {
    // @step Given a real CLI Codex session has typed protocol output a native Read and an armed Done
    let environment = Environment::new()?;
    let root = environment.directory.path();
    let file = root.join("evidence");
    std::fs::write(&file, "native tool evidence")?;
    let server = server(file.display().to_string()).await;
    let fixture = runtime::RuntimeFixture::new()?;
    let mut manager = fixture.manager(ProviderType::Codex, json!({"maxOutputTokens":4096}))?;
    manager.override_model_limits(None, Some(4096));
    let session_id = fixture.session_id("codex")?;
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        session_id,
        codelet_common::get_data_dir()?,
    )?;
    let mut session = Session::from_provider_manager(manager);
    let history = session
        .provider_session
        .extension::<CodexSession>("codex")
        .ok_or("missing Codex owner")?
        .history
        .clone();
    session.bind_provider_persistence(session_id)?;
    session.continue_enabled = true;
    session.continue_budget = 1;
    codelet_tools::set_continue_armed(session_id, true);
    let model = runtime::codex_model(&server.uri(), runtime::limits(session.provider_manager())?)
        .await?
        .with_history(history);
    let agent = rig::agent::AgentBuilder::new(model)
        .tool(codelet_tools::ReadTool::new(session_id))
        .tool(codelet_tools::DoneTool::new(session_id))
        .build();
    let output = runtime::Output::default();
    // @step When tool results and one outer continuation nudge complete the task
    let outcome = runtime::run(RigAgent::new(agent, 8), &mut session, session_id, &output).await;
    codelet_tools::set_continue_armed(session_id, false);
    outcome?;
    // @step Then native call result identities and owned reasoning remain paired across exact physical requests
    let sent = server
        .received_requests()
        .await
        .ok_or("missing captured requests")?;
    assert_eq!(sent.len(), 5);
    for request in &sent[2..] {
        let body: Value = request.body_json()?;
        let input = body["input"].as_array().ok_or("missing input")?;
        assert_eq!(
            input
                .iter()
                .filter(
                    |item| item["type"] == "function_call_output" && item["call_id"] == "read-call"
                )
                .count(),
            1
        );
        assert!(input
            .iter()
            .any(|item| item["type"] == "function_call_output"
                && item["output"].to_string().contains("native tool evidence")));
        assert!(input
            .iter()
            .any(|item| item["encrypted_content"] == "synthetic-native-owned"));
    }
    // @step And one outer nudge and once only physical billing survive internal protocol follow ups
    let events = output.0.lock().map_err(|_| "events poisoned")?;
    assert_eq!(events.iter().filter(|event|matches!(event,StreamEvent::ContinueState(state) if state.reason==ContinueStateReason::NudgeConsumed)).count(),1);
    assert!(events.iter().any(
        |event| matches!(event,StreamEvent::Status(text) if text.contains("done: typed complete"))
    ));
    let manifest = codelet_core::persistence::load_session(session_id)?;
    assert_eq!(manifest.token_usage.cumulative_billed_input, 60);
    assert_eq!(manifest.token_usage.cumulative_billed_output, 40);
    Ok(())
}

async fn server(file: String) -> MockServer {
    let server = MockServer::start().await;
    let count = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST")).respond_with(move |_: &wiremock::Request| {
        let number=count.fetch_add(1,Ordering::SeqCst);
        let output=match number {
            0 | 3 => vec![json!({"type":"reasoning","id":format!("reason-{number}"),"summary":[],"encrypted_content":"synthetic-native-owned"}),
                json!({"type":"message","id":format!("partial-{number}"),"role":"assistant","content":[{"type":"output_text","text":"checking"}]})],
            1 => vec![json!({"type":"function_call","id":"read-item","call_id":"read-call","name":"Read","arguments":json!({"file_path":file,"work_unit_id":"NONE"}).to_string()})],
            2 => vec![json!({"type":"message","id":"answer","role":"assistant","content":[{"type":"output_text","text":"read complete"}]})],
            _ => vec![json!({"type":"function_call","id":"done-item","call_id":"done-call","name":"Done","arguments":json!({"summary":"typed complete","work_unit_id":"NONE"}).to_string()})],
        };
        let response=json!({"type":"response.completed","response":{"id":format!("physical-{number}"),"status":"completed","end_turn":number==2,"output":output,
            "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}}});
        ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(format!("data: {response}\n\n"))
    }).mount(&server).await;
    server
}

#[tokio::test]
#[serial_test::serial]
async fn refusal_settles_visibly_without_protocol_or_outer_continuation() -> TestResult {
    // @step Given a real CLI Codex session has outer continuation enabled and receives a completed refusal
    let _environment = Environment::new()?;
    let fixture = runtime::RuntimeFixture::new()?;
    let mut manager = fixture.manager(ProviderType::Codex, json!({"maxOutputTokens":4096}))?;
    manager.override_model_limits(None, Some(4096));
    let session_id = fixture.session_id("codex")?;
    let mut session = Session::from_provider_manager(manager);
    session.continue_enabled = true;
    session.continue_budget = 1;
    let server = MockServer::start().await;
    let response = json!({"type":"response.completed","response":{"id":"refusal","status":"completed","end_turn":false,
        "output":[{"type":"message","id":"refused","role":"assistant","content":[{"type":"refusal","refusal":"Cannot comply"}]}],
        "usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}}});
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {response}\n\n")),
        )
        .mount(&server)
        .await;
    let model =
        runtime::codex_model(&server.uri(), runtime::limits(session.provider_manager())?).await?;
    let agent = rig::agent::AgentBuilder::new(model).build();
    let output = runtime::Output::default();
    // @step When the refusal carries an explicit nonfinal turn marker
    runtime::run(RigAgent::new(agent, 8), &mut session, session_id, &output).await?;
    // @step Then exactly one physical response settles visible refusal and its measured billing
    assert_eq!(
        server
            .received_requests()
            .await
            .ok_or("missing requests")?
            .len(),
        1
    );
    let events = output.0.lock().map_err(|_| "events poisoned")?;
    assert!(format!("{events:?}").contains("Cannot comply"));
    let manifest = codelet_core::persistence::load_session(session_id)?;
    assert_eq!(manifest.token_usage.cumulative_billed_input, 12);
    assert_eq!(manifest.token_usage.cumulative_billed_output, 8);
    // @step And no protocol follow up outer nudge or executable tool occurs
    assert!(!events.iter().any(|event| matches!(event, StreamEvent::ContinueState(state) if state.reason==ContinueStateReason::NudgeConsumed)));
    assert!(!events
        .iter()
        .any(|event| matches!(event, StreamEvent::ToolResult(_))));
    Ok(())
}
