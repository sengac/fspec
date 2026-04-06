//! Bridge WebSocket Relay Task — Multiplexed Envelope Protocol
//!
//! Handles the actual WebSocket connection and message relay between
//! the session's broadcast channel and the fspec-pro relay gateway.
//!
//! ARCH-004: The flat protocol is ELIMINATED. This module speaks ONLY the
//! multiplexed envelope protocol ({service, type, instance_id, ...}).
//!
//! Feature: spec/features/bridge-relay-multiplexed-wiring.feature

use crate::bridge::{get_or_create_bridge_manager, BridgeConnectionState};
use crate::bridge_multiplexed::{
    route_inbound, Envelope, InboundAction, InstanceMetadata, Service,
};
use crate::ToolError;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Maximum reconnection delay in seconds
const MAX_RECONNECT_DELAY_SECS: u64 = 30;

/// Initial reconnection delay in seconds
const INITIAL_RECONNECT_DELAY_SECS: u64 = 1;

// ── Types kept from original (used by NAPI session wiring) ──────────────────

/// Image data received from bridge endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
}

/// Input to be injected into the session
#[derive(Debug, Clone)]
pub struct InjectedInput {
    /// Text message content
    pub message: String,
    /// Optional images
    pub images: Option<Vec<ImageData>>,
}

impl InjectedInput {
    /// Create a new InjectedInput with just a message
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

/// Callback for injecting input into the session
pub type InputInjector = Arc<dyn Fn(InjectedInput) + Send + Sync>;

/// Callback for handling control actions (interrupt, clear, pause_response)
pub type ControlHandler = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;

/// Callback to emit FspecCommandRequest into the session
pub type CommandEmitter = Arc<dyn Fn(String, String, String, String) + Send + Sync>;

/// Pending commands map — tool_call_id → (request_id, command_name)
pub type PendingCommands = Arc<Mutex<HashMap<String, (String, String)>>>;

/// Callback to query the current session list from the session manager.
/// Returns a Vec of JSON values, each representing a session: `{id, state, name, provider_id, model_id}`.
pub type SessionListProvider = Arc<dyn Fn() -> Vec<serde_json::Value> + Send + Sync>;

/// Callback to query the current model info.
/// Returns `(provider_id, model_id)` if available.
pub type ModelInfoProvider = Arc<dyn Fn() -> (Option<String>, Option<String>) + Send + Sync>;

/// Global session list provider — set once at startup by the NAPI layer.
/// Used by `get_instance_metadata()` to populate `sessions`, `provider`, and `model`.
static SESSION_LIST_PROVIDER: RwLock<Option<SessionListProvider>> = RwLock::new(None);

/// Global model info provider — set once at startup by the NAPI layer.
static MODEL_INFO_PROVIDER: RwLock<Option<ModelInfoProvider>> = RwLock::new(None);

/// Set the global session list provider.
///
/// Called by the NAPI session manager layer to provide session listing capability
/// to the bridge relay without a direct dependency on `codelet-napi`.
pub fn set_session_list_provider(provider: Option<SessionListProvider>) {
    if let Ok(mut guard) = SESSION_LIST_PROVIDER.write() {
        *guard = provider;
    }
}

/// Set the global model info provider.
///
/// Called by the NAPI session manager layer to provide model info capability.
pub fn set_model_info_provider(provider: Option<ModelInfoProvider>) {
    if let Ok(mut guard) = MODEL_INFO_PROVIDER.write() {
        *guard = provider;
    }
}

/// Query the current session list via the registered provider.
fn query_session_list() -> Vec<serde_json::Value> {
    SESSION_LIST_PROVIDER
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|p| p()))
        .unwrap_or_default()
}

/// Query the current model info via the registered provider.
fn query_model_info() -> (Option<String>, Option<String>) {
    MODEL_INFO_PROVIDER
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|p| p()))
        .unwrap_or((None, None))
}

/// Sender for control messages to the outbound WebSocket writer.
///
/// Allows non-stream code (e.g., metadata update triggers) to inject
/// envelopes into the outbound WebSocket write loop.
pub type OutboundControlTx = mpsc::UnboundedSender<Envelope>;

/// Global per-session outbound control sender.
/// Each active bridge connection registers its outbound control channel here.
/// The session manager uses this to push `metadataUpdate` envelopes.
static OUTBOUND_CONTROL_SENDERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, Vec<OutboundControlTx>>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Register an outbound control sender for a session's bridge connection.
fn register_outbound_control(session_id: Uuid, tx: OutboundControlTx) {
    if let Ok(mut guard) = OUTBOUND_CONTROL_SENDERS.write() {
        guard.entry(session_id).or_default().push(tx);
    }
}

/// Remove all outbound control senders for a session.
fn remove_outbound_controls(session_id: Uuid) {
    if let Ok(mut guard) = OUTBOUND_CONTROL_SENDERS.write() {
        guard.remove(&session_id);
    }
}

/// Send a metadata update envelope to all bridge connections for all sessions.
///
/// Called by the NAPI layer when sessions change (created, destroyed, status change).
/// Builds the metadata with current sessions and sends `relay/metadataUpdate` to
/// every active bridge WebSocket.
pub fn broadcast_metadata_update() {
    let metadata = get_instance_metadata();
    let sessions_data = serde_json::json!({ "sessions": metadata.sessions });

    let senders = match OUTBOUND_CONTROL_SENDERS.read() {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };

    for txs in senders.values() {
        for tx in txs {
            let env = Envelope::relay_metadata_update(&metadata.name, sessions_data.clone());
            // Fire-and-forget: if the channel is closed the bridge is disconnected
            let _ = tx.send(env);
        }
    }
}

// ── Outbound envelope action ────────────────────────────────────────────────

/// Result of processing an outbound chunk for the multiplexed protocol.
#[derive(Debug)]
pub enum OutboundEnvelopeAction {
    /// Send as {service:"relay", type:"chunk", ...}
    RelayChunk(Envelope),
    /// Send as {service:"fspec", type:"commandResponse", ...}
    CommandResponse(Envelope),
    /// Skip the chunk (fspecCommandRequest or unmatched result)
    Skip,
}

// ── Instance metadata ───────────────────────────────────────────────────────

/// Build instance metadata from the current environment.
///
/// Instance name is derived from the last path component of CWD.
/// Sessions are queried via the registered `SessionListProvider`.
/// Provider/model are queried via the registered `ModelInfoProvider`.
pub fn get_instance_metadata() -> InstanceMetadata {
    let cwd = std::env::current_dir().unwrap_or_default();
    let name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let path = cwd.to_string_lossy().to_string();
    let os = std::env::consts::OS.to_string();

    let sessions = query_session_list();
    let (provider, model) = query_model_info();

    InstanceMetadata {
        name,
        path: Some(path),
        version: None,
        os: Some(os),
        provider,
        model,
        sessions,
    }
}

// ── Outbound processing ─────────────────────────────────────────────────────

/// Process an outbound chunk and produce the appropriate Envelope action.
///
/// - Regular chunks → Envelope::relay_chunk()
/// - FspecCommandResult → Envelope::fspec_command_response() if pending
/// - FspecCommandRequest → Skip (for TypeScript only)
pub fn process_outbound_envelope(
    chunk_json: &serde_json::Value,
    instance_id: &str,
    session_id: &str,
    pending_commands: Option<&PendingCommands>,
) -> OutboundEnvelopeAction {
    let chunk_type = chunk_json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    match chunk_type {
        "fspecCommandRequest" => OutboundEnvelopeAction::Skip,
        "fspecCommandResult" => {
            let fspec_result = chunk_json.get("fspecResult");
            let tool_call_id = fspec_result
                .and_then(|r| r.get("toolCallId"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            if tool_call_id.is_empty() {
                return OutboundEnvelopeAction::Skip;
            }

            let pending_entry = pending_commands.and_then(|pending| {
                let mut map = pending.lock().ok()?;
                map.remove(tool_call_id)
            });

            match pending_entry {
                Some((request_id, command_name)) => {
                    let success = fspec_result
                        .and_then(|r| r.get("success"))
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let data = fspec_result
                        .and_then(|r| r.get("data"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let error = fspec_result
                        .and_then(|r| r.get("error"))
                        .and_then(|e| e.as_str());

                    let result_value = serde_json::from_str(data)
                        .unwrap_or_else(|_| serde_json::Value::String(data.to_string()));

                    let env = Envelope::fspec_command_response(
                        instance_id,
                        &request_id,
                        &command_name,
                        success,
                        result_value,
                        error,
                    );
                    OutboundEnvelopeAction::CommandResponse(env)
                }
                None => OutboundEnvelopeAction::Skip,
            }
        }
        _ => {
            let env = Envelope::relay_chunk(
                instance_id,
                session_id,
                chunk_json.clone(),
            );
            OutboundEnvelopeAction::RelayChunk(env)
        }
    }
}

// ── Inbound processing ──────────────────────────────────────────────────────

/// Handle an inbound multiplexed envelope message.
///
/// Parses the text as an Envelope, routes via route_inbound(), and dispatches
/// to the appropriate callback. Returns Ok(Some(envelope)) when a response
/// needs to be sent back (e.g., pong), Ok(None) otherwise.
pub async fn handle_multiplexed_inbound(
    text: &str,
    _session_id: Uuid,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,
    pending_commands: Option<PendingCommands>,
) -> Result<Option<Envelope>, String> {
    let envelope: Envelope = serde_json::from_str(text)
        .map_err(|e| format!("Failed to parse envelope: {e}"))?;

    let action = route_inbound(&envelope);

    match action {
        InboundAction::SessionInput {
            session_id: _,
            message,
            images,
        } => {
            let injected = match images {
                Some(imgs) if !imgs.is_empty() => {
                    tracing::info!("Injecting input with {} image(s): {}", imgs.len(), message);
                    InjectedInput::with_images(message, imgs)
                }
                _ => {
                    tracing::info!("Injecting text input: {}", message);
                    InjectedInput::text_only(message)
                }
            };
            input_injector(injected);
            Ok(None)
        }
        InboundAction::SessionControl {
            session_id: _,
            action,
            response,
        } => {
            if let Some(handler) = control_handler {
                tracing::info!("Handling control action: {}", action);
                handler(&action, response.as_deref());
            } else {
                tracing::warn!("Received control '{}' but no handler configured", action);
            }
            Ok(None)
        }
        InboundAction::FspecCommand {
            request_id,
            command,
            args_json,
        } => {
            if let Some(emitter) = command_emitter {
                let tool_call_id = Uuid::new_v4().to_string();
                tracing::info!(
                    "Bridge command: {} (request_id: {}, tool_call_id: {})",
                    command,
                    request_id,
                    tool_call_id
                );

                if let Some(pending) = pending_commands {
                    if let Ok(mut map) = pending.lock() {
                        map.insert(
                            tool_call_id.clone(),
                            (request_id, command.clone()),
                        );
                    }
                }

                let project_root = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string());

                emitter(command, args_json, project_root, tool_call_id);
            } else {
                tracing::warn!("Received command but no emitter configured");
            }
            Ok(None)
        }
        InboundAction::SystemPing => {
            Ok(Some(Envelope::system_pong()))
        }
        InboundAction::AuthResponse { success, data } => {
            tracing::info!("Auth response: success={}, data={:?}", success, data);
            Ok(None)
        }
        InboundAction::TerminalCreate { .. }
        | InboundAction::TerminalInput { .. }
        | InboundAction::TerminalResize { .. }
        | InboundAction::TerminalDestroy { .. } => {
            // Terminal actions handled by bridge_pty — not dispatched here
            tracing::warn!("Terminal action received but not yet wired to PtyRegistry");
            Ok(None)
        }
        InboundAction::Unknown { service, msg_type } => {
            tracing::warn!("Unknown inbound: service={}, type={}", service, msg_type);
            Ok(None)
        }
    }
}

// ── Relay task ──────────────────────────────────────────────────────────────

/// Spawn a WebSocket relay task for a bridge connection.
pub async fn spawn_relay_task(
    session_id: Uuid,
    url: String,
    stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,
) -> Result<tokio::task::JoinHandle<()>, ToolError> {
    let handle = tokio::spawn(async move {
        relay_loop(
            session_id,
            url,
            stream_rx,
            input_injector,
            control_handler,
            command_emitter,
        )
        .await;
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
    command_emitter: Option<CommandEmitter>,
) {
    let mut reconnect_delay = Duration::from_secs(INITIAL_RECONNECT_DELAY_SECS);

    loop {
        match connect_and_relay(
            session_id,
            &url,
            &mut stream_rx,
            input_injector.clone(),
            control_handler.clone(),
            command_emitter.clone(),
        )
        .await
        {
            Ok(()) => {
                tracing::info!("Bridge {} cleanly disconnected", url);
                break;
            }
            Err(e) => {
                tracing::warn!(
                    "Bridge {} connection error: {}, reconnecting in {:?}",
                    url,
                    e,
                    reconnect_delay
                );
                update_connection_state(session_id, &url, BridgeConnectionState::Reconnecting)
                    .await;
                tokio::time::sleep(reconnect_delay).await;
                reconnect_delay = std::cmp::min(
                    reconnect_delay * 2,
                    Duration::from_secs(MAX_RECONNECT_DELAY_SECS),
                );
            }
        }

        let manager = get_or_create_bridge_manager(session_id).await;
        let mgr = manager.read().await;
        if !mgr.connections.contains_key(&url) {
            tracing::info!("Bridge {} was removed, stopping relay", url);
            break;
        }
    }
}

/// Connect to WebSocket and handle message relay using multiplexed protocol.
async fn connect_and_relay(
    session_id: Uuid,
    url: &str,
    stream_rx: &mut broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,
) -> Result<(), String> {
    let (ws_stream, _) = connect_async(url)
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;

    tracing::info!("Bridge connected to {}", url);
    update_connection_state(session_id, url, BridgeConnectionState::Connected).await;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    // ── Auth handshake ──────────────────────────────────────────────────
    let metadata = get_instance_metadata();
    let auth_env = Envelope::auth_agent("", &metadata);
    let auth_json = serde_json::to_string(&auth_env)
        .map_err(|e| format!("Failed to serialize auth envelope: {e}"))?;

    ws_write
        .send(Message::Text(auth_json.into()))
        .await
        .map_err(|e| format!("Failed to send auth envelope: {e}"))?;

    tracing::info!("Sent auth envelope for instance: {}", metadata.name);

    // Wait for auth response
    let auth_response = ws_read
        .next()
        .await
        .ok_or_else(|| "Connection closed before auth response".to_string())?
        .map_err(|e| format!("WebSocket error during auth: {e}"))?;

    if let Message::Text(text) = auth_response {
        let env: Envelope = serde_json::from_str(text.as_ref())
            .map_err(|e| format!("Failed to parse auth response: {e}"))?;

        if env.service == Service::Auth && env.msg_type == "authError" {
            let code = env
                .data
                .as_ref()
                .and_then(|d| d.get("code"))
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN");
            return Err(format!("Auth failed: {code}"));
        }

        if env.service != Service::Auth || env.msg_type != "authSuccess" {
            return Err(format!(
                "Unexpected auth response: service={:?}, type={}",
                env.service, env.msg_type
            ));
        }

        tracing::info!("Auth successful");
    } else {
        return Err("Expected text message for auth response".to_string());
    }

    // ── Message relay loop ──────────────────────────────────────────────
    let instance_id = metadata.name.clone();
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

    // Register outbound control channel for metadata updates
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<Envelope>();
    register_outbound_control(session_id, control_tx);

    // Spawn inbound handler
    let inbound_url = url.to_string();
    let inbound_shutdown_tx = shutdown_tx.clone();
    let inbound_control_handler = control_handler.clone();
    let inbound_command_emitter = command_emitter.clone();
    let inbound_pending_commands = pending_commands.clone();
    let inbound_handle = tokio::spawn(async move {
        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    match handle_multiplexed_inbound(
                        text.as_ref(),
                        session_id,
                        input_injector.clone(),
                        inbound_control_handler.clone(),
                        inbound_command_emitter.clone(),
                        Some(inbound_pending_commands.clone()),
                    )
                    .await
                    {
                        Ok(Some(response_env)) => {
                            // Need to send response (e.g., pong) — can't write from
                            // inbound task since ws_write is in the outbound loop.
                            // For now, log. Real pong needs channel to outbound.
                            tracing::debug!(
                                "Inbound produced response: {:?}",
                                response_env.msg_type
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!("Failed to handle inbound: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::warn!("WebSocket {} received close frame", inbound_url);
                    let _ = inbound_shutdown_tx.send(()).await;
                    break;
                }
                Ok(_) => {}
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
    let session_id_str = session_id.to_string();
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("Bridge {} received shutdown signal", outbound_url);
                break;
            }
            Some(control_env) = control_rx.recv() => {
                let msg_json = match serde_json::to_string(&control_env) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::warn!("Failed to serialize control envelope: {}", e);
                        continue;
                    }
                };
                if let Err(e) = ws_write.send(Message::Text(msg_json.into())).await {
                    tracing::warn!("Failed to send control envelope to WebSocket: {}", e);
                    remove_outbound_controls(session_id);
                    return Err(format!("Send failed: {e}"));
                }
            }
            chunk_result = stream_rx.recv() => {
                match chunk_result {
                    Ok(chunk_json) => {
                        let action = process_outbound_envelope(
                            &chunk_json,
                            &instance_id,
                            &session_id_str,
                            Some(&pending_commands),
                        );

                        let envelope = match action {
                            OutboundEnvelopeAction::RelayChunk(env) => env,
                            OutboundEnvelopeAction::CommandResponse(env) => env,
                            OutboundEnvelopeAction::Skip => continue,
                        };

                        let msg_json = match serde_json::to_string(&envelope) {
                            Ok(json) => json,
                            Err(e) => {
                                tracing::warn!("Failed to serialize envelope: {}", e);
                                continue;
                            }
                        };

                        if let Err(e) = ws_write.send(Message::Text(msg_json.into())).await {
                            tracing::warn!("Failed to send to WebSocket: {}", e);
                            remove_outbound_controls(session_id);
                            return Err(format!("Send failed: {e}"));
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Bridge {} lagged {} messages", outbound_url, n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Bridge {} broadcast channel closed", outbound_url);
                        break;
                    }
                }
            }
        }
    }

    remove_outbound_controls(session_id);
    let _ = inbound_handle.await;
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Update connection state in the manager
async fn update_connection_state(session_id: Uuid, url: &str, state: BridgeConnectionState) {
    let manager = get_or_create_bridge_manager(session_id).await;
    let mut mgr = manager.write().await;
    if let Some(conn) = mgr.get_connection_mut(url) {
        conn.state = state;
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
        assert!(input.images.is_none());
    }

    #[test]
    fn test_get_instance_metadata_has_name() {
        let metadata = get_instance_metadata();
        assert!(!metadata.name.is_empty());
        assert!(metadata.path.is_some());
        assert!(metadata.os.is_some());
    }

    #[test]
    fn test_process_outbound_envelope_regular_chunk() {
        let chunk = serde_json::json!({"type": "text", "text": "Hello"});
        let result = process_outbound_envelope(&chunk, "proj", "s1", None);
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.service, Service::Relay);
                assert_eq!(env.msg_type, "chunk");
                assert_eq!(env.instance_id.as_deref(), Some("proj"));
                assert_eq!(env.session_id.as_deref(), Some("s1"));
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    #[test]
    fn test_process_outbound_envelope_skip_fspec_request() {
        let chunk = serde_json::json!({"type": "fspecCommandRequest"});
        let result = process_outbound_envelope(&chunk, "proj", "s1", None);
        assert!(matches!(result, OutboundEnvelopeAction::Skip));
    }

    #[test]
    fn test_process_outbound_envelope_command_response() {
        let pending: PendingCommands = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut map = pending.lock().unwrap();
            map.insert("tc1".to_string(), ("r1".to_string(), "board".to_string()));
        }
        let chunk = serde_json::json!({
            "type": "fspecCommandResult",
            "fspecResult": {
                "success": true,
                "data": "{\"columns\":{}}",
                "toolCallId": "tc1"
            }
        });
        let result = process_outbound_envelope(&chunk, "proj", "s1", Some(&pending));
        match result {
            OutboundEnvelopeAction::CommandResponse(env) => {
                assert_eq!(env.service, Service::Fspec);
                assert_eq!(env.request_id.as_deref(), Some("r1"));
            }
            other => panic!("Expected CommandResponse, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_multiplexed_inbound_input() {
        let received = Arc::new(Mutex::new(String::new()));
        let r = received.clone();
        let injector: InputInjector = Arc::new(move |input: InjectedInput| {
            *r.lock().unwrap() = input.message;
        });

        let text = r#"{"service":"session","type":"input","session_id":"s1","data":{"message":"hi"}}"#;
        let result =
            handle_multiplexed_inbound(text, Uuid::new_v4(), injector, None, None, None).await;
        assert!(result.is_ok());
        assert_eq!(*received.lock().unwrap(), "hi");
    }

    #[tokio::test]
    async fn test_handle_multiplexed_inbound_ping() {
        let injector: InputInjector = Arc::new(|_| {});
        let text = r#"{"service":"system","type":"ping"}"#;
        let result =
            handle_multiplexed_inbound(text, Uuid::new_v4(), injector, None, None, None).await;
        match result {
            Ok(Some(env)) => {
                assert_eq!(env.service, Service::System);
                assert_eq!(env.msg_type, "pong");
            }
            other => panic!("Expected pong envelope, got {other:?}"),
        }
    }
}
