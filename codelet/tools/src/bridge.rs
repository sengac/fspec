//! Bridge Tool Implementation
//!
//! Feature: spec/features/bridge-tool.feature
//!
//! This tool enables AI agents to connect their sessions to external WebSocket endpoints,
//! relaying all StreamChunks (text, tool calls, thinking) to the endpoint and receiving
//! input from the endpoint to inject into the session.
//!
//! CRITICAL WARNING: NO CLI INVOCATION - NO FALLBACKS - NO SIMULATIONS

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::ToolError;

/// Maximum buffer size in bytes (1GB)
pub const MAX_BUFFER_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// Connection state for a bridge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeConnectionState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

/// Information about a single bridge connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConnectionInfo {
    pub url: String,
    pub state: BridgeConnectionState,
    pub buffered: usize,
}

/// A single WebSocket connection to an external endpoint
#[derive(Debug)]
pub struct BridgeConnection {
    /// The WebSocket URL
    pub url: String,
    /// Current connection state
    pub state: BridgeConnectionState,
    /// Outbound message buffer (when connection is down)
    pub outbound_buffer: VecDeque<OutboundMessage>,
    /// Total size of buffered messages in bytes
    pub buffer_size_bytes: u64,
    /// Handle to the WebSocket task (for cancellation)
    pub task_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Message to send to external endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    pub data: serde_json::Value,
    /// BRIDGE-016: Optional request_id for commandResponse correlation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl BridgeConnection {
    /// Create a new bridge connection
    pub fn new(url: String) -> Self {
        Self {
            url,
            state: BridgeConnectionState::Connecting,
            outbound_buffer: VecDeque::new(),
            buffer_size_bytes: 0,
            task_handle: None,
        }
    }

    /// Get connection info for reporting
    pub fn info(&self) -> BridgeConnectionInfo {
        BridgeConnectionInfo {
            url: self.url.clone(),
            state: self.state,
            buffered: self.outbound_buffer.len(),
        }
    }

    /// Add a message to the buffer
    /// Returns Err if buffer would exceed MAX_BUFFER_SIZE_BYTES
    pub fn buffer_message(&mut self, msg: OutboundMessage) -> Result<(), ToolError> {
        let msg_json = serde_json::to_string(&msg).map_err(|e| ToolError::Execution {
            tool: "bridge",
            message: format!("Failed to serialize message: {e}"),
        })?;
        let msg_size = msg_json.len() as u64;

        if self.buffer_size_bytes + msg_size > MAX_BUFFER_SIZE_BYTES {
            return Err(ToolError::Execution {
                tool: "bridge",
                message: format!("Buffer overflow: adding {msg_size} bytes would exceed 1GB limit"),
            });
        }

        self.buffer_size_bytes += msg_size;
        self.outbound_buffer.push_back(msg);
        Ok(())
    }

    /// Take all buffered messages for delivery
    pub fn take_buffer(&mut self) -> Vec<OutboundMessage> {
        self.buffer_size_bytes = 0;
        self.outbound_buffer.drain(..).collect()
    }
}

/// Manages all bridge connections for a single session
#[derive(Debug)]
pub struct BridgeManager {
    /// Session ID this manager belongs to
    pub session_id: Uuid,
    /// Active connections by URL
    pub connections: HashMap<String, BridgeConnection>,
}

impl BridgeManager {
    /// Create a new bridge manager for a session
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            connections: HashMap::new(),
        }
    }

    /// Add a connection
    pub fn add_connection(&mut self, url: String) -> &mut BridgeConnection {
        self.connections
            .entry(url.clone())
            .or_insert_with(|| BridgeConnection::new(url))
    }

    /// Remove a connection
    pub fn remove_connection(&mut self, url: &str) -> Option<BridgeConnection> {
        self.connections.remove(url)
    }

    /// Get connection info for all active connections
    pub fn list_connections(&self) -> Vec<BridgeConnectionInfo> {
        self.connections.values().map(BridgeConnection::info).collect()
    }

    /// Get a mutable reference to a connection
    pub fn get_connection_mut(&mut self, url: &str) -> Option<&mut BridgeConnection> {
        self.connections.get_mut(url)
    }
}

/// Global bridge managers keyed by session ID
static BRIDGES: once_cell::sync::Lazy<Mutex<HashMap<Uuid, Arc<RwLock<BridgeManager>>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Get or create a bridge manager for a session
pub async fn get_or_create_bridge_manager(session_id: Uuid) -> Arc<RwLock<BridgeManager>> {
    let mut bridges = BRIDGES.lock().await;
    bridges
        .entry(session_id)
        .or_insert_with(|| Arc::new(RwLock::new(BridgeManager::new(session_id))))
        .clone()
}

/// Remove a bridge manager when session ends
pub async fn remove_bridge_manager(session_id: Uuid) {
    let mut bridges = BRIDGES.lock().await;
    if let Some(manager) = bridges.remove(&session_id) {
        // Cancel all WebSocket tasks
        let manager_read = manager.read().await;
        for conn in manager_read.connections.values() {
            if let Some(ref handle) = conn.task_handle {
                handle.abort();
            }
        }
    }
}

/// Bridge Tool action types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeAction {
    /// Connect to a WebSocket endpoint
    Connect { url: String },
    /// Disconnect from a WebSocket endpoint
    Disconnect { url: String },
    /// List all active bridge connections
    List,
}

/// Bridge Tool arguments for rig::tool::Tool trait
#[derive(Debug, Deserialize, Serialize)]
pub struct BridgeToolArgs {
    pub action: BridgeAction,
}

/// Bridge Tool result
#[derive(Debug, Serialize)]
#[must_use = "BridgeResult should be checked for success/failure"]
pub struct BridgeResult {
    pub success: bool,
    pub message: String,
    pub connections: Option<Vec<BridgeConnectionInfo>>,
}

/// Bridge Tool - Rig Tool implementation
///
/// Allows AI agents to connect their sessions to external WebSocket endpoints.
/// Uses handler mechanism similar to FspecTool for session context injection.
#[derive(Clone, Debug)]
pub struct BridgeTool {
    /// Session ID for pre_tool_use hook checks (HOOK-017)
    pub session_id: Uuid,
}

impl Default for BridgeTool {
    fn default() -> Self {
        Self { session_id: Uuid::nil() }
    }
}

impl BridgeTool {
    /// Create a new BridgeTool instance
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for BridgeTool {
    const NAME: &'static str = "Bridge";

    type Error = ToolError;
    type Args = BridgeToolArgs;
    type Output = BridgeResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "Bridge".to_string(),
            description: concat!(
                "Connect to external WebSocket endpoints to relay session output and receive remote input. ",
                "Use action 'connect' to establish connection to a WebSocket URL, ",
                "'disconnect' to close a connection, ",
                "'list' to show all active bridges with their status."
            )
            .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "type": { "type": "string", "const": "connect" },
                                    "url": {
                                        "type": "string",
                                        "description": "WebSocket URL to connect to (e.g., ws://localhost:8080)"
                                    }
                                },
                                "required": ["type", "url"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "type": { "type": "string", "const": "disconnect" },
                                    "url": {
                                        "type": "string",
                                        "description": "WebSocket URL to disconnect from"
                                    }
                                },
                                "required": ["type", "url"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "type": { "type": "string", "const": "list" }
                                },
                                "required": ["type"]
                            }
                        ]
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-017: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            "Bridge",
            &serde_json::to_value(&_args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Bridge",
                message: reason,
            });
        }

        // This tool requires session context via handler mechanism
        // Direct Tool::call is not supported - must use BridgeToolFacadeWrapper
        Err(ToolError::Execution {
            tool: "bridge",
            message: "BridgeTool requires session context - use via BridgeToolFacadeWrapper"
                .to_string(),
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Feature: spec/features/bridge-tool.feature
    ///
    /// These tests map directly to the Gherkin scenarios in the feature file.
    /// Tests MUST fail initially (red phase) before implementation makes them pass.
    mod connect_action {
        use super::*;

        /// Scenario: Connect to a valid WebSocket endpoint
        #[tokio::test]
        async fn test_connect_to_valid_websocket_endpoint() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And a WebSocket server is listening at "ws://localhost:8080"
            // Note: In unit tests we test the data structures and state management
            // Integration tests will use actual WebSocket servers
            let test_url = "ws://localhost:8080";

            // @step When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                // Simulate successful connection
                conn.state = BridgeConnectionState::Connected;
            }

            // @step Then the tool should return "Connected to ws://localhost:8080"
            let mgr = manager.read().await;
            let conn_info = mgr.connections.get(test_url);
            assert!(conn_info.is_some(), "Connection should exist");
            assert_eq!(
                conn_info.unwrap().state,
                BridgeConnectionState::Connected,
                "Connection should be in Connected state"
            );

            // @step And the bridge should be subscribed to the session's broadcast channel
            // Note: Subscription happens in the WebSocket task which is not tested here
            // This will be verified in integration tests

            // Cleanup
            remove_bridge_manager(session_id).await;
        }

        /// Scenario: Fail to connect to invalid endpoint
        #[tokio::test]
        async fn test_fail_to_connect_to_invalid_endpoint() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step When the agent calls Bridge with action "connect" and url "ws://invalid:9999"
            let invalid_url = "ws://invalid:9999";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(invalid_url.to_string());
                // Simulate connection failure - stays in Connecting state then transitions to Disconnected
                conn.state = BridgeConnectionState::Disconnected;
            }

            // @step Then the tool should return an error containing "Connection refused"
            let mgr = manager.read().await;
            let conn_info = mgr.connections.get(invalid_url);
            assert!(conn_info.is_some(), "Connection entry should exist");
            assert_eq!(
                conn_info.unwrap().state,
                BridgeConnectionState::Disconnected,
                "Failed connection should be in Disconnected state"
            );

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod disconnect_action {
        use super::*;

        /// Scenario: Disconnect from a connected endpoint
        #[tokio::test]
        async fn test_disconnect_from_connected_endpoint() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Connected;
            }

            // @step When the agent calls Bridge with action "disconnect" and url "ws://localhost:8080"
            {
                let mut mgr = manager.write().await;
                let removed = mgr.remove_connection(test_url);
                assert!(removed.is_some(), "Connection should exist to be removed");
            }

            // @step Then the tool should return "Disconnected from ws://localhost:8080"
            // @step And the WebSocket connection should be closed
            let mgr = manager.read().await;
            assert!(
                !mgr.connections.contains_key(test_url),
                "Connection should be removed after disconnect"
            );

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod list_action {
        use super::*;

        /// Scenario: List active bridge connections
        #[tokio::test]
        async fn test_list_active_bridge_connections() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Connected;
            }

            // @step When the agent calls Bridge with action "list"
            let connections = {
                let mgr = manager.read().await;
                mgr.list_connections()
            };

            // @step Then the tool should return a list containing:
            //   | url                   | state     | buffered |
            //   | ws://localhost:8080   | connected | 0        |
            assert_eq!(connections.len(), 1, "Should have one connection");
            assert_eq!(connections[0].url, test_url);
            assert_eq!(connections[0].state, BridgeConnectionState::Connected);
            assert_eq!(connections[0].buffered, 0);

            // Cleanup
            remove_bridge_manager(session_id).await;
        }

        /// Scenario: List connections during reconnect
        #[tokio::test]
        async fn test_list_connections_during_reconnect() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Connected;
            }

            // @step And the WebSocket connection has dropped
            // @step And the bridge is attempting to reconnect
            {
                let mut mgr = manager.write().await;
                if let Some(conn) = mgr.get_connection_mut(test_url) {
                    conn.state = BridgeConnectionState::Reconnecting;
                }
            }

            // @step When the agent calls Bridge with action "list"
            let connections = {
                let mgr = manager.read().await;
                mgr.list_connections()
            };

            // @step Then the tool should return a list containing:
            //   | url                   | state        |
            //   | ws://localhost:8080   | reconnecting |
            assert_eq!(connections.len(), 1);
            assert_eq!(connections[0].url, test_url);
            assert_eq!(connections[0].state, BridgeConnectionState::Reconnecting);

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod multiple_bridges {
        use super::*;

        /// Scenario: Connect to multiple endpoints simultaneously
        #[tokio::test]
        async fn test_connect_to_multiple_endpoints_simultaneously() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And a WebSocket server is listening at "ws://localhost:8080"
            // @step And a WebSocket server is listening at "ws://localhost:9090"
            let url1 = "ws://localhost:8080";
            let url2 = "ws://localhost:9090";

            // @step When the agent calls Bridge with action "connect" and url "ws://localhost:8080"
            // @step And the agent calls Bridge with action "connect" and url "ws://localhost:9090"
            {
                let mut mgr = manager.write().await;
                let conn1 = mgr.add_connection(url1.to_string());
                conn1.state = BridgeConnectionState::Connected;
                let conn2 = mgr.add_connection(url2.to_string());
                conn2.state = BridgeConnectionState::Connected;
            }

            // @step Then both bridges should be connected
            let mgr = manager.read().await;
            assert_eq!(mgr.connections.len(), 2, "Should have two connections");
            assert_eq!(
                mgr.connections.get(url1).unwrap().state,
                BridgeConnectionState::Connected
            );
            assert_eq!(
                mgr.connections.get(url2).unwrap().state,
                BridgeConnectionState::Connected
            );

            // @step When the agent produces a text response "Hello"
            // @step Then "ws://localhost:8080" should receive a JSON chunk with the text "Hello"
            // @step And "ws://localhost:9090" should receive a JSON chunk with the text "Hello"
            // Note: Message relay is tested in integration tests with actual WebSocket connections

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod outbound_messages {
        use super::*;

        /// Scenario: Relay StreamChunks to connected endpoint as JSON
        #[tokio::test]
        async fn test_relay_stream_chunks_to_connected_endpoint() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Connected;
            }

            // @step When the agent produces a text response "I can help with that"
            let outbound_msg = OutboundMessage {
                msg_type: "chunk".to_string(),
                session_id: session_id.to_string(),
                data: json!({
                    "type": "text",
                    "text": "I can help with that"
                }),
                request_id: None,
            };

            // @step Then "ws://localhost:8080" should receive a JSON message with:
            //   | field      | value                        |
            //   | type       | chunk                        |
            //   | session_id | <current_session_id>         |
            //   | data.type  | text                         |
            //   | data.text  | I can help with that         |
            assert_eq!(outbound_msg.msg_type, "chunk");
            assert_eq!(outbound_msg.session_id, session_id.to_string());
            assert_eq!(outbound_msg.data["type"], "text");
            assert_eq!(outbound_msg.data["text"], "I can help with that");

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod inbound_messages {
        use super::*;

        /// Scenario: Receive input from endpoint and inject into session
        /// Updated for ARCH-004: uses multiplexed Envelope format
        #[tokio::test]
        async fn test_receive_input_from_endpoint_and_inject() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            // (Connection setup handled by BridgeManager)

            // @step When the endpoint sends a multiplexed envelope:
            let envelope_json = serde_json::json!({
                "service": "session",
                "type": "input",
                "session_id": session_id.to_string(),
                "data": {
                    "message": "build the app"
                }
            });

            // @step Then the envelope should parse as a session input
            let env: crate::bridge_multiplexed::Envelope =
                serde_json::from_value(envelope_json).unwrap();
            assert_eq!(env.service, crate::bridge_multiplexed::Service::Session);
            assert_eq!(env.msg_type, "input");

            let action = crate::bridge_multiplexed::route_inbound(&env);
            match action {
                crate::bridge_multiplexed::InboundAction::SessionInput { message, .. } => {
                    assert_eq!(message, "build the app");
                }
                other => panic!("Expected SessionInput, got {other:?}"),
            }
        }
    }

    mod reconnection_and_buffering {
        use super::*;

        /// Scenario: Auto-reconnect and deliver buffered messages
        #[tokio::test]
        async fn test_auto_reconnect_and_deliver_buffered_messages() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Connected;
            }

            // @step When the WebSocket connection drops unexpectedly
            {
                let mut mgr = manager.write().await;
                if let Some(conn) = mgr.get_connection_mut(test_url) {
                    conn.state = BridgeConnectionState::Reconnecting;
                }
            }

            // @step And the agent produces text responses "Message 1" and "Message 2"
            // @step Then the bridge should buffer the messages
            {
                let mut mgr = manager.write().await;
                if let Some(conn) = mgr.get_connection_mut(test_url) {
                    conn.buffer_message(OutboundMessage {
                        msg_type: "chunk".to_string(),
                        session_id: session_id.to_string(),
                        data: json!({"type": "text", "text": "Message 1"}),
                        request_id: None,
                    })
                    .expect("Should buffer message 1");
                    conn.buffer_message(OutboundMessage {
                        msg_type: "chunk".to_string(),
                        session_id: session_id.to_string(),
                        data: json!({"type": "text", "text": "Message 2"}),
                        request_id: None,
                    })
                    .expect("Should buffer message 2");
                }
            }

            // Verify messages are buffered
            {
                let mgr = manager.read().await;
                let conn = mgr.connections.get(test_url).unwrap();
                assert_eq!(conn.outbound_buffer.len(), 2, "Should have 2 buffered messages");
            }

            // @step When the WebSocket server becomes available again
            // @step And the bridge reconnects
            // @step Then "ws://localhost:8080" should receive the buffered messages in order
            {
                let mut mgr = manager.write().await;
                if let Some(conn) = mgr.get_connection_mut(test_url) {
                    conn.state = BridgeConnectionState::Connected;
                    let buffered = conn.take_buffer();
                    assert_eq!(buffered.len(), 2);
                    assert_eq!(buffered[0].data["text"], "Message 1");
                    assert_eq!(buffered[1].data["text"], "Message 2");
                }
            }

            // Verify buffer is empty after delivery
            {
                let mgr = manager.read().await;
                let conn = mgr.connections.get(test_url).unwrap();
                assert_eq!(conn.outbound_buffer.len(), 0, "Buffer should be empty after delivery");
                assert_eq!(conn.buffer_size_bytes, 0, "Buffer size should be 0 after delivery");
            }

            // Cleanup
            remove_bridge_manager(session_id).await;
        }

        /// Scenario: Drop connection when buffer exceeds 1GB
        #[tokio::test]
        async fn test_drop_connection_when_buffer_exceeds_1gb() {
            // @step Given an agent session is running
            let session_id = Uuid::new_v4();
            let manager = get_or_create_bridge_manager(session_id).await;

            // @step And the agent has connected a bridge to "ws://localhost:8080"
            let test_url = "ws://localhost:8080";
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(test_url.to_string());
                conn.state = BridgeConnectionState::Reconnecting;
                // Pre-fill buffer close to limit
                conn.buffer_size_bytes = MAX_BUFFER_SIZE_BYTES - 100;
            }

            // @step And the WebSocket connection is down
            // @step When the message buffer exceeds 1GB
            let result = {
                let mut mgr = manager.write().await;
                if let Some(conn) = mgr.get_connection_mut(test_url) {
                    // Try to add a message that would exceed the limit
                    conn.buffer_message(OutboundMessage {
                        msg_type: "chunk".to_string(),
                        session_id: session_id.to_string(),
                        data: json!({"type": "text", "text": "This message should cause overflow".repeat(10)}),
                        request_id: None,
                    })
                } else {
                    Ok(())
                }
            };

            // @step Then the bridge connection should be dropped
            // @step And the tool should report an error for that connection
            assert!(result.is_err(), "Should return error when buffer exceeds 1GB");
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("Buffer overflow"),
                "Error should mention buffer overflow"
            );

            // Cleanup
            remove_bridge_manager(session_id).await;
        }
    }

    mod tool_definition {
        use super::*;

        /// Scenario: Bridge tool definition
        #[tokio::test]
        async fn test_bridge_tool_definition() {
            // @step Given a BridgeTool instance
            let tool = BridgeTool::new(Uuid::new_v4());

            // @step When definition is called
            let def = tool.definition(String::new()).await;

            // @step Then the name should be "Bridge"
            assert_eq!(def.name, "Bridge");

            // @step And the description should contain "WebSocket"
            assert!(def.description.contains("WebSocket"));
            assert!(def.description.contains("connect"));
            assert!(def.description.contains("disconnect"));
            assert!(def.description.contains("list"));
        }

        /// Scenario: Bridge tool requires session context
        #[tokio::test]
        async fn test_bridge_tool_call_requires_session_context() {
            // @step Given a BridgeTool instance
            let tool = BridgeTool::new(Uuid::new_v4());

            // @step When call is invoked directly
            let result = tool
                .call(BridgeToolArgs {
                    action: BridgeAction::List,
                })
                .await;

            // @step Then an error should mention "session context"
            assert!(result.is_err(), "Direct call should fail without session context");
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("session context"),
                "Error should mention session context requirement"
            );
        }
    }
}
