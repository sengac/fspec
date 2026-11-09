use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_core::RigAgent;
use codelet_tools::GenerateCompactionTool;
use rig::client::CompletionClient;
use std::sync::{atomic::AtomicBool, Arc, Mutex, OnceLock};
use wiremock::{matchers::method, Mock, MockServer, Request, Respond, ResponseTemplate};

pub fn node(depth: &str, start: usize, end: usize) -> String {
    format!("<dag-node depth=\"{depth}\" turns=\"{start}-{end}\" label=\"Work\">completed evidence</dag-node>")
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("diagnostic buffer poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn content_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .map(content_text)
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Object(fields) => {
            fields.get("text").map(content_text).unwrap_or_default()
        }
        _ => String::new(),
    }
}

pub fn hierarchy() -> String {
    format!(
        "{}{}{}",
        node("D2", 0, 1),
        node("D0", 0, 0),
        node("D0", 1, 1)
    )
}

struct Responses {
    candidates: Mutex<std::collections::VecDeque<String>>,
    streamed: std::sync::atomic::AtomicUsize,
}

impl Respond for Responses {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value = request.body_json().expect("provider request JSON");
        if body["stream"] == true
            && self
                .streamed
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                > 0
        {
            // BUG-614: a compaction that installs nothing leaves the turn
            // running; the caller then answers with text.
            let chunk = serde_json::json!({"id":"answer","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"continuing on the current context"},"finish_reason":"stop"}],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}});
            return ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"));
        }
        if body["stream"] == true {
            let chunks = [
                serde_json::json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"summary-call","type":"function","function":{"name":"GenerateCompaction","arguments":"{\"work_unit_id\":\"NONE\"}"}}]},"finish_reason":null}]}),
                serde_json::json!({"id":"submission","object":"chat.completion.chunk","created":1,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}}),
            ];
            return ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!(
                    "{}data: [DONE]\n\n",
                    chunks
                        .iter()
                        .map(|chunk| format!("data: {chunk}\n\n"))
                        .collect::<String>()
                ));
        }
        let content = self
            .candidates
            .lock()
            .expect("candidate sequence")
            .pop_front()
            .expect("only the specified candidate attempts may run");
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id":"candidate", "object":"chat.completion", "created":1, "model":"o3",
            "usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120},
            "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]
        }))
    }
}

#[derive(Default)]
struct Output;

impl StreamOutput for Output {
    fn emit(&self, _event: StreamEvent) {}
}

pub struct Outcome {
    pub result: Result<(), String>,
    pub prompts: Vec<String>,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub blocked: bool,
    pub summary: Option<String>,
    pub logs: String,
}

pub async fn drive(
    fixture: &super::durable::Fixture,
    server: &MockServer,
    candidates: Vec<String>,
) -> Outcome {
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(Responses {
            candidates: Mutex::new(candidates.into()),
            streamed: std::sync::atomic::AtomicUsize::new(0),
        })
        .mount(server)
        .await;
    let id = fixture.caller.id;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(id, fixture.directory.path().into())
            .expect("invoking project attribution");
    let original = codelet_core::persistence::load_session(id).expect("original durable source");
    let before = serde_json::to_value(&original).expect("original manifest");
    let originals =
        codelet_core::persistence::get_session_messages_full(&original).expect("originals");
    let mut inner = fixture.caller.inner.lock().await;
    let queue = register_generate_compaction_handler(
        id,
        &inner,
        fixture.directory.path().into(),
        fixture.caller.clone(),
        Some(fixture.manager.clone()),
    );
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline-test")
            .base_url(server.uri())
            .build()
            .expect("local client");
    let agent = RigAgent::new(
        client
            .agent("o3")
            .max_tokens(4096)
            .tool(GenerateCompactionTool::new(id))
            .build(),
        3,
    );
    static LOGS: OnceLock<LogBuffer> = OnceLock::new();
    let logs = LOGS.get_or_init(|| {
        let logs = LogBuffer::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("install serial diagnostic capture");
        logs
    });
    logs.0.lock().expect("clear diagnostic bytes").clear();
    let result = run_agent_stream(
        agent,
        "compact this session",
        &mut inner,
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
        Arc::new(tokio::sync::Notify::new()),
        &Output,
        id,
    )
    .await
    .map_err(|error| error.to_string());
    assert!(result.is_ok(), "the caller's turn never fails: {result:?}");
    let applied = queue
        .apply(&mut inner, &fixture.caller)
        .expect("apply only validated memory");
    let blocked = inner.ensure_compaction_inference_ready().is_err();
    let retained = codelet_core::persistence::load_session(id).expect("retained manifest");
    assert_eq!(
        serde_json::to_value(
            codelet_core::persistence::get_session_messages_full(&retained)
                .expect("retained originals")
        )
        .expect("retained original values"),
        serde_json::to_value(originals).expect("original values")
    );
    let after = serde_json::to_value(&retained).expect("retained durable manifest");
    let requests = server.received_requests().await.expect("provider requests");
    let mut prompts = Vec::new();
    let mut streamed = Vec::new();
    for request in requests {
        let body: serde_json::Value = request.body_json().expect("request JSON");
        if body["stream"] == true {
            streamed.push(body);
        } else {
            prompts.push(
                body["messages"]
                    .as_array()
                    .expect("chat messages")
                    .iter()
                    .map(|message| content_text(&message["content"]))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    }
    // BUG-614: a queued candidate stops the turn at the tool boundary; one
    // that installs nothing returns its error to the caller, which continues.
    let result = match streamed.get(1) {
        None => Ok(()),
        Some(body) => {
            let tool = body["messages"]
                .as_array()
                .and_then(|messages| {
                    messages
                        .iter()
                        .rev()
                        .find(|message| message["role"] == "tool")
                })
                .map(|message| content_text(&message["content"]))
                .expect("the caller receives the generation result");
            let value: serde_json::Value = serde_json::from_str(&tool).expect("result JSON");
            let value = match value.as_str() {
                Some(encoded) => serde_json::from_str(encoded).expect("result JSON"),
                None => value,
            };
            assert_eq!(value["status"], "rejected_final", "{value}");
            assert_eq!(value["installed"], false, "{value}");
            Err(value["error"].as_str().expect("exact error").to_owned())
        }
    };
    assert!(streamed.len() <= 2, "no further ordinary inference");
    assert_eq!(applied, result.is_ok());
    let logs = String::from_utf8(logs.0.lock().expect("diagnostic bytes").clone())
        .expect("UTF-8 diagnostics");
    Outcome {
        result,
        prompts,
        before,
        after,
        blocked,
        summary: retained.compaction.map(|state| state.summary),
        logs,
    }
}
