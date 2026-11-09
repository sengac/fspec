//! BUG-641 harness: an openai/o3 session (200k window, 4096 output, so an
//! 80% threshold of 160000 and a usable window of 195904) whose compaction
//! builder can fail, and whose ordinary turn can run one tool round first.
//! The including test file also loads support/automatic_compaction.rs as
//! `automatic`, whose terminal fixture this harness shares.

use crate::automatic::terminal;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const BUILDER_ERROR: &str = "summary service unavailable; retry later";
pub const ANSWER: &str = "Original task finished";

pub struct Harness {
    pub directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    pub chunks: broadcast::Receiver<StreamChunk>,
    pub builders: Arc<AtomicUsize>,
    pub fail_builder: Arc<AtomicBool>,
    pub infer: Arc<AtomicUsize>,
    /// When non-zero, the first ordinary request answers with a tool call
    /// reporting this many prompt tokens.
    pub tool_round_usage: Arc<AtomicU64>,
    pub task: tokio::task::JoinHandle<()>,
    _injection: mpsc::Sender<codelet_tools::McpInjection>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn stream(chunk: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

impl Harness {
    pub async fn new(tracked_input_tokens: u64) -> Self {
        let (directory, session, input) = terminal::fixture();
        let admission = codelet_tools::tool_admission::register_auxiliary(
            session.id,
            directory.path().to_path_buf(),
        )
        .expect("trusted standalone agent context");
        let server = MockServer::start().await;
        let target = format!("Target session: {}", session.id);
        let builders = Arc::new(AtomicUsize::new(0));
        let infer = Arc::new(AtomicUsize::new(0));
        let fail_builder = Arc::new(AtomicBool::new(true));
        let tool_round_usage = Arc::new(AtomicU64::new(0));
        let (builder_count, builder_failure, infer_count, tool_usage) = (
            builders.clone(),
            fail_builder.clone(),
            infer.clone(),
            tool_round_usage.clone(),
        );
        let effect = directory.path().join("tool-round.txt");
        // The answer reports the tracked usage, as a real provider would for
        // an uncompacted context.
        let answer_usage = tracked_input_tokens.max(100);
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |request: &wiremock::Request| {
                let text = String::from_utf8_lossy(&request.body);
                if text.contains("Extract learnings from this session summary") {
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"message":"optional extraction disabled for this fixture"}
                    }));
                }
                if text.contains(&target) {
                    builder_count.fetch_add(1, Ordering::SeqCst);
                    if builder_failure.load(Ordering::SeqCst) {
                        return ResponseTemplate::new(400)
                            .set_body_json(serde_json::json!({"error":{"message":BUILDER_ERROR}}));
                    }
                    let end = text
                        .split("This operation covers exactly ")
                        .nth(1)
                        .and_then(|tail| tail.split_whitespace().next())
                        .and_then(|count| count.parse::<usize>().ok())
                        .expect("captured prefix size")
                        - 1;
                    let dag = format!(
                        "<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"Completed evidence\">\nContinue the remaining original task.\n</dag-node>"
                    );
                    return ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "id":"builder","object":"chat.completion","created":1,"model":"o3",
                        "choices":[{"index":0,"message":{"role":"assistant","content":dag},"finish_reason":"stop"}],
                        "usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140}
                    }));
                }
                let request_number = infer_count.fetch_add(1, Ordering::SeqCst);
                let usage = tool_usage.load(Ordering::SeqCst);
                if request_number == 0 && usage > 0 {
                    let arguments = serde_json::json!({
                        "work_unit_id":"NONE",
                        "command":format!("printf 'ran\\n' >> '{}'", effect.display())
                    })
                    .to_string();
                    return stream(&serde_json::json!({
                        "id":"tool-round","object":"chat.completion.chunk","created":1,"model":"o3",
                        "choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"tool-round","type":"function","function":{"name":"Bash","arguments":arguments}}]},"finish_reason":"tool_calls"}],
                        "usage":{"prompt_tokens":usage,"completion_tokens":10,"total_tokens":usage + 10}
                    }));
                }
                stream(&serde_json::json!({
                    "id":"answer","object":"chat.completion.chunk","created":1,"model":"o3",
                    "choices":[{"index":0,"delta":{"role":"assistant","content":ANSWER},"finish_reason":"stop"}],
                    "usage":{"prompt_tokens":answer_usage,"completion_tokens":5,"total_tokens":answer_usage + 5}
                }))
            })
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
            inner.token_tracker.input_tokens = tracked_input_tokens;
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
            builders,
            fail_builder,
            infer,
            tool_round_usage,
            task,
            _injection: injection,
            _admission: admission,
        }
    }

    pub fn send(&self) {
        self.session
            .send_input("finish remaining work".into(), None)
            .expect("submit the user message");
    }

    pub async fn until_done(&mut self) -> Vec<StreamChunk> {
        let result = codelet_test_helpers::deadlock_guard("the BUG-641 turn", async {
            let mut observed = Vec::new();
            loop {
                let chunk = self.chunks.recv().await.expect("live stream");
                let done = matches!(chunk, StreamChunk::Done);
                observed.push(chunk);
                if done {
                    return observed;
                }
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "turn deadline: inference={}, builders={}, status={:?}",
            self.infer.load(Ordering::SeqCst),
            self.builders.load(Ordering::SeqCst),
            self.session.get_status(),
        );
        result.expect("turn finishes")
    }

    pub async fn is_held(&self) -> bool {
        self.session
            .inner
            .lock()
            .await
            .ensure_compaction_inference_ready()
            .is_err()
    }
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

pub fn errors(chunks: &[StreamChunk]) -> Vec<String> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::Error { error, .. } => Some(error.clone()),
            _ => None,
        })
        .collect()
}

pub fn answered(chunks: &[StreamChunk]) -> bool {
    chunks
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text.contains(ANSWER)))
}
