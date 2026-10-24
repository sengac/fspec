use bytes::Bytes;
use codelet_providers::custom::stream::{open_stream, RhaiStreamProcessor, StreamChunk};
use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use codelet_providers::ProviderError;
use codelet_test_helpers::setup_temp_directory;
use futures::{Stream, StreamExt};
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::streaming::StreamingCompletionResponse;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type Item = Result<StreamChunk, ProviderError>;
pub type Bridge = Pin<Box<dyn Stream<Item = Item> + Send>>;
pub type RigStream = StreamingCompletionResponse<codelet_providers::custom::RhaiCustomCompletion>;

pub const SCRIPT: &str = r#"
fn build_url(config) { config.base_url }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_request(request) { #{ messages: request.messages } }
fn build_stream_request(request) { #{ stream: true } }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn map_error(status, body) {
    if status == 401 { #{ type: "auth", message: "unauthorized" } }
    else { #{ type: "api", message: body } }
}
fn parse_stream_chunk(config, data) {
    if data == "text" { return #{ kind: "text_delta", text: "partial" }; }
    if data == "reason" { return #{ kind: "reasoning_delta", text: "thinking" }; }
    if data == "usage" { return #{ kind: "usage", input_tokens: 12, output_tokens: 3 }; }
    if data == "tool" {
        return #{ kind: "tool_call_delta", index: 0, id: "call_1",
                  name: "read_file", arguments: "{\"path\":\"a.txt\"}" };
    }
    if data == "boom" || data == "[DONE]" { throw "unexpected stream payload"; }
    if data.starts_with("stop:") { return #{ kind: "stop", reason: data.sub_string(5) }; }
    #{ kind: "ignore" }
}
"#;

pub fn processor() -> TestResult<RhaiStreamProcessor> {
    let loader = ScriptLoader::with_default_engine();
    let ast = loader.engine_arc().compile(SCRIPT)?;
    Ok(RhaiStreamProcessor::new(
        loader.engine_arc(),
        Arc::new(ast),
        "bug227-custom".into(),
        rhai::Dynamic::from_map(rhai::Map::new()),
    ))
}

pub fn frame(data: &str) -> Bytes {
    Bytes::from(format!("data: {data}\n\n"))
}

pub fn frames(events: &[&str]) -> Vec<u8> {
    events
        .iter()
        .flat_map(|event| frame(event).to_vec())
        .collect()
}

pub fn bridge(
    body: impl Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
) -> TestResult<Bridge> {
    Ok(open_stream(processor()?, 200, body, |_, body| async move {
        ProviderError::api("bug227-custom", body)
    }))
}

pub fn body_error() -> Result<Bytes, std::io::Error> {
    Err(std::io::Error::other("deterministic body failure"))
}

pub fn assert_error(items: &[Item], message: &str) {
    assert_eq!(
        items.iter().filter(|item| item.is_err()).count(),
        1,
        "{items:?}"
    );
    assert!(
        matches!(items.last(), Some(Err(ProviderError::Api { provider, message: detail }))
        if provider == "bug227-custom" && detail.contains(message)),
        "{items:?}"
    );
}

pub async fn collect_bridge(events: &[&str]) -> TestResult<Vec<Item>> {
    Ok(
        bridge(futures::stream::iter([Ok(Bytes::from(frames(events)))]))?
            .collect()
            .await,
    )
}

pub struct Server {
    pub url: String,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Server {
    pub async fn start(
        body: Vec<u8>,
        status: u16,
        interrupted: bool,
        pending: bool,
    ) -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).await?;
                if count == 0 {
                    return Err(std::io::Error::other("request closed"));
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let length = body.len() + usize::from(interrupted || pending) * 4096;
            let headers = format!("HTTP/1.1 {status} response\r\nContent-Type: text/event-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n");
            socket.write_all(headers.as_bytes()).await?;
            socket.write_all(&body).await?;
            if pending {
                loop {
                    if socket.read(&mut buffer).await? == 0 {
                        break;
                    }
                }
            }
            socket.shutdown().await
        });
        Ok(Self { url, task })
    }

    pub async fn wait(&mut self) -> TestResult {
        tokio::time::timeout(Duration::from_secs(5), &mut self.task).await???;
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn rig_stream(url: &str) -> TestResult<RigStream> {
    let directory = setup_temp_directory()?;
    let script = directory.path().join("provider.rhai");
    std::fs::write(&script, SCRIPT)?;
    let config: ProviderConfig = serde_json::from_value(serde_json::json!({
        "name": "bug227-custom", "display_name": "BUG-227 custom", "base_url": url,
        "script": script, "models": { "smart": { "id": "offline" } }
    }))?;
    let provider = RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "smart".into(),
    )?;
    let model = RhaiCustomProviderModel::new(Arc::new(provider));
    Ok(model
        .stream(CompletionRequestBuilder::new(model.clone(), "hello").build())
        .await?)
}
