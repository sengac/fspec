use codelet_providers::codex::{
    history::CodexHistory, refreshing_client::RefreshingCodexClient, CodexResponsesModel,
};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequest};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[path = "prov148.rs"]
pub mod baseline;

pub use baseline::TestResult;

#[derive(Clone, Copy)]
pub enum Reply {
    AwaitSetup,
    AwaitBody,
    Complete,
}

#[derive(Debug)]
pub struct CapturedRequest {
    pub headers: String,
    pub body: Value,
}

pub struct Endpoint(SocketAddr);

impl Endpoint {
    pub async fn vacant() -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        Ok(Self(listener.local_addr()?))
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.0)
    }

    pub async fn serve(&self, reply: Reply) -> Result<Server, std::io::Error> {
        let listener = TcpListener::bind(self.0).await?;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let peer_closed = Arc::new(AtomicBool::new(false));
        let captured = requests.clone();
        let closed = peer_closed.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            while let Ok((socket, _)) = listener.accept().await {
                let captured = captured.clone();
                let closed = closed.clone();
                connections.spawn(async move {
                    let _ = respond(socket, captured, closed, reply).await;
                });
            }
        });
        Ok(Server {
            requests,
            peer_closed,
            task,
        })
    }
}

pub struct Server {
    pub requests: Arc<Mutex<Vec<CapturedRequest>>>,
    peer_closed: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub async fn wait_for_close(&self) -> TestResult {
        tokio::time::timeout(Duration::from_secs(1), async {
            while !self.peer_closed.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await?;
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn respond(
    mut socket: TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    peer_closed: Arc<AtomicBool>,
    reply: Reply,
) -> TestResult {
    let mut data = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = socket.read(&mut buffer).await?;
        if count == 0 {
            return Err("client closed before sending the request".into());
        }
        data.extend_from_slice(&buffer[..count]);
        if let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&data[..end]).into_owned();
            let length = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .ok_or("request content length missing")?;
            if data.len() >= end + 4 + length {
                requests
                    .lock()
                    .map_err(|_| "request capture unavailable")?
                    .push(CapturedRequest {
                        headers,
                        body: serde_json::from_slice(&data[end + 4..end + 4 + length])?,
                    });
                break;
            }
        }
    }
    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
    match reply {
        Reply::AwaitSetup => {}
        Reply::AwaitBody => socket.write_all(headers.as_bytes()).await?,
        Reply::Complete => {
            let body = [
                baseline::text(),
                baseline::terminal("completed", Some(true), None),
            ]
            .concat();
            socket
                .write_all(format!("{headers}{body}").as_bytes())
                .await?;
            return Ok(());
        }
    }
    loop {
        match socket.read(&mut buffer).await {
            Ok(0) | Err(_) => {
                peer_closed.store(true, Ordering::SeqCst);
                return Ok(());
            }
            Ok(_) => {}
        }
    }
}

pub fn model(
    url: &str,
    history: CodexHistory,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        }))
}

pub async fn collect_request(
    model: CodexResponsesModel,
    request: CompletionRequest,
) -> Result<baseline::Observed, Box<dyn std::error::Error>> {
    let mut stream = model.stream(request).await?;
    let mut observed = baseline::Observed::default();
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Text(text) => observed.text.push_str(&text.text),
            StreamedAssistantContent::Final(response) => {
                observed.finals.push(response.usage.total_tokens)
            }
            StreamedAssistantContent::ToolCall(_) => observed.tools += 1,
            _ => {}
        }
    }
    Ok(observed)
}
