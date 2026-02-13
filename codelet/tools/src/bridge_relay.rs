//! Bridge WebSocket Relay Task
//!
//! Handles the actual WebSocket connection and message relay between
//! the session's broadcast channel and the external endpoint.
//!
//! Feature: spec/features/bridge-tool.feature

use crate::bridge::{
    get_or_create_bridge_manager, BridgeConnectionState, OutboundMessage,
};
use crate::ToolError;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Maximum reconnection delay in seconds
const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// Initial reconnection delay in seconds
const INITIAL_RECONNECT_DELAY_SECS: u64 = 1;

/// Inbound message from WebSocket endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    pub message: String,
}

/// Callback for injecting input into the session
pub type InputInjector = Arc<dyn Fn(String) + Send + Sync>;

/// Spawn a WebSocket relay task for a bridge connection
///
/// This function:
/// 1. Connects to the WebSocket URL
/// 2. Updates connection state to Connected
/// 3. Spawns outbound/inbound message handlers
/// 4. Handles reconnection on disconnect
pub async fn spawn_relay_task(
    session_id: Uuid,
    url: String,
    stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
) -> Result<tokio::task::JoinHandle<()>, ToolError> {
    let handle = tokio::spawn(async move {
        relay_loop(session_id, url, stream_rx, input_injector).await;
    });
    
    Ok(handle)
}

/// Main relay loop with reconnection logic
async fn relay_loop(
    session_id: Uuid,
    url: String,
    mut stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
) {
    let mut reconnect_delay = Duration::from_secs(INITIAL_RECONNECT_DELAY_SECS);
    
    loop {
        // Try to connect
        match connect_and_relay(
            session_id,
            &url,
            &mut stream_rx,
            input_injector.clone(),
        ).await {
            Ok(()) => {
                // Clean disconnect, exit the loop
                tracing::info!("Bridge {} cleanly disconnected", url);
                break;
            }
            Err(e) => {
                tracing::warn!("Bridge {} connection error: {}, reconnecting in {:?}", 
                    url, e, reconnect_delay);
                
                // Update state to Reconnecting
                update_connection_state(session_id, &url, BridgeConnectionState::Reconnecting).await;
                
                // Wait before reconnecting
                tokio::time::sleep(reconnect_delay).await;
                
                // Exponential backoff
                reconnect_delay = std::cmp::min(
                    reconnect_delay * 2,
                    Duration::from_secs(MAX_RECONNECT_DELAY_SECS),
                );
            }
        }
        
        // Check if connection was removed (user called disconnect)
        let manager = get_or_create_bridge_manager(session_id).await;
        let mgr = manager.read().await;
        if !mgr.connections.contains_key(&url) {
            tracing::info!("Bridge {} was removed, stopping relay", url);
            break;
        }
    }
}

/// Connect to WebSocket and handle message relay
async fn connect_and_relay(
    session_id: Uuid,
    url: &str,
    stream_rx: &mut broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
) -> Result<(), String> {
    // Connect to WebSocket
    let (ws_stream, _) = connect_async(url)
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;
    
    tracing::info!("Bridge connected to {}", url);
    
    // Update state to Connected
    update_connection_state(session_id, url, BridgeConnectionState::Connected).await;
    
    // Reset reconnect delay on successful connection
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    // Send any buffered messages
    send_buffered_messages(session_id, url, &mut ws_write).await?;
    
    // Send "connected" message to identify this session to the endpoint
    send_connected_message(session_id, &mut ws_write).await?;
    
    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    
    // Spawn inbound message handler
    let inbound_url = url.to_string();
    let inbound_session_id = session_id;
    let inbound_shutdown_tx = shutdown_tx.clone();
    let inbound_handle = tokio::spawn(async move {
        tracing::warn!("Bridge {} inbound handler started, listening for session {}", inbound_url, inbound_session_id);
        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    tracing::warn!("Bridge {} received text message: {}", inbound_url, text.chars().take(100).collect::<String>());
                    if let Err(e) = handle_inbound_message(
                        text.as_ref(),
                        inbound_session_id,
                        input_injector.clone(),
                    ).await {
                        tracing::warn!("Failed to handle inbound message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::warn!("WebSocket {} received close frame", inbound_url);
                    let _ = inbound_shutdown_tx.send(()).await;
                    break;
                }
                Ok(_) => {
                    // Ignore other message types (binary, ping, pong)
                }
                Err(e) => {
                    tracing::warn!("WebSocket {} read error: {}", inbound_url, e);
                    let _ = inbound_shutdown_tx.send(()).await;
                    break;
                }
            }
        }
    });
    
    // Outbound message loop
    let outbound_url = url.to_string();
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                tracing::info!("Bridge {} received shutdown signal", outbound_url);
                break;
            }
            
            // Receive from broadcast channel
            chunk_result = stream_rx.recv() => {
                match chunk_result {
                    Ok(chunk_json) => {
                        // Create outbound message envelope
                        let outbound = OutboundMessage {
                            msg_type: "chunk".to_string(),
                            session_id: session_id.to_string(),
                            data: chunk_json,
                        };
                        
                        let msg_json = match serde_json::to_string(&outbound) {
                            Ok(json) => json,
                            Err(e) => {
                                tracing::warn!("Failed to serialize outbound message: {}", e);
                                continue;
                            }
                        };
                        
                        // Send to WebSocket
                        if let Err(e) = ws_write.send(Message::Text(msg_json.into())).await {
                            tracing::warn!("Failed to send to WebSocket: {}", e);
                            // Buffer the message for retry
                            buffer_message_on_error(session_id, &outbound_url, outbound).await;
                            return Err(format!("Send failed: {e}"));
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Bridge {} lagged {} messages", outbound_url, n);
                        // Continue receiving
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Bridge {} broadcast channel closed", outbound_url);
                        break;
                    }
                }
            }
        }
    }
    
    // Wait for inbound handler to finish
    let _ = inbound_handle.await;
    
    Ok(())
}

/// Handle an inbound message from the WebSocket
async fn handle_inbound_message(
    text: &str,
    session_id: Uuid,
    input_injector: InputInjector,
) -> Result<(), String> {
    // Parse the message
    let inbound: InboundMessage = serde_json::from_str(text)
        .map_err(|e| format!("Failed to parse inbound message: {e}"))?;
    
    // Verify session ID matches (or accept if it targets this session)
    if inbound.session_id != session_id.to_string() {
        tracing::warn!("Ignoring message for different session - expected: {}, got: {}", session_id, inbound.session_id);
        return Ok(());
    }
    
    // Handle based on message type
    match inbound.msg_type.as_str() {
        "input" => {
            tracing::warn!("Injecting input from bridge: {}", inbound.message);
            input_injector(inbound.message);
            Ok(())
        }
        _ => {
            tracing::warn!("Ignoring unknown message type: {}", inbound.msg_type);
            Ok(())
        }
    }
}

/// Send any buffered messages after reconnection
async fn send_buffered_messages(
    session_id: Uuid,
    url: &str,
    ws_write: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
        Message
    >,
) -> Result<(), String> {
    let manager = get_or_create_bridge_manager(session_id).await;
    let buffered = {
        let mut mgr = manager.write().await;
        if let Some(conn) = mgr.get_connection_mut(url) {
            conn.take_buffer()
        } else {
            vec![]
        }
    };
    
    if !buffered.is_empty() {
        tracing::info!("Sending {} buffered messages to {}", buffered.len(), url);
        for msg in buffered {
            let msg_json = serde_json::to_string(&msg)
                .map_err(|e| format!("Failed to serialize buffered message: {e}"))?;
            ws_write.send(Message::Text(msg_json.into())).await
                .map_err(|e| format!("Failed to send buffered message: {e}"))?;
        }
    }
    
    Ok(())
}

/// Send "connected" message to identify this session to the endpoint
async fn send_connected_message(
    session_id: Uuid,
    ws_write: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
        Message
    >,
) -> Result<(), String> {
    let msg = OutboundMessage {
        msg_type: "connected".to_string(),
        session_id: session_id.to_string(),
        data: serde_json::json!({}),
    };
    
    let msg_json = serde_json::to_string(&msg)
        .map_err(|e| format!("Failed to serialize connected message: {e}"))?;
    
    ws_write.send(Message::Text(msg_json.into())).await
        .map_err(|e| format!("Failed to send connected message: {e}"))?;
    
    tracing::info!("Sent connected message with session_id: {}", session_id);
    Ok(())
}

/// Buffer a message when send fails (for retry on reconnect)
async fn buffer_message_on_error(session_id: Uuid, url: &str, msg: OutboundMessage) {
    let manager = get_or_create_bridge_manager(session_id).await;
    let mut mgr = manager.write().await;
    if let Some(conn) = mgr.get_connection_mut(url) {
        if let Err(e) = conn.buffer_message(msg) {
            tracing::error!("Failed to buffer message for {}: {}", url, e);
            // Buffer overflow - connection will be dropped
            conn.state = BridgeConnectionState::Disconnected;
        }
    }
}

/// Update connection state in the manager
async fn update_connection_state(session_id: Uuid, url: &str, state: BridgeConnectionState) {
    let manager = get_or_create_bridge_manager(session_id).await;
    let mut mgr = manager.write().await;
    if let Some(conn) = mgr.get_connection_mut(url) {
        conn.state = state;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inbound_message_parse() {
        let json = r#"{"type": "input", "session_id": "test-id", "message": "hello"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "input");
        assert_eq!(msg.session_id, "test-id");
        assert_eq!(msg.message, "hello");
    }
    
    #[test]
    fn test_outbound_message_serialize() {
        let msg = OutboundMessage {
            msg_type: "chunk".to_string(),
            session_id: "test-id".to_string(),
            data: serde_json::json!({"type": "text", "text": "hello"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"chunk\""));
        assert!(json.contains("\"session_id\":\"test-id\""));
    }
}
