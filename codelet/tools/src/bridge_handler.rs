//! Bridge command handler mechanism
//!
//! Provides a global handler for BridgeTool to manage WebSocket connections
//! with session context. Similar architecture to fspec_handler.rs.
//!
//! The handler is set per-session in agent_loop and enables the Bridge tool
//! to access session broadcast channels and input injection.
//!
//! ## Architecture
//!
//! 1. Session manager sets handler via `set_bridge_handler()` before agent run
//! 2. Session manager sets session context via `set_bridge_session_context(session_id, ...)`
//! 3. BridgeToolFacadeWrapper (constructed with session_id) calls `execute_bridge_command()`
//! 4. Handler spawns relay task with session context
//! 5. Handler returns result to BridgeTool
//!
//! ## Session Association (TOOL-012)
//!
//! Tools are constructed WITH their session_id at creation time. The session_id
//! is stored as a field on the wrapper struct and passed in the BridgeRequest.

use crate::bridge::{
    get_or_create_bridge_manager, BridgeAction, BridgeConnectionInfo, BridgeConnectionState,
    BridgeResult,
};
use crate::bridge_relay::{spawn_relay_task, CommandEmitter, ControlHandler, InputInjector};
use crate::ToolError;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Request to execute a bridge command
#[derive(Debug, Clone)]
pub struct BridgeRequest {
    /// Session ID for this request
    pub session_id: Uuid,
    /// The bridge action to execute
    pub action: BridgeAction,
}

/// Handler function type for bridge command execution
/// Takes a request and returns the result
pub type BridgeHandler = Arc<dyn Fn(BridgeRequest) -> BridgeResult + Send + Sync>;

/// Factory function to create a broadcast receiver
pub type BroadcastReceiverFactory = Arc<dyn Fn() -> broadcast::Receiver<serde_json::Value> + Send + Sync>;

/// Session context for bridge relay tasks
pub struct BridgeSessionContext {
    /// Factory to create broadcast receivers (can be called multiple times)
    pub broadcast_rx_factory: BroadcastReceiverFactory,
    /// Callback to inject input into the session
    pub input_injector: InputInjector,
    /// BRIDGE-008: Optional handler for control messages (interrupt, clear)
    pub control_handler: Option<ControlHandler>,
    /// BRIDGE-017: Optional emitter for fspec command execution
    pub command_emitter: Option<CommandEmitter>,
}

static BRIDGE_HANDLER: RwLock<Option<BridgeHandler>> = RwLock::new(None);
static BRIDGE_SESSION_CONTEXTS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, Arc<BridgeSessionContext>>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the bridge command handler
///
/// Called by session manager before agent run to configure how bridge commands
/// are executed with session context.
pub fn set_bridge_handler(handler: Option<BridgeHandler>) {
    if let Ok(mut guard) = BRIDGE_HANDLER.write() {
        *guard = handler;
    }
}

/// Set session context for bridge relay tasks
///
/// Called by session manager to provide broadcast channel and input injection
/// for the relay tasks.
///
/// BRIDGE-008: Now accepts an optional control_handler for interrupt/clear actions
/// BRIDGE-017: Now accepts an optional command_emitter for fspec command execution
pub fn set_bridge_session_context(
    session_id: Uuid,
    broadcast_rx_factory: BroadcastReceiverFactory,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,
) {
    if let Ok(mut guard) = BRIDGE_SESSION_CONTEXTS.write() {
        guard.insert(session_id, Arc::new(BridgeSessionContext {
            broadcast_rx_factory,
            input_injector,
            control_handler,
            command_emitter,
        }));
    }
}

/// Remove session context when session ends
pub fn remove_bridge_session_context(session_id: Uuid) {
    if let Ok(mut guard) = BRIDGE_SESSION_CONTEXTS.write() {
        guard.remove(&session_id);
    }
}

/// Get session context for a session
///
/// SESS-018: Made public so the bridge's inbound handler can route
/// session:input / session:control envelopes to the correct per-session
/// injector/handler instead of always using the connection owner's.
pub fn get_bridge_session_context(session_id: Uuid) -> Option<Arc<BridgeSessionContext>> {
    BRIDGE_SESSION_CONTEXTS
        .read()
        .ok()
        .and_then(|guard| guard.get(&session_id).cloned())
}

/// Execute a bridge command via the configured handler
///
/// Called by BridgeToolFacadeWrapper when the LLM invokes the Bridge tool.
///
/// Returns an error result if no handler is configured.
pub fn execute_bridge_command(request: BridgeRequest) -> BridgeResult {
    let handler = match BRIDGE_HANDLER.read() {
        Ok(guard) => guard.clone(),
        Err(_) => {
            return BridgeResult {
                success: false,
                message: "Failed to acquire bridge handler lock".to_string(),
                connections: None,
            };
        }
    };

    match handler {
        Some(h) => h(request),
        None => BridgeResult {
            success: false,
            message: "Bridge handler not configured - BridgeTool requires session context"
                .to_string(),
            connections: None,
        },
    }
}

/// Check if a bridge handler is configured for a specific session
///
/// Checks that both the global handler AND session context are configured.
pub fn has_bridge_handler_for_session(session_id: Uuid) -> bool {
    let has_handler = BRIDGE_HANDLER
        .read()
        .map(|guard| guard.is_some())
        .unwrap_or(false);

    let has_context = BRIDGE_SESSION_CONTEXTS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false);

    has_handler && has_context
}

/// Default implementation of bridge actions (used when handler is set up)
///
/// This function implements the actual bridge command logic that can be called
/// from the handler. The handler provides the session context (broadcast channel,
/// input sender) and delegates to these functions.
pub async fn handle_bridge_action(
    session_id: Uuid,
    action: BridgeAction,
) -> Result<BridgeResult, ToolError> {
    let manager = get_or_create_bridge_manager(session_id).await;

    match action {
        BridgeAction::Connect { url } => {
            // Validate URL
            if !url.starts_with("ws://") && !url.starts_with("wss://") {
                return Ok(BridgeResult {
                    success: false,
                    message: format!("Invalid WebSocket URL: {url}. Must start with ws:// or wss://"),
                    connections: None,
                });
            }

            // Check if session context is available
            let context = match get_bridge_session_context(session_id) {
                Some(ctx) => ctx,
                None => {
                    return Ok(BridgeResult {
                        success: false,
                        message: "Bridge session context not configured - cannot spawn relay task".to_string(),
                        connections: None,
                    });
                }
            };

            // Add connection entry
            {
                let mut mgr = manager.write().await;
                let conn = mgr.add_connection(url.clone());
                conn.state = BridgeConnectionState::Connecting;
            }

            // Get a broadcast receiver from the factory
            let broadcast_rx = (context.broadcast_rx_factory)();
            let input_injector = context.input_injector.clone();
            let control_handler = context.control_handler.clone();
            let command_emitter = context.command_emitter.clone();

            // Spawn the relay task (BRIDGE-008: with control_handler, BRIDGE-017: with command_emitter)
            match spawn_relay_task(session_id, url.clone(), broadcast_rx, input_injector, control_handler, command_emitter).await {
                Ok(handle) => {
                    // Store the task handle
                    let mut mgr = manager.write().await;
                    if let Some(conn) = mgr.get_connection_mut(&url) {
                        conn.task_handle = Some(handle);
                    }

                    Ok(BridgeResult {
                        success: true,
                        message: format!("Connected to {url}"),
                        connections: None,
                    })
                }
                Err(e) => {
                    // Remove the connection entry on failure
                    let mut mgr = manager.write().await;
                    mgr.remove_connection(&url);

                    Ok(BridgeResult {
                        success: false,
                        message: format!("Failed to connect to {url}: {e}"),
                        connections: None,
                    })
                }
            }
        }

        BridgeAction::Disconnect { url } => {
            let mut mgr = manager.write().await;

            if let Some(conn) = mgr.remove_connection(&url) {
                // Cancel the WebSocket task if running
                if let Some(handle) = conn.task_handle {
                    handle.abort();
                }

                Ok(BridgeResult {
                    success: true,
                    message: format!("Disconnected from {url}"),
                    connections: None,
                })
            } else {
                Ok(BridgeResult {
                    success: false,
                    message: format!("No active connection to {url}"),
                    connections: None,
                })
            }
        }

        BridgeAction::List => {
            let mgr = manager.read().await;
            let connections: Vec<BridgeConnectionInfo> = mgr.list_connections();

            if connections.is_empty() {
                Ok(BridgeResult {
                    success: true,
                    message: "No active bridge connections".to_string(),
                    connections: Some(connections),
                })
            } else {
                let summary = connections
                    .iter()
                    .map(|c| {
                        format!(
                            "  - {} ({:?}, {} buffered)",
                            c.url, c.state, c.buffered
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(BridgeResult {
                    success: true,
                    message: format!("Active bridge connections:\n{summary}"),
                    connections: Some(connections),
                })
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn with_clean_handler<T>(f: impl FnOnce() -> T) -> T {
        set_bridge_handler(None);
        let result = f();
        set_bridge_handler(None);
        result
    }

    #[test]
    #[serial]
    fn test_no_handler_returns_error() {
        with_clean_handler(|| {
            let result = execute_bridge_command(BridgeRequest {
                session_id: Uuid::new_v4(),
                action: BridgeAction::List,
            });

            assert!(!result.success);
            assert!(result.message.contains("not configured"));
        });
    }

    #[test]
    #[serial]
    fn test_handler_receives_request() {
        with_clean_handler(|| {
            use std::sync::atomic::{AtomicBool, Ordering};

            let called = Arc::new(AtomicBool::new(false));
            let called_clone = called.clone();

            let handler: BridgeHandler = Arc::new(move |req| {
                called_clone.store(true, Ordering::SeqCst);
                assert!(matches!(req.action, BridgeAction::List));
                BridgeResult {
                    success: true,
                    message: "test result".to_string(),
                    connections: Some(vec![]),
                }
            });

            set_bridge_handler(Some(handler));

            let result = execute_bridge_command(BridgeRequest {
                session_id: Uuid::new_v4(),
                action: BridgeAction::List,
            });

            assert!(called.load(Ordering::SeqCst));
            assert!(result.success);
            assert_eq!(result.message, "test result");
        });
    }

    #[test]
    #[serial]
    fn test_has_bridge_handler_for_session() {
        with_clean_handler(|| {
            let session_id = Uuid::new_v4();

            // No handler or context
            assert!(!has_bridge_handler_for_session(session_id));

            // Set handler only
            let handler: BridgeHandler = Arc::new(|_| BridgeResult {
                success: true,
                message: String::new(),
                connections: None,
            });
            set_bridge_handler(Some(handler));
            assert!(!has_bridge_handler_for_session(session_id)); // Still false - no context

            // Set context
            let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
            let broadcast_factory: BroadcastReceiverFactory = Arc::new(move || tx.subscribe());
            let input_injector: InputInjector = Arc::new(|_| {});
            set_bridge_session_context(session_id, broadcast_factory, input_injector, None, None);

            assert!(has_bridge_handler_for_session(session_id)); // Now true

            // Remove context
            remove_bridge_session_context(session_id);
            assert!(!has_bridge_handler_for_session(session_id));
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_bridge_action_list_empty() {
        let session_id = Uuid::new_v4();
        let result = handle_bridge_action(session_id, BridgeAction::List)
            .await
            .expect("Should succeed");

        assert!(result.success);
        assert!(result.message.contains("No active"));
        assert!(result.connections.is_some());
        assert!(result.connections.unwrap().is_empty());

        // Cleanup
        crate::bridge::remove_bridge_manager(session_id).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_bridge_action_invalid_url() {
        let session_id = Uuid::new_v4();
        let result = handle_bridge_action(
            session_id,
            BridgeAction::Connect {
                url: "http://invalid".to_string(),
            },
        )
        .await
        .expect("Should succeed with error message");

        assert!(!result.success);
        assert!(result.message.contains("Invalid WebSocket URL"));

        // Cleanup
        crate::bridge::remove_bridge_manager(session_id).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_handle_bridge_action_connect() {
        use tokio::sync::broadcast;
        
        let session_id = Uuid::new_v4();
        
        // Set up session context with mock factories
        let (broadcast_tx, _) = broadcast::channel::<serde_json::Value>(16);
        let broadcast_tx = Arc::new(broadcast_tx);
        let broadcast_tx_clone = broadcast_tx.clone();
        
        let broadcast_rx_factory: crate::bridge_handler::BroadcastReceiverFactory = 
            Arc::new(move || broadcast_tx_clone.subscribe());
        
        let input_injector: crate::bridge_relay::InputInjector = 
            Arc::new(|_input: crate::bridge_relay::InjectedInput| {
                // Mock input injector - do nothing
            });
        
        set_bridge_session_context(session_id, broadcast_rx_factory, input_injector, None, None);
        
        // This test will try to connect but fail because there's no server
        // The important thing is that it doesn't fail due to missing context
        let result = handle_bridge_action(
            session_id,
            BridgeAction::Connect {
                url: "ws://127.0.0.1:59999".to_string(), // Non-existent server
            },
        )
        .await
        .expect("Should not return ToolError");

        // The connect action spawns a task, so it may initially succeed
        // The task will then fail to connect and update state
        // For this test, we just verify we got past the context check
        assert!(result.success || result.message.contains("Failed"));

        // Cleanup
        remove_bridge_session_context(session_id);
        crate::bridge::remove_bridge_manager(session_id).await;
    }
}
