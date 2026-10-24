use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use futures::StreamExt;
use rig::completion::{CompletionError, CompletionModel, CompletionRequestBuilder};
use rig::providers::openai;
use rig::streaming::StreamedAssistantContent;
use std::time::Duration;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub async fn client_error(
    server: &wiremock::MockServer,
    credentials: bool,
) -> Result<rig::http_client::Error, Box<dyn std::error::Error>> {
    use rig::http_client::HttpClientExt;
    let mut request = http::Request::builder()
        .method("POST")
        .uri(format!("{}/responses", server.uri()));
    if credentials {
        request = request
            .header("authorization", "Bearer secret-credential")
            .header("chatgpt-account-id", "private-account")
            .header("cookie", "session=cookie-secret; quoted=\"quoted-secret\"");
    }
    let result = RefreshingCodexClient::new_api_key()
        .send_streaming(request.body(b"{}".to_vec())?)
        .await;
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("rejection unexpectedly succeeded".into()),
    }
}

pub fn diagnostic(
    error: &rig::http_client::Error,
) -> Result<&codelet_providers::codex::diagnostics::CodexRejection, Box<dyn std::error::Error>> {
    match error {
        rig::http_client::Error::Instance(error) => error
            .downcast_ref::<codelet_providers::codex::diagnostics::CodexRejection>()
            .ok_or_else(|| "missing Codex rejection type".into()),
        _ => Err("missing HTTP instance error".into()),
    }
}

pub fn model_diagnostic(
    error: &CompletionError,
) -> Result<&codelet_providers::codex::diagnostics::CodexRejection, Box<dyn std::error::Error>> {
    match error {
        CompletionError::HttpError(error) => diagnostic(error),
        _ => Err("missing typed completion HTTP error".into()),
    }
}

pub async fn model_error(
    url: &str,
    timeout: Duration,
) -> Result<CompletionError, Box<dyn std::error::Error>> {
    let client = openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(url)
        .http_client(RefreshingCodexClient::new_api_key().with_idle_timeout(timeout))
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model");
    let request = CompletionRequestBuilder::new(model.clone(), "Hello")
        .preamble("Test instructions".to_owned())
        .build();
    let mut stream = model.stream(request).await?;
    let mut failure = None;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => {
                assert!(failure.is_none(), "one terminal error is expected");
                failure = Some(error);
            }
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_)) => {
                return Err("rejection produced completion or tool dispatch".into());
            }
            _ => {}
        }
    }
    failure.ok_or_else(|| "missing terminal error".into())
}

pub struct StalledRejection {
    pub url: String,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl StalledRejection {
    pub async fn start() -> Result<Self, std::io::Error> {
        Self::serve("", true).await
    }

    pub async fn broken() -> Result<Self, std::io::Error> {
        Self::serve(r#"{"error":{"code":"context_length_exceeded"}}"#, false).await
    }

    async fn serve(body: &'static str, stall: bool) -> Result<Self, std::io::Error> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut request = [0; 8192];
            if socket.read(&mut request).await? == 0 {
                return Ok(());
            }
            socket.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 1024\r\nx-request-id: broken-request\r\nConnection: close\r\n\r\n").await?;
            socket.write_all(body.as_bytes()).await?;
            if stall {
                tokio::time::sleep(Duration::from_secs(10)).await;
            } else {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            socket.shutdown().await
        });
        Ok(Self { url, task })
    }
}

impl Drop for StalledRejection {
    fn drop(&mut self) {
        self.task.abort();
    }
}
