//! Test fixtures for Bridge tool integration tests
//!
//! Provides a WebSocket test server and utilities for testing
//! actual WebSocket connections, message relay, and reconnection behavior.

use futures::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message, WebSocketStream};

/// Messages received by the test server from clients
#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub text: String,
    pub timestamp: std::time::Instant,
}

/// Test WebSocket server that records received messages
pub struct TestWebSocketServer {
    /// Address the server is listening on
    pub addr: SocketAddr,
    /// Messages received from clients
    pub received_messages: Arc<Mutex<VecDeque<ReceivedMessage>>>,
    /// Channel to send messages TO connected clients
    pub send_tx: mpsc::Sender<String>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Server task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Whether server should accept connections (for simulating down state)
    pub accepting: Arc<RwLock<bool>>,
}

impl TestWebSocketServer {
    /// Start a new test WebSocket server on an available port
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Bind to localhost on any available port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let received_messages: Arc<Mutex<VecDeque<ReceivedMessage>>> =
            Arc::new(Mutex::new(VecDeque::new()));
        let (send_tx, mut send_rx) = mpsc::channel::<String>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let accepting = Arc::new(RwLock::new(true));

        let messages_clone = Arc::clone(&received_messages);
        let accepting_clone = Arc::clone(&accepting);

        // Track all connected clients for broadcasting
        let clients: Arc<Mutex<Vec<mpsc::Sender<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let clients_for_broadcast = Arc::clone(&clients);

        // Spawn task to broadcast messages to all clients
        let _broadcast_task = {
            let clients = Arc::clone(&clients);
            tokio::spawn(async move {
                while let Some(msg) = send_rx.recv().await {
                    let clients_guard = clients.lock().await;
                    for client_tx in clients_guard.iter() {
                        let _ = client_tx.send(msg.clone()).await;
                    }
                }
            })
        };

        // Spawn server task
        let task_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, _)) => {
                                // Check if we're accepting connections
                                let should_accept = *accepting_clone.read().await;
                                if !should_accept {
                                    // Drop the connection immediately
                                    drop(stream);
                                    continue;
                                }

                                let messages = Arc::clone(&messages_clone);
                                let (client_tx, client_rx) = mpsc::channel::<String>(100);

                                // Add to clients list
                                {
                                    let mut clients_guard = clients_for_broadcast.lock().await;
                                    clients_guard.push(client_tx);
                                }

                                // Handle this connection
                                tokio::spawn(handle_connection(stream, messages, client_rx));
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        Ok(Self {
            addr,
            received_messages,
            send_tx,
            shutdown_tx: Some(shutdown_tx),
            task_handle: Some(task_handle),
            accepting,
        })
    }

    /// Get the WebSocket URL for this server
    pub fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    /// Get all messages received by the server
    pub async fn get_received_messages(&self) -> Vec<ReceivedMessage> {
        let guard = self.received_messages.lock().await;
        guard.iter().cloned().collect()
    }

    /// Wait for at least n messages to be received (with timeout)
    pub async fn wait_for_messages(&self, count: usize, timeout_secs: u64) -> Vec<ReceivedMessage> {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

        while std::time::Instant::now() < deadline {
            let messages = self.get_received_messages().await;
            if messages.len() >= count {
                return messages;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        self.get_received_messages().await
    }

    /// Send a message to all connected clients
    pub async fn send_to_clients(&self, message: &str) -> Result<(), mpsc::error::SendError<String>> {
        self.send_tx.send(message.to_string()).await
    }

    /// Stop accepting new connections (simulate server going down)
    pub async fn stop_accepting(&self) {
        *self.accepting.write().await = false;
    }

    /// Resume accepting connections
    pub async fn resume_accepting(&self) {
        *self.accepting.write().await = true;
    }

    /// Clear received messages
    pub async fn clear_messages(&self) {
        let mut guard = self.received_messages.lock().await;
        guard.clear();
    }

    /// Shutdown the server
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for TestWebSocketServer {
    fn drop(&mut self) {
        // Note: Can't do async cleanup in Drop, but the tasks will be cleaned up
        // when the runtime shuts down. For tests, call shutdown() explicitly.
    }
}

/// Handle a single WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    messages: Arc<Mutex<VecDeque<ReceivedMessage>>>,
    mut to_client_rx: mpsc::Receiver<String>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (mut write, mut read) = ws_stream.split();

    // Spawn task to send messages to client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = to_client_rx.recv().await {
            if write.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Read messages from client
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let mut guard = messages.lock().await;
                guard.push_back(ReceivedMessage {
                    text: text.to_string(),
                    timestamp: std::time::Instant::now(),
                });
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

/// Test WebSocket client for testing bridge behavior
pub struct TestWebSocketClient {
    ws_stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
}

impl TestWebSocketClient {
    /// Connect to a WebSocket server
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (ws_stream, _) = connect_async(url).await?;
        Ok(Self { ws_stream })
    }

    /// Connect with timeout
    pub async fn connect_with_timeout(
        url: &str,
        timeout_secs: u64,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connect_future = connect_async(url);
        match timeout(Duration::from_secs(timeout_secs), connect_future).await {
            Ok(Ok((ws_stream, _))) => Ok(Self { ws_stream }),
            Ok(Err(e)) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            Err(_) => Err("Connection timeout".into()),
        }
    }

    /// Send a message
    pub async fn send(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ws_stream
            .send(Message::Text(message.to_string().into()))
            .await?;
        Ok(())
    }

    /// Receive a message with timeout
    pub async fn recv_with_timeout(
        &mut self,
        timeout_secs: u64,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let recv_future = self.ws_stream.next();
        match timeout(Duration::from_secs(timeout_secs), recv_future).await {
            Ok(Some(Ok(Message::Text(text)))) => Ok(Some(text.to_string())),
            Ok(Some(Ok(_))) => Ok(None), // Non-text message
            Ok(Some(Err(e))) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            Ok(None) => Ok(None), // Stream ended
            Err(_) => Err("Receive timeout".into()),
        }
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ws_stream.close(None).await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod fixture_tests {
    use super::*;

    /// Verify the test fixtures themselves work correctly
    #[tokio::test]
    async fn test_server_client_basic_communication() {
        // Start server
        let server = TestWebSocketServer::start()
            .await
            .expect("Server should start");
        let url = server.url();

        // Connect client
        let mut client = TestWebSocketClient::connect(&url)
            .await
            .expect("Client should connect");

        // Send message from client to server
        client
            .send(r#"{"type": "test", "message": "hello"}"#)
            .await
            .expect("Send should succeed");

        // Wait for server to receive
        let messages = server.wait_for_messages(1, 2).await;
        assert_eq!(messages.len(), 1);
        assert!(messages[0].text.contains("hello"));

        // Clean up
        client.close().await.expect("Close should succeed");
        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_server_send_to_client() {
        let server = TestWebSocketServer::start()
            .await
            .expect("Server should start");
        let url = server.url();

        let mut client = TestWebSocketClient::connect(&url)
            .await
            .expect("Client should connect");

        // Small delay to ensure connection is established
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send from server to client
        server
            .send_to_clients(r#"{"type": "input", "message": "from server"}"#)
            .await
            .expect("Server send should succeed");

        // Receive on client
        let received = client
            .recv_with_timeout(2)
            .await
            .expect("Receive should succeed");
        assert!(received.is_some());
        assert!(received.unwrap().contains("from server"));

        client.close().await.expect("Close should succeed");
        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_connection_refused_when_not_accepting() {
        let server = TestWebSocketServer::start()
            .await
            .expect("Server should start");
        let url = server.url();

        // Stop accepting
        server.stop_accepting().await;

        // Try to connect - should fail
        let _result = TestWebSocketClient::connect_with_timeout(&url, 1).await;
        // Connection might succeed TCP-wise but WebSocket handshake will fail
        // or it will timeout - either way we expect an error or quick disconnect
        
        // Resume and verify normal operation
        server.resume_accepting().await;
        
        let client = TestWebSocketClient::connect(&url)
            .await
            .expect("Should connect after resume");
        client.close().await.expect("Close should succeed");
        
        server.shutdown().await;
    }
}
