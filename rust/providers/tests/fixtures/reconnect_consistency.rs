use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient,
    websocket::CodexWebSocketHandle, CodexResponsesModel,
};
use futures::{SinkExt, StreamExt};
use rig::providers::openai;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn model(
    url: &str,
    history: CodexHistory,
    websocket: bool,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let mut transport = RefreshingCodexClient::new_api_key();
    if websocket {
        transport = transport.with_websocket(CodexWebSocketHandle::default());
    }
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(transport)
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

pub struct SocketServer {
    pub url: String,
    pub requests: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl SocketServer {
    pub async fn start(
        bodies: Vec<String>,
        gate: Option<Arc<Notify>>,
    ) -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            let result = async {
                let mut socket = None;
                for (attempt, body) in bodies.into_iter().enumerate() {
                    if socket.is_none() {
                        let (stream, _) = listener.accept().await?;
                        socket = Some(accept_async(stream).await?);
                    }
                    let connection = socket.as_mut().ok_or("missing socket")?;
                    let request = connection.next().await.ok_or("missing request")??;
                    recorded
                        .lock()
                        .map_err(|_| "capture unavailable")?
                        .push(serde_json::from_str(request.to_text()?)?);
                    let events: Vec<_> = body
                        .split("\n\n")
                        .filter_map(|event| event.strip_prefix("data: "))
                        .collect();
                    for (index, event) in events.iter().enumerate() {
                        if attempt == 0 && index + 1 == events.len() {
                            if let Some(gate) = &gate {
                                gate.notified().await;
                            }
                        }
                        connection
                            .send(Message::Text((*event).to_string().into()))
                            .await?;
                    }
                    if !body.contains("response.completed") && !body.contains("response.incomplete")
                    {
                        connection.close(None).await?;
                        socket = None;
                    }
                }
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            }
            .await;
            assert!(result.is_ok(), "scripted socket failed: {result:?}");
        });
        Ok(Self {
            url,
            requests,
            task,
        })
    }
}
