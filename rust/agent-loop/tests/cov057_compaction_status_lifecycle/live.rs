//! Loopback live agent loop for the compaction status lifecycle scenarios.
//!
//! The real `agent_loop` drives the real stream loop, compaction builder
//! sub-agent and `BackgroundOutput` against an OpenAI-compatible wiremock
//! endpoint, so the observed chunk sequence is the one production publishes.

use super::terminal;
use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

/// Label of the DAG node the compaction builder commits.
pub const LABEL: &str = "Committed status evidence";
/// Text of the resumed (or normal) assistant answer.
pub const ANSWER: &str = "Original task finished";
/// Token count above the 160000-token threshold of a 200000/4096 model.
pub const ABOVE_THRESHOLD: u64 = 170_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// The turn never crosses the compaction threshold.
    Normal,
    /// The tracked context already exceeds the threshold before the API call.
    PrePrompt,
    /// The first response finishes above the threshold, so the compaction
    /// hook cancels the follow-up request.
    PostResponse,
}

pub struct Live {
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    pub builders: Arc<AtomicUsize>,
    chunks: broadcast::Receiver<StreamChunk>,
    task: tokio::task::JoinHandle<()>,
    _directory: tempfile::TempDir,
    _injection: mpsc::Sender<codelet_tools::McpInjection>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Live {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn stream(id: &str, delta: serde_json::Value, finish: &str, prompt: u64) -> ResponseTemplate {
    let chunk = json!({
        "id": id, "object": "chat.completion.chunk", "created": 1, "model": "o3",
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
        "usage": {"prompt_tokens": prompt, "completion_tokens": 10, "total_tokens": prompt + 10}
    });
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

fn builder(body: &str) -> ResponseTemplate {
    let end = body
        .split("This operation covers exactly ")
        .nth(1)
        .and_then(|tail| tail.split_whitespace().next())
        .and_then(|count| count.parse::<usize>().ok())
        .and_then(|count| count.checked_sub(1))
        .expect("captured prefix size");
    let dag = format!(
        "<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"{LABEL}\">\nThe evidence is preserved. Continue the original task without repeating it.\n</dag-node>"
    );
    ResponseTemplate::new(200).set_body_json(json!({
        "id": "builder", "object": "chat.completion", "created": 1, "model": "o3",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": dag}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 100, "completion_tokens": 40, "total_tokens": 140}
    }))
}

async fn mount(server: &MockServer, trigger: Trigger, target: String, builders: Arc<AtomicUsize>) {
    let inference = AtomicUsize::new(0);
    Mock::given(method("POST"))
        .respond_with(move |request: &Request| {
            let body = String::from_utf8_lossy(&request.body);
            if body.contains("Extract learnings from this session summary") {
                return ResponseTemplate::new(400).set_body_json(json!({
                    "error": {"message": "optional extraction disabled for this fixture"}
                }));
            }
            if body.contains(&target) {
                builders.fetch_add(1, Ordering::SeqCst);
                return builder(&body);
            }
            let first = inference.fetch_add(1, Ordering::SeqCst) == 0;
            if trigger == Trigger::PostResponse && first {
                let arguments = json!({
                    "work_unit_id": "NONE", "command": "printf 'status evidence\\n'"
                })
                .to_string();
                let call = json!({"index": 0, "id": "status-tool", "type": "function",
                    "function": {"name": "Bash", "arguments": arguments}});
                let delta = json!({"role": "assistant", "tool_calls": [call]});
                return stream("work", delta, "tool_calls", ABOVE_THRESHOLD);
            }
            let delta = json!({"role": "assistant", "content": ANSWER});
            stream("answer", delta, "stop", 100)
        })
        .mount(server)
        .await;
}

impl Live {
    pub async fn new(trigger: Trigger) -> Self {
        let (directory, session, input) = terminal::fixture();
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )
        .expect("trusted standalone agent context");
        let server = MockServer::start().await;
        let builders = Arc::new(AtomicUsize::new(0));
        let target = format!("Target session: {}", session.id);
        mount(&server, trigger, target, builders.clone()).await;
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
            if trigger == Trigger::PrePrompt {
                inner.token_tracker.input_tokens = ABOVE_THRESHOLD;
            }
        }
        let mut manifest = codelet_core::persistence::load_session(session.id)
            .expect("persisted selected session");
        manifest.provider = "openai".into();
        codelet_core::persistence::save_session(&manifest).expect("persist selected provider");
        let chunks = session.subscribe_to_stream();
        let (injection, injections) = mpsc::channel(4);
        let task = tokio::spawn(agent_loop(session.clone(), input, injections));
        Self {
            session,
            server,
            builders,
            chunks,
            task,
            _directory: directory,
            _injection: injection,
            _admission: admission,
        }
    }

    /// The marker that identifies compaction builder requests for this session.
    pub fn target(&self) -> String {
        format!("Target session: {}", self.session.id)
    }

    /// Submit the original task and collect every chunk of the turn,
    /// including anything published by turn reconciliation after `Done`.
    pub async fn turn(&mut self) -> Vec<StreamChunk> {
        self.session
            .send_input("finish remaining work".into(), None)
            .expect("submit original task");
        let chunks = &mut self.chunks;
        let mut observed = codelet_test_helpers::deadlock_guard("the awaited block", async {
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
        .expect("turn finishes within the bounded deadline");
        // The agent loop owns the inner session for the whole turn, including
        // post-Done reconciliation; acquiring it proves the turn has settled.
        drop(self.session.inner.lock().await);
        while let Ok(chunk) = self.chunks.try_recv() {
            observed.push(chunk);
        }
        observed
    }

    /// Bodies of every provider request the loopback server received.
    pub async fn bodies(&self) -> Vec<String> {
        self.server
            .received_requests()
            .await
            .expect("recorded provider requests")
            .iter()
            .map(|request| String::from_utf8_lossy(&request.body).into_owned())
            .collect()
    }

    /// Bodies of ordinary inference requests (not builder or extraction calls).
    pub async fn inference_bodies(&self) -> Vec<String> {
        let target = self.target();
        self.bodies()
            .await
            .into_iter()
            .filter(|body| {
                !body.contains(&target) && !body.contains("Extract learnings from this session")
            })
            .collect()
    }
}
