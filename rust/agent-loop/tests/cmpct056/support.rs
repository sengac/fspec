use std::sync::Arc;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::{background_session::BackgroundSession, session_manager::SessionManager};
use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool};
use rig::tool::Tool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Barrier;
use uuid::Uuid;

mod selected;

pub const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">completed evidence</dag-node>\n<dag-files>\n</dag-files>";

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub caller: Arc<BackgroundSession>,
    pub target: Arc<BackgroundSession>,
}

impl Fixture {
    pub async fn new(url: &str) -> Self {
        let directory = codelet_test_helpers::setup_temp_directory().expect("temporary data");
        std::fs::create_dir_all(directory.path().join("cache")).expect("model cache");
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../fixtures/prov101_models.json"),
        )
        .expect("offline model fixture");
        codelet_common::set_data_directory(directory.path().to_path_buf()).expect("redirect data");
        codelet_core::persistence::reset_stores_for_tests();
        std::env::set_var("OPENAI_API_KEY", "test-no-network");
        std::env::set_var("OPENAI_BASE_URL", url);
        std::env::set_var("OPENAI_MODEL", "o3");
        let manager = Arc::new(SessionManager::new());
        let project = directory.path().to_string_lossy().into_owned();
        let mut sessions = Vec::new();
        for name in ["caller", "target"] {
            let session_id = Uuid::new_v4().to_string();
            manager
                .create_session_with_id(&session_id, "openai/o3", &project, name)
                .await
                .expect("create live session");
            let session = manager.get_session(&session_id).expect("live session");
            let mut manifest =
                codelet_core::persistence::load_session(session.id).expect("durable live target");
            codelet_core::persistence::append_message(&mut manifest, "user", "committed evidence")
                .expect("persist user original");
            codelet_core::persistence::append_message(
                &mut manifest,
                "assistant",
                "committed response",
            )
            .expect("persist assistant original");
            {
                let mut inner = session.inner.lock().await;
                inner
                    .provider_manager_mut()
                    .set_model_direct("openai", "o3", Some(100_000), Some(4096), None)
                    .expect("explicit selected target limits");
                selected::install(&mut inner, session.id).await;
                inner.messages = vec![
                    rig::message::Message::user("committed evidence"),
                    rig::message::Message::assistant("committed response"),
                ];
            }
            session.set_status(SessionStatus::Idle);
            sessions.push(session);
        }
        Self {
            directory,
            manager,
            caller: sessions.remove(0),
            target: sessions.remove(0),
        }
    }

    pub fn tool(&self, caller: &Arc<BackgroundSession>) -> GenerateCompactionTool {
        let inner = caller
            .inner
            .try_lock()
            .expect("register outside stream ownership");
        register_generate_compaction_handler(
            caller.id,
            &inner,
            self.directory.path().into(),
            caller.clone(),
            Some(self.manager.clone()),
        );
        GenerateCompactionTool::new(caller.id)
    }

    pub fn start(&self) -> tokio::task::JoinHandle<Result<String, codelet_tools::ToolError>> {
        let tool = self.tool(&self.caller);
        let target = self.target.id.to_string();
        let caller = self.caller.id;
        let project = self.directory.path().to_path_buf();
        tokio::spawn(async move {
            let _admission = codelet_tools::tool_admission::register_auxiliary(caller, project)
                .expect("trusted synthetic compaction caller");
            tool.call(GenerateCompactionArgs {
                session_id: Some(target),
            })
            .await
        })
    }
}

pub async fn snapshot(session: &BackgroundSession) -> serde_json::Value {
    let inner = session.inner.lock().await;
    let manifest = codelet_core::persistence::load_session(session.id).expect("durable manifest");
    let originals = codelet_core::persistence::get_session_messages_full(&manifest)
        .expect("complete original messages");
    serde_json::json!({
        "messages": inner.messages,
        "turns": inner.turns,
        "tokens": inner.token_tracker,
        "manifest": manifest,
        "originals": originals,
    })
}

pub fn assert_clean(session: &BackgroundSession) {
    assert_eq!(session.get_status(), SessionStatus::Idle);
    assert!(!session
        .compaction_in_progress
        .load(std::sync::atomic::Ordering::SeqCst));
    assert!(session.get_compaction_progress().is_none());
}

pub fn assert_no_completion(chunks: &mut tokio::sync::broadcast::Receiver<StreamChunk>) {
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(chunk, StreamChunk::CompactionComplete { .. }));
    }
}

pub struct Server {
    pub url: String,
    pub arrived: Arc<Barrier>,
    pub release: Arc<Barrier>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub async fn new(status: u16) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("local listener");
        let url = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let arrived = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let request_barrier = arrived.clone();
        let response_barrier = release.clone();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("provider connects");
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket
                    .read(&mut buffer)
                    .await
                    .expect("read provider request");
                if count == 0 {
                    return;
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request[..header_end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .map(|value| value.trim().parse::<usize>().expect("content length"))
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            request_barrier.wait().await;
            response_barrier.wait().await;
            let body = if status == 200 {
                serde_json::json!({"id":"test", "object":"chat.completion", "created":1,
                    "usage":{"prompt_tokens":500,"completion_tokens":50,"total_tokens":550},
                    "model":"o3", "choices":[{"index":0,"message":{"role":"assistant","content":DAG},
                    "finish_reason":"stop"}]}).to_string()
            } else {
                "{\"error\":{\"message\":\"selected builder rejected\"}}".into()
            };
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes()).await;
        });
        Self {
            url,
            arrived,
            release,
            task,
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
