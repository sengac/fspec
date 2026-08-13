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
use crate::session_registry::SessionRegistry;
use crate::ToolError;
use std::sync::Arc;
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
pub type BroadcastReceiverFactory =
    Arc<dyn Fn() -> broadcast::Receiver<serde_json::Value> + Send + Sync>;

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

/// Per-session bridge handler storage (BUG-128: replaced global singleton).
static BRIDGE_HANDLERS: once_cell::sync::Lazy<SessionRegistry<BridgeHandler>> =
    once_cell::sync::Lazy::new(SessionRegistry::new);

/// Per-session bridge session context storage.
static BRIDGE_SESSION_CONTEXTS: once_cell::sync::Lazy<SessionRegistry<Arc<BridgeSessionContext>>> =
    once_cell::sync::Lazy::new(SessionRegistry::new);

/// Set the bridge command handler for a specific session.
///
/// Called by session manager before agent run to configure how bridge commands
/// are executed with session context.
pub fn set_bridge_handler(session_id: Uuid, handler: Option<BridgeHandler>) {
    BRIDGE_HANDLERS.set(session_id, handler);
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
    let context = Arc::new(BridgeSessionContext {
        broadcast_rx_factory,
        input_injector,
        control_handler,
        command_emitter,
    });
    BRIDGE_SESSION_CONTEXTS.set(session_id, Some(context));
}

/// Remove session context when session ends
pub fn remove_bridge_session_context(session_id: Uuid) {
    BRIDGE_SESSION_CONTEXTS.remove(&session_id);
}

/// Get session context for a session
///
/// SESS-018: Made public so the bridge's inbound handler can route
/// session:input / session:control envelopes to the correct per-session
/// injector/handler instead of always using the connection owner's.
pub fn get_bridge_session_context(session_id: Uuid) -> Option<Arc<BridgeSessionContext>> {
    BRIDGE_SESSION_CONTEXTS.get(&session_id)
}

/// Execute a bridge command via the configured handler for the request's session.
///
/// Called by BridgeToolFacadeWrapper when the LLM invokes the Bridge tool.
/// Looks up the handler by `request.session_id` from the per-session map.
///
/// Returns an error result if no handler is configured for this session.
pub fn execute_bridge_command(request: BridgeRequest) -> BridgeResult {
    match BRIDGE_HANDLERS.get(&request.session_id) {
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
/// Checks that both the per-session handler AND session context are configured.
pub fn has_bridge_handler_for_session(session_id: Uuid) -> bool {
    BRIDGE_HANDLERS.has(&session_id) && BRIDGE_SESSION_CONTEXTS.has(&session_id)
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
        BridgeAction::Connect { url } => handle_connect(session_id, url, &manager).await,

        BridgeAction::Disconnect { url } => handle_disconnect(url, &manager).await,

        BridgeAction::List => handle_list(&manager).await,
    }
}

/// Handle a WebSocket connect action
async fn handle_connect(
    session_id: Uuid,
    url: String,
    manager: &Arc<tokio::sync::RwLock<crate::bridge::BridgeManager>>,
) -> Result<BridgeResult, ToolError> {
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
                message: "Bridge session context not configured - cannot spawn relay task"
                    .to_string(),
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
    match spawn_relay_task(
        session_id,
        url.clone(),
        broadcast_rx,
        input_injector,
        control_handler,
        command_emitter,
    )
    .await
    {
        Ok(handle) => {
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

/// Handle a WebSocket disconnect action
async fn handle_disconnect(
    url: String,
    manager: &Arc<tokio::sync::RwLock<crate::bridge::BridgeManager>>,
) -> Result<BridgeResult, ToolError> {
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

/// Handle a list connections action
async fn handle_list(
    manager: &Arc<tokio::sync::RwLock<crate::bridge::BridgeManager>>,
) -> Result<BridgeResult, ToolError> {
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
            .map(|c| format!("  - {} ({:?}, {} buffered)", c.url, c.state, c.buffered))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(BridgeResult {
            success: true,
            message: format!("Active bridge connections:\n{summary}"),
            connections: Some(connections),
        })
    }
}
