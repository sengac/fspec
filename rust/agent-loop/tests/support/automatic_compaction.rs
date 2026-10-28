#[path = "terminal_recovery.rs"]
pub mod terminal;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct Harness {
    pub directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub server: MockServer,
    pub chunks: broadcast::Receiver<StreamChunk>,
    pub builders: Arc<AtomicUsize>,
    pub fail_builder: Arc<AtomicBool>,
    pub infer: Arc<AtomicUsize>,
    pub cancel_on_summary: Arc<AtomicBool>,
    pub completed_tool: Arc<AtomicBool>,
    pub self_compaction: Arc<AtomicBool>,
    pub loop_abort: Arc<AtomicBool>,
    pub task: tokio::task::JoinHandle<()>,
    _injection: Option<mpsc::Sender<codelet_tools::McpInjection>>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Harness {
    pub async fn new(repeat: bool, fail: bool, delay: bool) -> Self {
        Self::with_hooks(repeat, fail, delay, None).await
    }

    pub async fn with_hooks(
        repeat: bool,
        fail: bool,
        delay: bool,
        hooks: Option<&dyn codelet_sessions::session_manager::SessionManagerHooks>,
    ) -> Self {
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
        let allow_success = Arc::new(AtomicBool::new(!repeat));
        let builder_count = builders.clone();
        let fail_builder = Arc::new(AtomicBool::new(fail));
        let builder_failure = fail_builder.clone();
        let infer_count = infer.clone();
        let succeed = allow_success.clone();
        let cancel_on_summary = Arc::new(AtomicBool::new(false));
        let cancel = cancel_on_summary.clone();
        let weak_session = Arc::downgrade(&session);
        let completed_tool = Arc::new(AtomicBool::new(false));
        let execute_tool = completed_tool.clone();
        let self_compaction = Arc::new(AtomicBool::new(false));
        let submit_summary = self_compaction.clone();
        let loop_abort = Arc::new(AtomicBool::new(false));
        let abort_loop = loop_abort.clone();
        let effect = directory.path().join("completed-command.txt");
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |request: &wiremock::Request| {
                let body: serde_json::Value =
                    serde_json::from_slice(&request.body).expect("provider request JSON");
                let text = String::from_utf8_lossy(&request.body);
                if text.contains("Extract learnings from this session summary") {
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"message":"optional extraction disabled for this fixture"}
                    }));
                }
                if text.contains(&target) {
                    builder_count.fetch_add(1, Ordering::SeqCst);
                    if builder_failure.load(Ordering::SeqCst) {
                        return ResponseTemplate::new(400).set_body_json(
                            serde_json::json!({"error":{"message":"summary service unavailable; retry later"}}),
                        );
                    }
                    let end = text
                        .split("This operation covers exactly ")
                        .nth(1)
                        .and_then(|tail| tail.split_whitespace().next())
                        .and_then(|count| count.parse::<usize>().ok())
                        .expect("captured prefix size") - 1;
                    let dag = format!(
                        "<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"Completed evidence\">\nThe command already ran. Continue the remaining original task, without repeating it.\n</dag-node>"
                    );
                    let response = ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "id":"builder","object":"chat.completion","created":1,"model":"o3",
                        "choices":[{"index":0,"message":{"role":"assistant","content":dag},"finish_reason":"stop"}],
                        "usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140}
                    }));
                    return if delay {
                        response.set_delay(Duration::from_secs(30))
                    } else {
                        response
                    };
                }
                let request_number = infer_count.fetch_add(1, Ordering::SeqCst);
                if request_number == 0 && abort_loop.load(Ordering::SeqCst) {
                    let prose = "The architecture of the streaming loop detector relies on word level tokenization with a bounded sliding window that evaluates four distinct repetition signals in order of specificity before escalating";
                    let text = format!("{prose} {}", "the model thinks that ".repeat(100));
                    let mut events = String::new();
                    for word in text.split_whitespace() {
                        let chunk = serde_json::json!({
                            "id":"loop","object":"chat.completion.chunk","created":1,"model":"o3",
                            "choices":[{"index":0,"delta":{"role":"assistant","content":format!("{word} ")},"finish_reason":null}]
                        });
                        events.push_str(&format!("data: {chunk}\n\n"));
                    }
                    events.push_str("data: [DONE]\n\n");
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(events);
                }
                if request_number == 0 && submit_summary.load(Ordering::SeqCst) {
                    let chunk = serde_json::json!({
                        "id":"self-summary","object":"chat.completion.chunk","created":1,"model":"o3",
                        "choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"summary","type":"function","function":{"name":"GenerateCompaction","arguments":serde_json::json!({"work_unit_id":"NONE"}).to_string()}}]},"finish_reason":"tool_calls"}],
                        "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
                    });
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"));
                }
                let with_tool = execute_tool.load(Ordering::SeqCst);
                if with_tool && request_number == 0 {
                    let arguments = serde_json::json!({
                        "work_unit_id":"NONE","command":format!("printf 'executed\\n' >> '{}'", effect.display())
                    }).to_string();
                    let chunk = serde_json::json!({
                        "id":"work","object":"chat.completion.chunk","created":1,"model":"o3",
                        "choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"completed-command","type":"function","function":{"name":"Bash","arguments":arguments}}]},"finish_reason":"tool_calls"}],
                        "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
                    });
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"));
                }
                if request_number == usize::from(with_tool) || !succeed.load(Ordering::SeqCst) {
                    if cancel.load(Ordering::SeqCst) {
                        let target = weak_session.clone();
                        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
                            weak_session.upgrade().expect("live target").id,
                            Some(Arc::new(move |_, source| {
                                let target = target.clone();
                                Box::pin(async move {
                                    target.upgrade().expect("live target").interrupt();
                                    Ok(format!("<dag-node depth=\"D1\" turns=\"0-{}\" label=\"Completed evidence\">\nOriginal task cancelled.\n</dag-node>", source.prefix.len() - 1))
                                })
                            })),
                        );
                    }
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"code":"context_length_exceeded","message":"ordinary request exceeds the context window"}
                    }));
                }
                assert_eq!(body["stream"], true);
                let chunk = serde_json::json!({
                    "id":"continued","object":"chat.completion.chunk","created":1,"model":"o3",
                    "choices":[{"index":0,"delta":{"role":"assistant","content":"Original task finished"},"finish_reason":"stop"}],
                    "usage":{"prompt_tokens":100,"completion_tokens":5,"total_tokens":105}
                });
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
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
        }
        let mut manifest = codelet_core::persistence::load_session(session.id)
            .expect("persisted selected session");
        manifest.provider = "openai".into();
        codelet_core::persistence::save_session(&manifest).expect("persist selected provider");
        let chunks = session.subscribe_to_stream();
        let (injection, injections) = mpsc::channel(4);
        let task = if let Some(hooks) = hooks {
            hooks.spawn_agent_loop(session.clone(), input, injections);
            tokio::spawn(std::future::pending())
        } else {
            tokio::spawn(agent_loop(session.clone(), input, injections))
        };
        Self {
            directory,
            session,
            server,
            chunks,
            builders,
            fail_builder,
            infer,
            cancel_on_summary,
            completed_tool,
            self_compaction,
            loop_abort,
            task,
            _injection: Some(injection),
            _admission: admission,
        }
    }

    pub fn send(&self) {
        self.session
            .send_input("finish remaining work".into(), None)
            .expect("submit original task");
    }

    pub fn close_injections(&mut self) {
        self._injection.take();
    }

    pub async fn until_done(&mut self) -> Vec<StreamChunk> {
        let result = tokio::time::timeout(Duration::from_secs(20), async {
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
            "turn deadline: inference={}, builders={}, status={:?}, chunks={:?}",
            self.infer.load(Ordering::SeqCst),
            self.builders.load(Ordering::SeqCst),
            self.session.get_status(),
            self.session.get_buffered_output(30)
        );
        result.expect("turn finishes within bounded recovery deadline")
    }

    pub fn assert_originals_available(&self) {
        let manifest =
            codelet_core::persistence::load_session(self.session.id).expect("durable originals");
        assert!(manifest.messages.len() >= 2);
        assert!(self.directory.path().exists());
    }
}
