use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct Fixture {
    pub sessions: Vec<Arc<BackgroundSession>>,
    pub builders: Arc<[AtomicUsize; 2]>,
    pub inferences: Arc<[AtomicUsize; 2]>,
    events: broadcast::Receiver<(SessionId, StreamChunk)>,
    manager: Arc<SessionManager>,
    _directory: tempfile::TempDir,
    _server: MockServer,
}

impl Fixture {
    pub async fn new() -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../../../fspec-tui/tests/fixtures/prov101_models.json"),
        )?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().into())?;
        let server = MockServer::start().await;
        std::env::set_var("OPENAI_API_KEY", "bug210-no-network");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        let manager = SessionManager::new_arc();
        manager.set_hooks(Arc::new(
            codelet_napi::session_hooks::NapiSessionManagerHooks,
        ));
        let mut sessions = Vec::new();
        for index in 0..2 {
            let id = manager
                .create_session("openai/o3", &directory.path().to_string_lossy())
                .await?;
            let session = manager.get_session(&id)?;
            let user = format!("evidence-session-{index}");
            let mut manifest = codelet_core::persistence::load_session(session.id)?;
            codelet_core::persistence::append_message(&mut manifest, "user", &user)?;
            codelet_core::persistence::append_message(
                &mut manifest,
                "assistant",
                "completed work",
            )?;
            {
                let mut inner = session.inner.lock().await;
                inner.messages = vec![
                    rig::message::Message::user(user),
                    rig::message::Message::assistant("completed work"),
                ];
                inner.provider_manager_mut().set_facade_override(None);
                inner.provider_manager_mut().set_model_direct(
                    "openai",
                    "o3",
                    Some(200_000),
                    Some(4096),
                    None,
                )?;
            }
            sessions.push(session);
        }
        let builders = Arc::new([AtomicUsize::new(0), AtomicUsize::new(0)]);
        let inferences = Arc::new([AtomicUsize::new(0), AtomicUsize::new(0)]);
        let targets: Vec<_> = sessions
            .iter()
            .map(|session| session.id.to_string())
            .collect();
        let builder_counts = builders.clone();
        let inference_counts = inferences.clone();
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |request: &wiremock::Request| {
                let text = String::from_utf8_lossy(&request.body);
                if text.contains("Extract learnings from this session summary") {
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"message":"optional extraction disabled"}
                    }));
                }
                if let Some(index) = targets.iter().position(|target| text.contains(&format!("Target session: {target}"))) {
                    builder_counts[index].fetch_add(1, Ordering::SeqCst);
                    let end = text.split("This operation covers exactly ")
                        .nth(1)
                        .and_then(|tail| tail.split_whitespace().next())
                        .and_then(|count| count.parse::<usize>().ok())
                        .and_then(|count| count.checked_sub(1));
                    let Some(end) = end else {
                        return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                            "error":{"message":"missing captured history boundary"}
                        }));
                    };
                    let dag = format!("<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"evidence-session-{index}\">\nCompleted work for evidence-session-{index}. Continue its remaining task.\n</dag-node>");
                    return ResponseTemplate::new(200)
                        .set_delay(Duration::from_millis(30))
                        .set_body_json(serde_json::json!({
                            "id":"builder","object":"chat.completion","created":1,"model":"o3",
                            "choices":[{"index":0,"message":{"role":"assistant","content":dag},"finish_reason":"stop"}],
                            "usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens":140}
                        }));
                }
                let Some(index) = (0..2).find(|index| text.contains(&format!("evidence-session-{index}"))) else {
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"message":"request lost its originating task identity"}
                    }));
                };
                if inference_counts[index].fetch_add(1, Ordering::SeqCst) == 0 {
                    return ResponseTemplate::new(400).set_body_json(serde_json::json!({
                        "error":{"code":"context_length_exceeded","message":"ordinary request exceeds the context window"}
                    }));
                }
                let chunk = serde_json::json!({
                    "id":"continued","object":"chat.completion.chunk","created":1,"model":"o3",
                    "choices":[{"index":0,"delta":{"role":"assistant","content":format!("finished-session-{index}")},"finish_reason":"stop"}],
                    "usage":{"prompt_tokens":100,"completion_tokens":5,"total_tokens":105}
                });
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
            })
            .mount(&server)
            .await;
        let events = manager.chunks_tx().subscribe();
        Ok(Self {
            sessions,
            builders,
            inferences,
            events,
            manager,
            _directory: directory,
            _server: server,
        })
    }

    pub fn send(&self) -> TestResult {
        for (index, session) in self.sessions.iter().enumerate() {
            session.send_input(format!("finish evidence-session-{index}"), None)?;
        }
        Ok(())
    }

    pub async fn until_done(&mut self) -> TestResult<Vec<(SessionId, StreamChunk)>> {
        Ok(tokio::time::timeout(Duration::from_secs(20), async {
            let mut events = Vec::new();
            let mut finished = std::collections::HashSet::new();
            while finished.len() < self.sessions.len() {
                let (origin, chunk) = self.events.recv().await?;
                if matches!(chunk, StreamChunk::Done) {
                    finished.insert(origin.value.clone());
                }
                events.push((origin, chunk));
            }
            Ok::<_, broadcast::error::RecvError>(events)
        })
        .await??)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for session in &self.sessions {
            let _ = self.manager.destroy_session(&session.id.to_string());
        }
    }
}
