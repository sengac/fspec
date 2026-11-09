//! BUG-517 — stop_reason propagation tested end to end.
//!
//! A loopback server streams the provider's SSE response through the CLI
//! stream loop into a background session. The output is a tee: the
//! `BackgroundOutput` persists the assistant message (the stop_reason read
//! back from the message store), and the plain CLI's `CliOutput` logs the
//! truncation warning (captured from tracing). No mapping is written in the
//! test.
//!
//! Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature

use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{run_agent_stream, CliOutput, StreamEvent, StreamOutput};
use codelet_core::RigAgent;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::wasm_compat::WasmCompatSend;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::{matchers::method, Mock, MockServer, Request, Respond, ResponseTemplate};

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

const TRUNCATION_WARNING: &str = "Response truncated: model hit max_tokens output limit";

/// Forwards every event to the background session's output and to the plain
/// CLI output, and records the stop_reason the stream loop finished with.
struct Tee {
    background: BackgroundOutput,
    cli: CliOutput,
    finished: Mutex<Vec<Option<String>>>,
}

impl StreamOutput for Tee {
    fn prepare_tool_dispatch(&self, id: &str) -> Result<(), String> {
        self.background.prepare_tool_dispatch(id)
    }

    fn interruption_reason(&self) -> Option<codelet_rpc_types::InterruptionReason> {
        self.background.interruption_reason()
    }

    fn publish_committed_context(
        &self,
        identity: &codelet_core::persistence::compaction_boundary::CompactionIdentity,
        snapshot: codelet_rpc_types::CommittedContextSnapshot,
        context: codelet_rpc_types::RequestContextSnapshot,
    ) -> Result<(), String> {
        self.background
            .publish_committed_context(identity, snapshot, context)
    }

    fn emit(&self, event: StreamEvent) {
        if let StreamEvent::Done(stop_reason) = &event {
            if let Ok(mut finished) = self.finished.lock() {
                finished.push(stop_reason.clone());
            }
        }
        self.cli.emit(event.clone());
        self.background.emit(event);
    }

    fn flush(&self) {
        self.background.flush();
    }
}

/// Collects everything the tracing subscriber writes during the turn.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("capture poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Captured {
    fn text(&self) -> String {
        self.0
            .lock()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    }
}

/// What one turn left behind.
struct Turn {
    finished: Vec<Option<String>>,
    persisted: Vec<String>,
    log: String,
}

fn sse(events: &[Value]) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(
            events
                .iter()
                .map(|event| format!("data: {event}\n\n"))
                .collect::<String>(),
        )
}

/// An Anthropic message stream that ends with `stop_reason`.
fn anthropic_stream(stop_reason: &str) -> ResponseTemplate {
    sse(&[
        json!({"type":"message_start","message":{"id":"bug517","type":"message","role":"assistant",
            "model":"claude-sonnet-4-5","content":[],"stop_reason":null,"stop_sequence":null,
            "usage":{"input_tokens":12,"output_tokens":0}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"The answer is"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":stop_reason,"stop_sequence":null},
            "usage":{"output_tokens":4096}}),
        json!({"type":"message_stop"}),
    ])
}

/// An OpenAI chat completions stream that ends with `finish_reason`.
fn openai_stream(finish_reason: &str) -> ResponseTemplate {
    let chunk = |delta: Value, finish: Value| {
        json!({"id":"bug517","object":"chat.completion.chunk","created":1,"model":"gpt-test",
            "choices":[{"index":0,"delta":delta,"finish_reason":finish}]})
    };
    let mut body: String = [
        chunk(json!({"role":"assistant","content":"The answer is"}), Value::Null),
        chunk(json!({}), json!(finish_reason)),
        json!({"id":"bug517","object":"chat.completion.chunk","created":1,"model":"gpt-test",
            "choices":[],"usage":{"prompt_tokens":12,"completion_tokens":4096,"total_tokens":4108}}),
    ]
    .iter()
    .map(|event| format!("data: {event}\n\n"))
    .collect();
    body.push_str("data: [DONE]\n\n");
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}

/// Run one turn of `model` through the CLI stream loop into a background
/// session whose provider is `provider`.
async fn turn<M>(model: M, provider: &str) -> Result<Turn, Box<dyn std::error::Error>>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
{
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let _logging = tracing::subscriber::set_default(subscriber);
    let output = Tee {
        background: BackgroundOutput::with_provider(session.clone(), provider.into(), None),
        cli: CliOutput,
        finished: Mutex::new(Vec::new()),
    };
    let agent = rig::agent::AgentBuilder::new(model)
        .max_tokens(4096)
        .build();
    output.emit(StreamEvent::ResponseAttemptStarted);
    run_agent_stream(
        RigAgent::with_default_depth(agent),
        "answer briefly",
        &mut *session.inner.lock().await,
        session.is_interrupted.clone(),
        Arc::new(AtomicBool::new(false)),
        session.interrupt_notify.clone(),
        &output,
        session.id,
    )
    .await?;
    let persisted = support::read(session.id)?
        .iter()
        .filter(|metadata| metadata.pointer("/message/role") == Some(&json!("assistant")))
        .filter_map(|metadata| metadata.pointer("/message/stop_reason"))
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let finished = output
        .finished
        .lock()
        .map_err(|_| "finished poisoned")?
        .clone();
    drop(directory);
    Ok(Turn {
        finished,
        persisted,
        log: captured.text(),
    })
}

/// First an Anthropic stream that hits max_tokens inside a Write tool call
/// (the tool_use block never closes), then a normal answer.
struct TruncatedThenAnswered(AtomicUsize);

impl Respond for TruncatedThenAnswered {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
            return anthropic_stream("end_turn");
        }
        sse(&[
            json!({"type":"message_start","message":{"id":"bug517-tool","type":"message",
                "role":"assistant","model":"claude-sonnet-4-5","content":[],"stop_reason":null,
                "stop_sequence":null,"usage":{"input_tokens":12,"output_tokens":0}}}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use",
                "id":"toolu_bug517","name":"Write","input":{}}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta",
                "partial_json":"{\"file_path\": \"/tmp/big.rs\", \"content\": \"fn main() {"}}),
            json!({"type":"message_delta","delta":{"stop_reason":"max_tokens","stop_sequence":null},
                "usage":{"output_tokens":4096}}),
            json!({"type":"message_stop"}),
        ])
    }
}

/// The text of the last user message of a recorded Anthropic request.
fn last_user_text(request: &Request) -> String {
    let body: Value = request.body_json().unwrap_or_default();
    body["messages"]
        .as_array()
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| message["role"] == "user")
        })
        .map(|message| match &message["content"] {
            Value::String(text) => text.clone(),
            content => content.to_string(),
        })
        .unwrap_or_default()
}

async fn anthropic_turn(stop_reason: &str) -> Result<Turn, Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(anthropic_stream(stop_reason))
        .mount(&server)
        .await;
    let client: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("bug517-offline")
        .base_url(server.uri())
        .build()?;
    turn(client.completion_model("claude-sonnet-4-5"), "claude").await
}

#[tokio::test]
#[serial_test::serial]
async fn anthropic_streaming_propagates_max_tokens_through_final_response() -> TestResult {
    // @step Given the agent is using the Anthropic provider in streaming mode
    // @step And the model hits the max_tokens limit during text generation
    // @step When the Anthropic SSE stream emits a message_delta with stop_reason "max_tokens"
    let turn = anthropic_turn("max_tokens").await?;

    // @step Then the FinalResponse yielded from the streaming pipeline contains StopReason::MaxTokens
    assert_eq!(turn.finished, vec![Some("max_tokens".to_string())]);

    // @step And the stream_loop displays a truncation warning to the user
    assert!(turn.log.contains(TRUNCATION_WARNING), "log:\n{}", turn.log);

    // @step And the persisted AssistantMessage stop_reason is "max_tokens"
    assert_eq!(turn.persisted, vec!["max_tokens".to_string()]);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn truncated_tool_calls_produce_an_informative_error_identifying_truncation() -> TestResult {
    // @step Given the agent is using any provider in streaming mode
    // @step And the model hits max_tokens while generating a tool call JSON body
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(TruncatedThenAnswered(AtomicUsize::new(0)))
        .mount(&server)
        .await;
    let client: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("bug517-offline")
        .base_url(server.uri())
        .build()?;

    // @step When the accumulated tool call arguments fail JSON parsing due to truncation
    let turn = turn(client.completion_model("claude-sonnet-4-5"), "claude").await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("requests are recorded")?;
    let sent_back = requests.get(1).map(last_user_text).unwrap_or_default();

    // @step Then the message sent back to the model says the tool call was truncated because it exceeded the output token limit
    assert!(
        sent_back.contains("tool call was truncated because it exceeded the output token limit"),
        "message sent back: {sent_back}"
    );

    // @step And the message names the truncated tool and the max_tokens limit, not only a generic JSON parse failure
    assert!(
        sent_back.contains("Truncated tool: Write"),
        "message sent back: {sent_back}"
    );
    assert!(
        sent_back.contains("max_tokens"),
        "message sent back: {sent_back}"
    );

    // @step And the agent loop continues to allow the model to retry
    assert_eq!(
        requests.len(),
        2,
        "the model gets exactly one retry request"
    );
    assert_eq!(turn.finished.last(), Some(&Some("end_turn".to_string())));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn normal_end_turn_completion_has_no_truncation_warning_and_correct_persistence() -> TestResult
{
    // @step Given the agent is using any provider in streaming mode
    // @step And the model completes its response naturally with stop_reason "end_turn"
    // @step When the FinalResponse is yielded from the streaming pipeline
    let turn = anthropic_turn("end_turn").await?;
    assert_eq!(turn.finished, vec![Some("end_turn".to_string())]);

    // @step Then no truncation warning is shown to the user
    assert!(!turn.log.contains(TRUNCATION_WARNING), "log:\n{}", turn.log);

    // @step And the persisted AssistantMessage stop_reason is "end_turn"
    assert_eq!(turn.persisted, vec!["end_turn".to_string()]);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn openai_streaming_propagates_max_tokens_instead_of_hardcoding_end_turn() -> TestResult {
    // @step Given the agent is using the OpenAI provider in streaming mode
    // @step And the model hits the max_tokens limit during text generation
    // @step When the OpenAI SSE stream emits a response with finish_reason "length"
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(openai_stream("length"))
        .mount(&server)
        .await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("bug517-offline")
            .base_url(server.uri())
            .build()?;
    let turn = turn(client.completion_model("gpt-test"), "openai").await?;

    // @step Then the FinalResponse contains StopReason::MaxTokens
    assert_eq!(turn.finished, vec![Some("max_tokens".to_string())]);

    // @step And the persisted AssistantMessage stop_reason is "max_tokens"
    assert_eq!(turn.persisted, vec!["max_tokens".to_string()]);

    // @step And the stop_reason is not hardcoded to "end_turn"
    assert!(!turn.persisted.iter().any(|reason| reason == "end_turn"));
    Ok(())
}
