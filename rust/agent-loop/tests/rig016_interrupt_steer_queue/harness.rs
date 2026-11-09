//! RIG-016 harness: a real agent loop over an offline OpenAI-compatible
//! wiremock endpoint whose replies are scripted per provider request.

#[path = "../support/terminal_recovery.rs"]
pub mod terminal;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// One scripted provider reply.
#[derive(Clone)]
pub enum Reply {
    /// Stream `text` and stop, after `delay_ms`.
    Text(&'static str, u64),
    /// Ask for one Bash tool call running `command`.
    Bash(&'static str),
    /// BUG-546: ask for two Bash tool calls in one response.
    BashPair(&'static str, &'static str),
}

pub struct Harness {
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    pub chunks: broadcast::Receiver<StreamChunk>,
    task: tokio::task::JoinHandle<()>,
    _directory: tempfile::TempDir,
    _injection: mpsc::Sender<codelet_tools::McpInjection>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn sse(delta: serde_json::Value, finish: &str) -> String {
    let chunk = serde_json::json!({
        "id":"rig016","object":"chat.completion.chunk","created":1,"model":"o3",
        "choices":[{"index":0,"delta":delta,"finish_reason":finish}],
        "usage":{"prompt_tokens":100,"completion_tokens":5,"total_tokens":105}
    });
    format!("data: {chunk}\n\ndata: [DONE]\n\n")
}

fn respond(reply: &Reply) -> ResponseTemplate {
    let (body, delay) = match reply {
        Reply::Text(text, delay) => (
            sse(
                serde_json::json!({"role":"assistant","content":text}),
                "stop",
            ),
            *delay,
        ),
        Reply::Bash(command) => {
            let arguments =
                serde_json::json!({"work_unit_id":"NONE","command":command}).to_string();
            let call = serde_json::json!({"role":"assistant","tool_calls":[{"index":0,"id":"rig016-call","type":"function","function":{"name":"Bash","arguments":arguments}}]});
            (sse(call, "tool_calls"), 0)
        }
        Reply::BashPair(first, second) => {
            let tool = |index: usize, command: &str| {
                let arguments =
                    serde_json::json!({"work_unit_id":"NONE","command":command}).to_string();
                serde_json::json!({"index":index,"id":format!("bug546-call-{}", index + 1),"type":"function","function":{"name":"Bash","arguments":arguments}})
            };
            let calls = serde_json::json!({"role":"assistant","tool_calls":[tool(0, first), tool(1, second)]});
            (sse(calls, "tool_calls"), 0)
        }
    };
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
        .set_delay(Duration::from_millis(delay))
}

impl Harness {
    pub async fn new(script: Vec<Reply>) -> Self {
        let (directory, session, input) = terminal::fixture();
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )
        .expect("trusted standalone agent context");
        let server = MockServer::start().await;
        let served = Arc::new(AtomicUsize::new(0));
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |_: &wiremock::Request| {
                let index = served.fetch_add(1, Ordering::SeqCst);
                let fallback = Reply::Text("unscripted reply", 0);
                respond(script.get(index).unwrap_or(&fallback))
            })
            .mount(&server)
            .await;
        std::env::set_var("OPENAI_API_KEY", "test-no-network");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        {
            let mut inner = session.inner.lock().await;
            inner.provider_manager_mut().set_facade_override(None);
            inner
                .provider_manager_mut()
                .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)
                .expect("selected serializer and physical model");
        }
        let mut manifest =
            codelet_core::persistence::load_session(session.id).expect("persisted session");
        manifest.provider = "openai".into();
        codelet_core::persistence::save_session(&manifest).expect("persist provider");
        let chunks = session.subscribe_to_stream();
        let (injection, injections) = mpsc::channel(4);
        let task = tokio::spawn(agent_loop(session.clone(), input, injections));
        Self {
            session,
            server,
            chunks,
            task,
            _directory: directory,
            _injection: injection,
            _admission: admission,
        }
    }

    /// Provider request bodies received so far, in order.
    pub async fn requests(&self) -> Vec<serde_json::Value> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter_map(|request| serde_json::from_slice(&request.body).ok())
            .collect()
    }

    /// Wait until `count` provider requests have arrived.
    pub async fn until_requests(&self, count: usize) {
        let arrived = codelet_test_helpers::deadlock_guard("the awaited block", async {
            while self.requests().await.len() < count {
                codelet_test_helpers::yield_now().await;
            }
        })
        .await;
        assert!(arrived.is_ok(), "expected {count} provider requests");
    }

    /// Collect chunks until `predicate` matches one; returns all seen.
    pub async fn until(&mut self, predicate: impl Fn(&StreamChunk) -> bool) -> Vec<StreamChunk> {
        let seen = codelet_test_helpers::deadlock_guard("the awaited block", async {
            let mut seen = Vec::new();
            loop {
                let Ok(chunk) = self.chunks.recv().await else {
                    continue;
                };
                let matched = predicate(&chunk);
                seen.push(chunk);
                if matched {
                    return seen;
                }
            }
        })
        .await;
        assert!(
            seen.is_ok(),
            "stream deadline; status={:?}",
            self.session.get_status()
        );
        seen.unwrap_or_default()
    }

    /// Wait for `count` Done chunks.
    pub async fn until_done(&mut self, count: usize) {
        for _ in 0..count {
            self.until(|chunk| matches!(chunk, StreamChunk::Done)).await;
        }
    }
}

/// `(role, text)` for every message of an OpenAI chat request body.
pub fn messages(body: &serde_json::Value) -> Vec<(String, String)> {
    body["messages"]
        .as_array()
        .map(|messages| {
            messages
                .iter()
                .map(|message| {
                    let role = message["role"].as_str().unwrap_or_default().to_owned();
                    let text = match &message["content"] {
                        serde_json::Value::String(text) => text.clone(),
                        serde_json::Value::Array(parts) => parts
                            .iter()
                            .filter_map(|part| part["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                        _ => String::new(),
                    };
                    (role, text)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The text of the request's last user message.
pub fn last_user(body: &serde_json::Value) -> String {
    messages(body)
        .into_iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, text)| text)
        .unwrap_or_default()
}

/// Whether any message of the request mentions `needle`.
pub fn mentions(body: &serde_json::Value, needle: &str) -> bool {
    messages(body).iter().any(|(_, text)| text.contains(needle))
}
