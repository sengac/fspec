//! BUG-614: a production agent loop whose provider replies are scripted.
//!
//! Ordinary inference requests pop the next [`Reply`]; compactor sub-agent
//! requests (they carry `Target session: <id>`) pop the next [`Candidate`].
//! `{end}` in a candidate is replaced by the last captured persisted turn the
//! operation covers; `{end}` in an inject_summary reply is replaced by the end
//! of the range the previous rejection asked the model to cover.

#[path = "terminal_recovery.rs"]
pub mod terminal;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::{BackgroundSession, IncomingMessage};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

pub enum Reply {
    Tool {
        name: &'static str,
        arguments: serde_json::Value,
    },
    Text(String),
}

impl Reply {
    pub fn inject_summary(content: &str) -> Self {
        Self::Tool {
            name: "inject_summary",
            arguments: serde_json::json!({"work_unit_id": "NONE", "content": content}),
        }
    }

    pub fn generate_compaction() -> Self {
        Self::Tool {
            name: "GenerateCompaction",
            arguments: serde_json::json!({"work_unit_id": "NONE"}),
        }
    }

    pub fn text(text: &str) -> Self {
        Self::Text(text.into())
    }
}

pub struct Candidate {
    pub text: String,
    pub finish_reason: &'static str,
}

impl Candidate {
    pub fn stop(text: &str) -> Self {
        Self {
            text: text.into(),
            finish_reason: "stop",
        }
    }

    pub fn length(text: &str) -> Self {
        Self {
            text: text.into(),
            finish_reason: "length",
        }
    }
}

type Queue<T> = Arc<Mutex<VecDeque<T>>>;

pub struct Scripted {
    pub directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    pub chunks: broadcast::Receiver<StreamChunk>,
    ordinary: Queue<Reply>,
    builder: Queue<Candidate>,
    task: tokio::task::JoinHandle<()>,
    _injection: mpsc::Sender<codelet_tools::McpInjection>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Scripted {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn stream(chunk: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

fn range_end(text: &str, marker: &str) -> Option<usize> {
    text.rsplit(marker)
        .next()
        .filter(|_| text.contains(marker))
        .map(|tail| {
            tail.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .and_then(|digits| digits.parse().ok())
}

fn respond(
    request: &Request,
    target: &str,
    ordinary: &Queue<Reply>,
    builder: &Queue<Candidate>,
) -> ResponseTemplate {
    let text = String::from_utf8_lossy(&request.body).into_owned();
    if text.contains("Extract learnings from this session summary") {
        return ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error":{"message":"optional extraction disabled for this fixture"}
        }));
    }
    if text.contains(target) {
        let Some(candidate) = builder.lock().expect("builder script").pop_front() else {
            return ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error":{"message":"unscripted builder request"}}),
            );
        };
        let covered = range_end(&text, "This operation covers exactly ").map(|count| count - 1);
        let content = match covered {
            Some(end) => candidate.text.replace("{end}", &end.to_string()),
            None => candidate.text,
        };
        return ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id":"builder","object":"chat.completion","created":1,"model":"o3",
            "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":candidate.finish_reason}],
            "usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140}
        }));
    }
    let Some(reply) = ordinary.lock().expect("ordinary script").pop_front() else {
        return ResponseTemplate::new(400)
            .set_body_json(serde_json::json!({"error":{"message":"unscripted ordinary request"}}));
    };
    let usage = serde_json::json!({"prompt_tokens":100,"completion_tokens":10,"total_tokens":110});
    match reply {
        Reply::Tool {
            name,
            mut arguments,
        } => {
            if let (Some(content), Some(end)) = (
                arguments["content"].as_str().map(str::to_owned),
                range_end(&text, "cover persisted turns 0-"),
            ) {
                arguments["content"] = content.replace("{end}", &end.to_string()).into();
            }
            stream(&serde_json::json!({
                "id":"scripted","object":"chat.completion.chunk","created":1,"model":"o3",
                "choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":format!("call-{}", uuid::Uuid::new_v4().simple()),"type":"function","function":{"name":name,"arguments":arguments.to_string()}}]},"finish_reason":"tool_calls"}],
                "usage":usage
            }))
        }
        Reply::Text(content) => stream(&serde_json::json!({
            "id":"scripted","object":"chat.completion.chunk","created":1,"model":"o3",
            "choices":[{"index":0,"delta":{"role":"assistant","content":content},"finish_reason":"stop"}],
            "usage":usage
        })),
    }
}

impl Scripted {
    pub async fn new() -> Self {
        let (directory, session, input) = terminal::fixture();
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )
        .expect("trusted standalone agent context");
        let server = MockServer::start().await;
        let target = format!("Target session: {}", session.id);
        let ordinary: Queue<Reply> = Arc::default();
        let builder: Queue<Candidate> = Arc::default();
        let (replies, candidates) = (ordinary.clone(), builder.clone());
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |request: &Request| respond(request, &target, &replies, &candidates))
            .mount(&server)
            .await;
        std::env::set_var("OPENAI_API_KEY", "test-no-network");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        {
            let mut inner = session.inner.lock().await;
            inner
                .messages
                .push(rig::message::Message::user("committed evidence"));
            inner
                .messages
                .push(rig::message::Message::assistant("committed response"));
            inner.provider_manager_mut().set_facade_override(None);
            inner
                .provider_manager_mut()
                .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)
                .expect("selected serializer and physical model");
        }
        let mut manifest = codelet_core::persistence::load_session(session.id)
            .expect("persisted selected session");
        manifest.provider = "openai".into();
        codelet_core::persistence::save_session(&manifest).expect("persist selected provider");
        let chunks = session.subscribe_to_stream();
        let (injection, injections) = mpsc::channel(4);
        let task = tokio::spawn(agent_loop(session.clone(), input, injections));
        Self {
            directory,
            session,
            server,
            chunks,
            ordinary,
            builder,
            task,
            _injection: injection,
            _admission: admission,
        }
    }

    pub fn script(&self, replies: Vec<Reply>) {
        self.ordinary
            .lock()
            .expect("ordinary script")
            .extend(replies);
    }

    pub fn candidates(&self, candidates: Vec<Candidate>) {
        self.builder
            .lock()
            .expect("builder script")
            .extend(candidates);
    }

    pub fn send(&self, text: &str) {
        self.session
            .send_input(text.into(), None)
            .expect("submit user input");
    }

    /// A sub-agent's AgentManager report, delivered as a supervisor message.
    pub fn report(&self, text: &str) {
        self.session
            .receive_incoming_message(
                IncomingMessage::new(
                    uuid::Uuid::new_v4().to_string(),
                    "worker".into(),
                    text.into(),
                )
                .expect("AgentManager report"),
            )
            .expect("deliver AgentManager report");
    }

    pub async fn until_done(&mut self) -> Vec<StreamChunk> {
        let chunks = &mut self.chunks;
        codelet_test_helpers::deadlock_guard("the scripted turn", async {
            let mut observed = Vec::new();
            loop {
                let chunk = chunks.recv().await.expect("live stream");
                let done = matches!(chunk, StreamChunk::Done);
                observed.push(chunk);
                if done {
                    return observed;
                }
            }
        })
        .await
        .expect("turn finishes")
    }

    async fn requests(&self, builder: bool) -> Vec<serde_json::Value> {
        let target = format!("Target session: {}", self.session.id);
        self.server
            .received_requests()
            .await
            .expect("recorded requests")
            .iter()
            .filter(|request| {
                let text = String::from_utf8_lossy(&request.body);
                !text.contains("Extract learnings from this session summary")
                    && text.contains(&target) == builder
            })
            .map(|request| request.body_json().expect("request JSON"))
            .collect()
    }

    /// Bodies of the ordinary (main model) inference requests, in order.
    pub async fn ordinary_requests(&self) -> Vec<serde_json::Value> {
        self.requests(false).await
    }

    /// Bodies of the compactor sub-agent requests, in order.
    pub async fn builder_requests(&self) -> Vec<serde_json::Value> {
        self.requests(true).await
    }

    /// The live session, failing instead of waiting forever for its lock.
    pub async fn inner(&self) -> tokio::sync::MutexGuard<'_, codelet_cli::session::Session> {
        codelet_test_helpers::deadlock_guard("the session lock", self.session.inner.lock())
            .await
            .expect("the agent loop releases the session")
    }

    pub async fn inference_ready(&self) -> bool {
        self.inner()
            .await
            .ensure_compaction_inference_ready()
            .is_ok()
    }

    pub fn committed_summary(&self) -> Option<String> {
        codelet_core::persistence::load_session(self.session.id)
            .expect("durable manifest")
            .compaction
            .map(|state| state.summary)
    }
}

/// Text of one chat message's content (string or content parts).
pub fn content_text(content: &serde_json::Value) -> String {
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

/// The (role, text) messages of a chat-completions request body.
pub fn messages(body: &serde_json::Value) -> Vec<(String, String)> {
    body["messages"]
        .as_array()
        .map(|messages| {
            messages
                .iter()
                .map(|message| {
                    (
                        message["role"].as_str().unwrap_or_default().to_owned(),
                        content_text(&message["content"]),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn errors(chunks: &[StreamChunk]) -> Vec<String> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::Error { error, .. } => Some(error.clone()),
            _ => None,
        })
        .collect()
}

pub fn texts(chunks: &[StreamChunk]) -> String {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

pub fn notifications(chunks: &[StreamChunk]) -> Vec<String> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::UserNotification { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect()
}
