use super::environment::TestResult;
use codelet_agent_loop::agent_loop::agent_loop;
use codelet_cli::session::Session;
use codelet_providers::{
    copilot::{get_copilot_auth_path, write_copilot_auth, CopilotAuthJson},
    ProviderManager,
};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub const OPAQUE: &str = "synthetic-encrypted-owned-opaque";

pub async fn auth(root: &Path, server: &MockServer, account: &str) -> TestResult {
    assert!(
        get_copilot_auth_path().starts_with(root),
        "credentials escaped temporary root"
    );
    assert!(server.uri().starts_with("http://127.0.0.1:"));
    write_copilot_auth(&CopilotAuthJson {
        github_oauth_token: account.into(),
        copilot_token: Some("synthetic-short-token".into()),
        copilot_token_expires_at: Some(u64::MAX),
        endpoints_api: Some(server.uri()),
        enterprise_url: None,
    })
    .await?;
    Ok(())
}

pub async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(|request:&wiremock::Request| {
        let model=serde_json::from_slice::<Value>(&request.body).ok()
            .and_then(|body|body["model"].as_str().map(str::to_owned)).unwrap_or_else(||"gpt-5".into());
        let item=json!({"type":"function_call","id":"owned-item","call_id":"owned-call","name":"Done",
            "arguments":json!({"summary":"live Copilot complete","work_unit_id":"NONE"}).to_string(),"status":"completed"});
        let response=json!({"id":"owned-response","object":"response","created_at":1,"model":model,"status":"completed",
            "output":[item],"reasoning_opaque":OPAQUE,"usage":{"input_tokens":12,"output_tokens":3,"total_tokens":15}});
        let body=format!("data: {}\n\ndata: {}\n\n",
            json!({"type":"response.output_item.done","sequence_number":0,"output_index":0,"item":item}),
            json!({"type":"response.completed","sequence_number":1,"response":response}));
        ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(body)
    }).mount(&server).await;
    server
}

pub struct Live {
    pub background: Arc<BackgroundSession>,
    chunks: broadcast::Receiver<(codelet_rpc_types::SessionId, StreamChunk)>,
    task: tokio::task::JoinHandle<()>,
    _injections: mpsc::Sender<codelet_tools::McpInjection>,
    _admission: codelet_tools::tool_admission::AuxiliaryScope,
}

impl Drop for Live {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Live {
    pub fn new(root: &Path, fork: Option<&Session>) -> TestResult<Self> {
        let mut manager = ProviderManager::with_provider("github-copilot")?;
        manager.set_model_direct("github-copilot", "gpt-5", Some(200000), Some(4096), None)?;
        let mut inner = Session::from_provider_manager(manager);
        if let Some(original) = fork {
            inner.provider_session = original.provider_session.clone();
            inner.messages = original.messages.clone();
        }
        let manifest = codelet_core::persistence::create_session_with_provider(
            "live opaque",
            root,
            "github-copilot",
        )?;
        let (input_tx, input_rx) = mpsc::channel(4);
        let (chunks_tx, chunks) = broadcast::channel(256);
        let background = Arc::new(BackgroundSession::new(
            manifest.id,
            "live opaque".into(),
            root.display().to_string(),
            None,
            None,
            inner,
            input_tx,
            None,
            None,
            None,
            chunks_tx,
            broadcast::channel(32).0,
            broadcast::channel(4).0,
        ));
        background.set_continue_state(true, 1);
        let admission =
            codelet_tools::tool_admission::register_auxiliary(background.id, root.to_owned())?;
        let (injections, injection_rx) = mpsc::channel(4);
        let task = tokio::spawn(agent_loop(background.clone(), input_rx, injection_rx));
        Ok(Self {
            background,
            chunks,
            task,
            _injections: injections,
            _admission: admission,
        })
    }

    pub async fn turn(&mut self, text: &str) -> TestResult {
        self.background.send_input(text.into(), None)?;
        let events = tokio::time::timeout(Duration::from_secs(10), async {
            let mut events = Vec::new();
            loop {
                let (_, chunk) = self.chunks.recv().await?;
                let done = matches!(chunk, StreamChunk::Done);
                events.push(chunk);
                if done {
                    return Ok::<_, broadcast::error::RecvError>(events);
                }
            }
        })
        .await??;
        assert!(
            !events
                .iter()
                .any(|chunk| matches!(chunk, StreamChunk::Error { .. })),
            "{events:?}"
        );
        let observed = serde_json::to_string(&events)?;
        assert!(
            observed.contains("Completion recorded"),
            "native Done was not accepted"
        );
        assert!(
            !observed.contains(OPAQUE),
            "owned metadata reached diagnostics"
        );
        assert!(!observed.contains("invalid work_unit_id"));
        assert_eq!(self.background.inner.lock().await.continue_nudges_used, 0);
        Ok(())
    }
}

pub async fn request(server: &MockServer, index: usize) -> TestResult<Value> {
    let requests = server.received_requests().await.ok_or("missing capture")?;
    let request = requests.get(index).ok_or("missing request")?;
    assert_eq!(request.url.path(), "/responses");
    Ok(request.body_json()?)
}
