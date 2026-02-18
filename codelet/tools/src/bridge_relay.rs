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

// Message type constants
const MSG_TYPE_INPUT: &str = "input";
const MSG_TYPE_CONTROL: &str = "control";

// Control action constants
const ACTION_INTERRUPT: &str = "interrupt";
const ACTION_CLEAR: &str = "clear";
const ACTION_PAUSE_RESPONSE: &str = "pause_response";

/// Image data received from bridge endpoint (BRIDGE-007)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
}

/// Input to be injected into the session (BRIDGE-007)
/// Replaces the simple String to support images from Telegram bridge
#[derive(Debug, Clone)]
pub struct InjectedInput {
    /// Text message content
    pub message: String,
    /// Optional images (from Telegram photo messages)
    pub images: Option<Vec<ImageData>>,
}

impl InjectedInput {
    /// Create a new InjectedInput with just a message (backward compatibility)
    pub fn text_only(message: String) -> Self {
        Self {
            message,
            images: None,
        }
    }
    
    /// Create a new InjectedInput with message and images
    pub fn with_images(message: String, images: Vec<ImageData>) -> Self {
        Self {
            message,
            images: if images.is_empty() { None } else { Some(images) },
        }
    }
}

/// Inbound message from WebSocket endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    /// Message content (for input messages)
    #[serde(default)]
    pub message: String,
    /// Optional images array (BRIDGE-007: Telegram photo support)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageData>>,
    /// Optional action for control messages (BRIDGE-008)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Optional response for pause_response control action (BRIDGE-014)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}

/// Callback for injecting input into the session (BRIDGE-007: updated to InjectedInput)
pub type InputInjector = Arc<dyn Fn(InjectedInput) + Send + Sync>;

/// Callback for handling control actions (BRIDGE-008: interrupt, clear; BRIDGE-014: pause_response)
/// Takes the action name and optional response value (for pause_response)
pub type ControlHandler = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;

/// Spawn a WebSocket relay task for a bridge connection
///
/// This function:
/// 1. Connects to the WebSocket URL
/// 2. Updates connection state to Connected
/// 3. Spawns outbound/inbound message handlers
/// 4. Handles reconnection on disconnect
///
/// BRIDGE-008: Now accepts an optional control_handler for interrupt/clear actions
pub async fn spawn_relay_task(
    session_id: Uuid,
    url: String,
    stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
) -> Result<tokio::task::JoinHandle<()>, ToolError> {
    let handle = tokio::spawn(async move {
        relay_loop(session_id, url, stream_rx, input_injector, control_handler).await;
    });
    
    Ok(handle)
}

/// Main relay loop with reconnection logic
async fn relay_loop(
    session_id: Uuid,
    url: String,
    mut stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
) {
    let mut reconnect_delay = Duration::from_secs(INITIAL_RECONNECT_DELAY_SECS);
    
    loop {
        // Try to connect
        match connect_and_relay(
            session_id,
            &url,
            &mut stream_rx,
            input_injector.clone(),
            control_handler.clone(),
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
    control_handler: Option<ControlHandler>,
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
    let inbound_control_handler = control_handler.clone();
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
                        inbound_control_handler.clone(),
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
///
/// BRIDGE-008: Now handles "control" message type with actions "interrupt" and "clear"
pub async fn handle_inbound_message(
    text: &str,
    session_id: Uuid,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
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
        MSG_TYPE_INPUT => {
            // BRIDGE-007: Create InjectedInput with message and optional images
            let injected = match inbound.images {
                Some(images) if !images.is_empty() => {
                    tracing::info!("Injecting input from bridge with {} image(s): {}", 
                        images.len(), inbound.message);
                    InjectedInput::with_images(inbound.message, images)
                }
                _ => {
                    tracing::info!("Injecting text input from bridge: {}", inbound.message);
                    InjectedInput::text_only(inbound.message)
                }
            };
            input_injector(injected);
            Ok(())
        }
        MSG_TYPE_CONTROL => {
            // BRIDGE-008: Handle control messages (interrupt, clear)
            // BRIDGE-014: Also handles pause_response
            let action = inbound.action.as_deref().unwrap_or("");
            
            match action {
                ACTION_INTERRUPT | ACTION_CLEAR => {
                    if let Some(handler) = control_handler {
                        tracing::info!("Handling control action from bridge: {}", action);
                        handler(action, None);
                    } else {
                        tracing::warn!("Received control action '{}' but no control handler is configured", action);
                    }
                    Ok(())
                }
                ACTION_PAUSE_RESPONSE => {
                    // BRIDGE-014: Handle pause response with response value
                    if let Some(handler) = control_handler {
                        let response = inbound.response.as_deref();
                        tracing::info!("Handling pause_response from bridge: {:?}", response);
                        handler(action, response);
                    } else {
                        tracing::warn!("Received pause_response but no control handler is configured");
                    }
                    Ok(())
                }
                _ => {
                    // Unknown action - log warning but don't crash (graceful handling)
                    tracing::warn!("Ignoring unknown control action: {}", action);
                    Ok(())
                }
            }
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
        assert!(msg.images.is_none());
    }
    
    // @step Given the bridge_relay module receives a JSON message with images
    // @step When the InboundMessage is deserialized
    // @step Then the images array should contain 1 element
    #[test]
    fn test_inbound_message_parse_with_images() {
        let json = r#"{"type": "input", "session_id": "test-123", "message": "What is this?", "images": [{"data": "base64...", "media_type": "image/jpeg"}]}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "input");
        assert_eq!(msg.session_id, "test-123");
        assert_eq!(msg.message, "What is this?");
        
        // @step And the first image should have data "base64..."
        // @step And the first image should have media_type "image/jpeg"
        let images = msg.images.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].data, "base64...");
        assert_eq!(images[0].media_type, "image/jpeg");
    }
    
    // @step Given the bridge_relay module receives a JSON message without images
    // @step When the InboundMessage is deserialized
    // @step Then the images field should be None
    // @step And the message field should be "Hello"
    #[test]
    fn test_inbound_message_backward_compatibility() {
        let json = r#"{"type": "input", "session_id": "test-123", "message": "Hello"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.message, "Hello");
        assert!(msg.images.is_none());
    }
    
    #[test]
    fn test_inbound_message_with_multiple_images() {
        let json = r#"{"type": "input", "session_id": "test-123", "message": "Compare these", "images": [{"data": "img1", "media_type": "image/jpeg"}, {"data": "img2", "media_type": "image/png"}]}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        let images = msg.images.unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].media_type, "image/jpeg");
        assert_eq!(images[1].media_type, "image/png");
    }
    
    #[test]
    fn test_injected_input_text_only() {
        let input = InjectedInput::text_only("Hello".to_string());
        assert_eq!(input.message, "Hello");
        assert!(input.images.is_none());
    }
    
    #[test]
    fn test_injected_input_with_images() {
        let images = vec![ImageData {
            data: "base64data".to_string(),
            media_type: "image/jpeg".to_string(),
        }];
        let input = InjectedInput::with_images("Caption".to_string(), images);
        assert_eq!(input.message, "Caption");
        assert!(input.images.is_some());
        assert_eq!(input.images.unwrap().len(), 1);
    }
    
    #[test]
    fn test_injected_input_with_empty_images() {
        let input = InjectedInput::with_images("Text".to_string(), vec![]);
        assert_eq!(input.message, "Text");
        assert!(input.images.is_none()); // Empty vec becomes None
    }
    
    // @step Given the bridge receives an inbound message with images
    // @step When handle_inbound_message processes the message
    // @step Then the InputInjector callback should receive InjectedInput with message and images
    // @step And the images should be propagated to the session
    #[tokio::test]
    async fn test_injected_input_propagates_images() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        let received_images = Arc::new(AtomicBool::new(false));
        let received_images_clone = received_images.clone();
        
        let input_injector: InputInjector = Arc::new(move |input: InjectedInput| {
            if input.images.is_some() {
                received_images_clone.store(true, Ordering::SeqCst);
            }
        });
        
        let session_id = uuid::Uuid::new_v4();
        let json = format!(
            r#"{{"type": "input", "session_id": "{session_id}", "message": "What is this?", "images": [{{"data": "base64...", "media_type": "image/jpeg"}}]}}"#
        );
        
        // This calls the input_injector internally
        let result = super::handle_inbound_message(&json, session_id, input_injector, None).await;
        assert!(result.is_ok());
        assert!(received_images.load(Ordering::SeqCst), "InputInjector should receive images");
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
    
    // =========================================================================
    // BRIDGE-008: Control message tests
    // =========================================================================
    
    /// @step Given the bridge is connected to a session
    /// @step And the agent is processing a request
    /// @step When the bridge receives a message with type "control" and action "interrupt"
    /// @step Then the agent should stop processing
    /// @step And the is_interrupted flag should be set to true
    #[tokio::test]
    async fn test_handle_interrupt_control_message() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // @step Given the bridge is connected to a session
        let session_id = uuid::Uuid::new_v4();
        
        // @step And the agent is processing a request
        let interrupt_called = Arc::new(AtomicBool::new(false));
        let interrupt_called_clone = interrupt_called.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |action: &str, _response: Option<&str>| {
            if action == "interrupt" {
                interrupt_called_clone.store(true, Ordering::SeqCst);
            }
        });
        
        // @step When the bridge receives a message with type "control" and action "interrupt"
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "interrupt"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        // @step Then the agent should stop processing
        assert!(result.is_ok());
        
        // @step And the is_interrupted flag should be set to true
        assert!(interrupt_called.load(Ordering::SeqCst), "Control handler should be called for interrupt");
    }
    
    /// @step Given the bridge is connected to a session
    /// @step And the session has conversation history
    /// @step When the bridge receives a message with type "control" and action "clear"
    /// @step Then the session should be reset
    /// @step And the conversation history should be cleared
    #[tokio::test]
    async fn test_handle_clear_control_message() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // @step Given the bridge is connected to a session
        let session_id = uuid::Uuid::new_v4();
        
        // @step And the session has conversation history
        let clear_called = Arc::new(AtomicBool::new(false));
        let clear_called_clone = clear_called.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |action: &str, _response: Option<&str>| {
            if action == "clear" {
                clear_called_clone.store(true, Ordering::SeqCst);
            }
        });
        
        // @step When the bridge receives a message with type "control" and action "clear"
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "clear"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        // @step Then the session should be reset
        assert!(result.is_ok());
        
        // @step And the conversation history should be cleared
        assert!(clear_called.load(Ordering::SeqCst), "Control handler should be called for clear");
    }
    
    /// @step Given the bridge is connected to a session
    /// @step When the bridge receives a message with type "control" and action "unknown"
    /// @step Then an error should be logged
    /// @step And the bridge should not crash
    /// @step And the session should remain active
    #[tokio::test]
    async fn test_handle_unknown_control_action_gracefully() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // @step Given the bridge is connected to a session
        let session_id = uuid::Uuid::new_v4();
        
        let handler_called = Arc::new(AtomicBool::new(false));
        let handler_called_clone = handler_called.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |_action: &str, _response: Option<&str>| {
            handler_called_clone.store(true, Ordering::SeqCst);
        });
        
        // @step When the bridge receives a message with type "control" and action "unknown"
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "unknown"}}"#
        );
        
        // @step Then an error should be logged (tracing::warn in implementation)
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        // @step And the bridge should not crash
        assert!(result.is_ok(), "Unknown action should be handled gracefully");
        
        // @step And the session should remain active
        // Handler should NOT be called for unknown actions
        assert!(!handler_called.load(Ordering::SeqCst), "Handler should not be called for unknown action");
    }
    
    /// @step Given the bridge is connected to a session
    /// @step When the bridge receives a message with type "input"
    /// @step Then the message should be forwarded to the agent
    /// @step And the agent should process the input
    #[tokio::test]
    async fn test_forward_input_messages_to_agent() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // @step Given the bridge is connected to a session
        let session_id = uuid::Uuid::new_v4();
        
        let input_called = Arc::new(AtomicBool::new(false));
        let input_called_clone = input_called.clone();
        
        let input_injector: InputInjector = Arc::new(move |_| {
            input_called_clone.store(true, Ordering::SeqCst);
        });
        let control_handler: ControlHandler = Arc::new(|_action: &str, _response: Option<&str>| {});
        
        // @step When the bridge receives a message with type "input"
        let json = format!(
            r#"{{"type": "input", "session_id": "{session_id}", "message": "Hello, agent!"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        // @step Then the message should be forwarded to the agent
        assert!(result.is_ok());
        
        // @step And the agent should process the input
        assert!(input_called.load(Ordering::SeqCst), "Input injector should be called for input messages");
    }
    
    /// Test backward compatibility: input messages work without control handler
    #[tokio::test]
    async fn test_input_without_control_handler_backward_compatible() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        let input_called = Arc::new(AtomicBool::new(false));
        let input_called_clone = input_called.clone();
        
        let input_injector: InputInjector = Arc::new(move |_| {
            input_called_clone.store(true, Ordering::SeqCst);
        });
        
        let session_id = uuid::Uuid::new_v4();
        let json = format!(
            r#"{{"type": "input", "session_id": "{session_id}", "message": "Test"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, None).await;
        assert!(result.is_ok());
        assert!(input_called.load(Ordering::SeqCst));
    }
    
    /// Test control message without handler is handled gracefully
    #[tokio::test]
    async fn test_control_message_without_handler() {
        let input_injector: InputInjector = Arc::new(|_| {});
        
        let session_id = uuid::Uuid::new_v4();
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "interrupt"}}"#
        );
        
        // Should not panic - just log a warning
        let result = super::handle_inbound_message(&json, session_id, input_injector, None).await;
        assert!(result.is_ok(), "Should handle gracefully when no control handler is set");
    }
    
    /// Test control message parsing with action field
    #[test]
    fn test_inbound_message_parse_control() {
        let json = r#"{"type": "control", "session_id": "test-id", "message": "", "action": "interrupt"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "control");
        assert_eq!(msg.session_id, "test-id");
        assert_eq!(msg.action, Some("interrupt".to_string()));
    }
    
    // =========================================================================
    // BRIDGE-014: Pause response control message tests
    // =========================================================================
    
    /// @step Given the bridge is connected to a session
    /// @step And the session is paused waiting for access decision
    /// @step When the bridge receives a control message with action "pause_response" and response "allow_once"
    /// @step Then the control handler should be called with action and response
    /// @step And the session should resume with AllowOnce response
    #[tokio::test]
    async fn test_handle_pause_response_allow_once() {
        use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
        
        let session_id = uuid::Uuid::new_v4();
        
        let handler_called = Arc::new(AtomicBool::new(false));
        let handler_called_clone = handler_called.clone();
        let received_response = Arc::new(Mutex::new(String::new()));
        let received_response_clone = received_response.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |action: &str, response: Option<&str>| {
            if action == "pause_response" {
                handler_called_clone.store(true, Ordering::SeqCst);
                if let Some(resp) = response {
                    *received_response_clone.lock().unwrap() = resp.to_string();
                }
            }
        });
        
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "pause_response", "response": "allow_once"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        assert!(result.is_ok());
        assert!(handler_called.load(Ordering::SeqCst), "Control handler should be called for pause_response");
        assert_eq!(*received_response.lock().unwrap(), "allow_once");
    }
    
    /// @step Given the bridge receives pause_response with "allow_session"
    /// @step Then the control handler should receive response "allow_session"
    #[tokio::test]
    async fn test_handle_pause_response_allow_session() {
        use std::sync::Mutex;
        
        let session_id = uuid::Uuid::new_v4();
        let received_response = Arc::new(Mutex::new(String::new()));
        let received_response_clone = received_response.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |_action: &str, response: Option<&str>| {
            if let Some(resp) = response {
                *received_response_clone.lock().unwrap() = resp.to_string();
            }
        });
        
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "pause_response", "response": "allow_session"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        assert!(result.is_ok());
        assert_eq!(*received_response.lock().unwrap(), "allow_session");
    }
    
    /// @step Given the bridge receives pause_response with "deny"
    /// @step Then the control handler should receive response "deny"
    #[tokio::test]
    async fn test_handle_pause_response_deny() {
        use std::sync::Mutex;
        
        let session_id = uuid::Uuid::new_v4();
        let received_response = Arc::new(Mutex::new(String::new()));
        let received_response_clone = received_response.clone();
        
        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |_action: &str, response: Option<&str>| {
            if let Some(resp) = response {
                *received_response_clone.lock().unwrap() = resp.to_string();
            }
        });
        
        let json = format!(
            r#"{{"type": "control", "session_id": "{session_id}", "message": "", "action": "pause_response", "response": "deny"}}"#
        );
        
        let result = super::handle_inbound_message(&json, session_id, input_injector, Some(control_handler)).await;
        
        assert!(result.is_ok());
        assert_eq!(*received_response.lock().unwrap(), "deny");
    }
    
    /// Test parsing inbound message with response field
    #[test]
    fn test_inbound_message_parse_pause_response() {
        let json = r#"{"type": "control", "session_id": "test-id", "message": "", "action": "pause_response", "response": "allow_once"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.msg_type, "control");
        assert_eq!(msg.session_id, "test-id");
        assert_eq!(msg.action, Some("pause_response".to_string()));
        assert_eq!(msg.response, Some("allow_once".to_string()));
    }
}
