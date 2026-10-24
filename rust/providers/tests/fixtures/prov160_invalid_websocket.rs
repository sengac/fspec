use futures::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[derive(Clone, Copy, Debug)]
pub enum Fault {
    Protocol,
    Utf8,
    Capacity,
    Binary,
    DisconnectThenComplete,
}

impl Fault {
    fn frame(self) -> Vec<u8> {
        match self {
            Self::Protocol => vec![0x83, 0],
            Self::Utf8 => vec![0x81, 1, 0xff],
            Self::Capacity => {
                let mut header = vec![0x82, 0x7f];
                header.extend_from_slice(&(64_u64 * 1024 * 1024 + 1).to_be_bytes());
                header
            }
            Self::Binary => vec![0x82, 2, b'{', b'}'],
            Self::DisconnectThenComplete => Vec::new(),
        }
    }
}

pub struct Server {
    pub url: String,
    pub requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub async fn start(fault: Fault) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests = Arc::new(AtomicUsize::new(0));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            while let Ok((socket, _)) = listener.accept().await {
                let recorded = recorded.clone();
                connections.spawn(async move {
                    let Ok(mut socket) = tokio_tungstenite::accept_async(socket).await else {
                        return;
                    };
                    if !matches!(
                        socket.next().await,
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Text(_)))
                    ) {
                        return;
                    }
                    let attempt = recorded.fetch_add(1, Ordering::SeqCst);
                    if matches!(fault, Fault::DisconnectThenComplete) {
                        if attempt > 0 {
                            let terminal = serde_json::json!({
                                "type": "response.completed",
                                "response": {
                                    "id": "resp_recovered",
                                    "status": "completed",
                                    "output": [],
                                    "end_turn": true,
                                    "usage": {"input_tokens": 12, "output_tokens": 8, "total_tokens": 20}
                                }
                            });
                            let _ = socket.send(tokio_tungstenite::tungstenite::Message::Text(
                                terminal.to_string().into(),
                            )).await;
                        }
                        return;
                    }
                    if socket.get_mut().write_all(&fault.frame()).await.is_ok() {
                        let _ = socket.next().await;
                    }
                });
            }
        });
        Ok(Self {
            url,
            requests,
            task,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
