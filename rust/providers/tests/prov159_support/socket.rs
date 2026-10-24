use codelet_providers::codex::{
    refreshing_client::RefreshingCodexClient, routing::CodexRoutingHandle,
    websocket::CodexWebSocketHandle,
};
use futures::{SinkExt, StreamExt};
use rig::http_client::HttpClientExt;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::{Notify, Semaphore};
use tokio_tungstenite::{accept_hdr_async, tungstenite::Message};

pub struct SocketProbe {
    client: RefreshingCodexClient,
    handle: CodexWebSocketHandle,
    routing: CodexRoutingHandle,
    url: String,
    pub started: Arc<Notify>,
    pub release: Arc<Semaphore>,
    pub headers: Arc<Mutex<Vec<http::HeaderMap>>>,
    pub messages: Arc<Mutex<Vec<serde_json::Value>>>,
    task: tokio::task::JoinHandle<()>,
}

impl SocketProbe {
    pub async fn start() -> Result<Self, std::io::Error> {
        Self::with_handshake(false).await
    }

    pub async fn with_handshake(blocked: bool) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/prov159-socket", listener.local_addr()?);
        let started = Arc::new(Notify::new());
        let release = Arc::new(Semaphore::new(if blocked { 0 } else { 100 }));
        let headers = Arc::new(Mutex::new(Vec::new()));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let task = tokio::spawn({
            let started = started.clone();
            let release = release.clone();
            let headers = headers.clone();
            let messages = messages.clone();
            async move {
                let mut connections = tokio::task::JoinSet::new();
                while let Ok((stream, _)) = listener.accept().await {
                    let started = started.clone();
                    let release = release.clone();
                    let headers = headers.clone();
                    let messages = messages.clone();
                    connections.spawn(async move {
                        started.notify_one();
                        let Ok(permit) = release.acquire().await else { return };
                        permit.forget();
                        let socket = accept_hdr_async(stream, |request: &http::Request<()>, mut response: http::Response<()>| {
                            headers.lock().expect("header capture lock is available").push(request.headers().clone());
                            response.headers_mut().insert("x-codex-turn-state", http::HeaderValue::from_static("account-A-route"));
                            Ok(response)
                        }).await;
                        let Ok(mut socket) = socket else { return };
                        while let Some(Ok(Message::Text(text))) = socket.next().await {
                            let Ok(body) = serde_json::from_str(&text) else { return };
                            messages.lock().expect("message capture lock is available").push(body);
                            let response = serde_json::json!({"type": "response.completed", "response": {
                                "id": "account-A-continuation", "status": "completed", "output": []
                            }});
                            if socket.send(Message::Text(response.to_string().into())).await.is_err() { break; }
                        }
                    });
                }
            }
        });
        let handle = CodexWebSocketHandle::default();
        let client = crate::network::client(&url, 3600).with_websocket(handle.clone());
        Ok(Self {
            client,
            handle,
            routing: CodexRoutingHandle::default(),
            url,
            started,
            release,
            headers,
            messages,
            task,
        })
    }

    pub async fn send(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_with(&self.client, true).await
    }

    pub async fn send_reauthenticated(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_with(
            &crate::network::client(&self.url, 3600).with_websocket(self.handle.clone()),
            true,
        )
        .await
    }

    pub async fn send_unrouted(
        &self,
        reauthenticated: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = if reauthenticated {
            crate::network::client(&self.url, 3600).with_websocket(self.handle.clone())
        } else {
            self.client.clone()
        };
        self.send_with(&client, false).await
    }

    async fn send_with(
        &self,
        client: &RefreshingCodexClient,
        routed: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = http::Request::builder().method("POST").uri(&self.url);
        if routed {
            request = request.extension(self.routing.clone());
        }
        let request = request.body(serde_json::to_vec(
            &serde_json::json!({"model": "test", "input": []}),
        )?)?;
        let mut response = client.send_streaming(request).await?;
        while let Some(bytes) = response.body_mut().next().await {
            bytes?;
        }
        Ok(())
    }

    pub fn requests(&self) -> usize {
        self.messages
            .lock()
            .expect("message capture lock is available")
            .len()
    }
}

impl Drop for SocketProbe {
    fn drop(&mut self) {
        self.task.abort();
    }
}
