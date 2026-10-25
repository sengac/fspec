#![doc = "Feature: spec/features/gemini-invocation-identity.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput, ToolCallEvent};
use codelet_core::{
    persistence::transcript::{JournalStore, ToolState},
    RigAgent,
};
use rig::providers::gemini::completion;
use rig::{client::CompletionClient, completion::ToolDefinition, providers::gemini, tool::Tool};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[path = "support/bug253_streaming_compatibility.rs"]
mod streaming_compatibility;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

#[derive(Clone)]
struct ReadFile(PathBuf, Arc<AtomicUsize>);

impl Tool for ReadFile {
    const NAME: &'static str = "read_file";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Read fixture contents".into(),
            parameters: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        }
    }

    async fn call(&self, args: Value) -> Result<String, Self::Error> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| std::io::Error::other("missing path"))?;
        let result = std::fs::read_to_string(self.0.join(path))?;
        self.1.fetch_add(1, Ordering::SeqCst);
        Ok(result)
    }
}

struct GeminiResponses(AtomicUsize);

impl Respond for GeminiResponses {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let index = self.0.fetch_add(1, Ordering::SeqCst);
        let parts = if index < 2 || index == 3 {
            json!([{"functionCall":{"name":"read_file","args":{"path":if index == 0 {"first"} else {"second"},"work_unit_id":"NONE"}},"thoughtSignature":"signed-invocation"}])
        } else {
            json!([{"text":"finished"}])
        };
        let frame = json!({"candidates":[{"content":{"role":"model","parts":parts},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":2,"totalTokenCount":12}});
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {frame}\n\n"))
    }
}

#[tokio::test]
#[serial_test::serial]
async fn repeated_gemini_calls_retain_separate_authentic_results() -> TestResult {
    // @step Given a Gemini provider requesting signed reads of different files
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    std::fs::write(directory.path().join("first"), "first authentic result")?;
    std::fs::write(directory.path().join("second"), "second authentic result")?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.id, directory.path().into())?;
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(GeminiResponses(AtomicUsize::new(0)))
        .mount(&server)
        .await;
    let client = gemini::Client::builder()
        .api_key("fixture-key")
        .base_url(server.uri())
        .build()?;
    let invocations = Arc::new(AtomicUsize::new(0));
    let agent = client
        .agent("fixture-model")
        .tool(ReadFile(directory.path().into(), invocations.clone()))
        .build();
    let output = BackgroundOutput::with_provider(session.clone(), "gemini".into(), None);
    // @step When a background turn consumes repeated reads and a later response attempt reads again
    for _ in 0..2 {
        output.emit(StreamEvent::ResponseAttemptStarted);
        run_agent_stream(
            RigAgent::with_default_depth(agent.clone()),
            "read both files",
            &mut *session.inner.lock().await,
            session.is_interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            session.interrupt_notify.clone(),
            &output,
            session.id,
        )
        .await?;
    }
    // @step Then each invocation executes once with a distinct durable identity and authentic result
    assert_eq!(invocations.load(Ordering::SeqCst), 3);
    assert!(!support::read(session.id)?.is_empty());
    for _ in 0..2 {
        let projection = JournalStore::configured(session.id)?.project(&HashSet::new())?;
        assert_eq!(projection.tools.len(), 3);
        assert_ne!(projection.tools[0].id, projection.tools[1].id);
        assert_eq!(
            projection
                .tools
                .iter()
                .map(|tool| &tool.id)
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        assert_ne!(
            projection.tools[0].attempt_id,
            projection.tools[2].attempt_id
        );
        for (tool, expected) in projection.tools.iter().zip([
            "first authentic result",
            "second authentic result",
            "second authentic result",
        ]) {
            assert_eq!(tool.state, ToolState::Completed);
            assert_eq!(tool.result.as_deref(), Some(expected));
        }
    }
    // @step And physical continuations preserve function names and thought signatures
    let requests = server.received_requests().await.ok_or("missing requests")?;
    assert!(requests.len() >= 3);
    let body: Value = serde_json::from_slice(&requests[2].body)?;
    let contents = body["contents"].as_array().ok_or("missing contents")?;
    let mut signed = 0;
    let mut results = 0;
    for part in contents
        .iter()
        .filter_map(|content| content["parts"].as_array())
        .flatten()
    {
        if part.get("functionCall").is_some() {
            assert_eq!(part["functionCall"]["name"], "read_file");
            assert_eq!(part["thoughtSignature"], "signed-invocation");
            signed += 1;
        }
        if part.get("functionResponse").is_some() {
            assert_eq!(part["functionResponse"]["name"], "read_file");
            results += 1;
        }
    }
    assert_eq!((signed, results), (2, 2));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn duplicate_dispatch_and_cold_recovery_never_repeat_a_side_effect() -> TestResult {
    // @step Given an acknowledged Gemini invocation whose result has not been captured
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let output = BackgroundOutput::with_provider(session.clone(), "gemini".into(), None);
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "internal-invocation".into(),
        call_id: None,
        name: "read_file".into(),
        args: json!({"path":"first"}),
    }));
    // @step When its internal identity is dispatched twice and the journal is reopened repeatedly
    output.prepare_tool_dispatch("internal-invocation")?;
    let sentinel = directory.path().join("side-effect");
    std::fs::write(&sentinel, "once")?;
    assert!(output.prepare_tool_dispatch("internal-invocation").is_err());
    // @step Then duplicate dispatch is refused and replay retains outcome-unknown without reexecution
    for _ in 0..2 {
        let projection = JournalStore::configured(session.id)?.project(&HashSet::new())?;
        assert_eq!(projection.tools.len(), 1);
        assert_eq!(projection.tools[0].state, ToolState::OutcomeUnknown);
        assert_eq!(projection.tools[0].result, None);
        assert_eq!(std::fs::read_to_string(&sentinel)?, "once");
    }
    Ok(())
}
