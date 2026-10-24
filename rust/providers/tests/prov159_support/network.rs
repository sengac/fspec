use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
use rig::http_client::{Error, HttpClientExt, LazyBody};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Notify, Semaphore};

pub struct RefreshServer {
    pub uri: String,
    pub started: Arc<Notify>,
    pub release: Arc<Semaphore>,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl RefreshServer {
    pub async fn start() -> Result<Self, std::io::Error> {
        Self::with_body(
            serde_json::json!({
                "id_token": "id-A", "access_token": "access-A-rotated",
                "refresh_token": "refresh-A-rotated", "expires_in": 3600
            })
            .to_string(),
        )
        .await
    }

    pub async fn with_body(body: String) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let uri = format!("http://{}", listener.local_addr()?);
        let started = Arc::new(Notify::new());
        let release = Arc::new(Semaphore::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let task = tokio::spawn({
            let started = started.clone();
            let release = release.clone();
            let requests = requests.clone();
            async move {
                while let Ok((mut stream, _)) = listener.accept().await {
                    let mut request = Vec::new();
                    loop {
                        let mut buffer = [0; 4096];
                        let Ok(count) = stream.read(&mut buffer).await else {
                            break;
                        };
                        if count == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..count]);
                        let Some(header_end) =
                            request.windows(4).position(|part| part == b"\r\n\r\n")
                        else {
                            continue;
                        };
                        let headers = String::from_utf8_lossy(&request[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if request.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                    if let Ok(mut captured) = requests.lock() {
                        captured.push(String::from_utf8_lossy(&request).into_owned());
                    }
                    started.notify_one();
                    let Ok(permit) = release.acquire().await else {
                        break;
                    };
                    permit.forget();
                    let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                    let _ = stream.write_all(response.as_bytes()).await;
                }
            }
        });
        Ok(Self {
            uri,
            started,
            release,
            requests,
            task,
        })
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .expect("request capture lock is available")
            .clone()
    }
}

impl Drop for RefreshServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub fn client(issuer: &str, expires: u64) -> RefreshingCodexClient {
    RefreshingCodexClient::new_oauth(
        "access-A".to_owned(),
        "refresh-A".to_owned(),
        "A".to_owned(),
        Some(expires),
        issuer.to_owned(),
    )
}

pub async fn send(client: &RefreshingCodexClient, url: &str) -> Result<(), Error> {
    let request = http::Request::builder()
        .method("POST")
        .uri(url)
        .body(Vec::<u8>::new())
        .map_err(Error::Protocol)?;
    let _: http::Response<LazyBody<Vec<u8>>> = client.send(request).await?;
    Ok(())
}

pub async fn bounded<F: std::future::Future>(
    future: F,
) -> Result<F::Output, tokio::time::error::Elapsed> {
    tokio::time::timeout(std::time::Duration::from_secs(5), future).await
}
