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

// ── SESS-017: Session creator + PTY registry providers ─────────────────────

/// Callback that spawns a new codelet session and returns the new session_id.
///
/// SESS-017: Registered by the NAPI layer (`init_bridge_session_creator()`).
/// The bridge calls this when a `session:create` envelope arrives so the
/// dashboard's "+ > New fspec Session" click can actually spawn a session.
pub type SessionCreator =
    Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

/// Global session creator. Set once at startup by the NAPI layer.
static SESSION_CREATOR: RwLock<Option<SessionCreator>> = RwLock::new(None);

/// Register the global session creator.
pub fn set_session_creator(creator: Option<SessionCreator>) {
    if let Ok(mut guard) = SESSION_CREATOR.write() {
        *guard = creator;
    }
}

/// Query the registered session creator (returns a clone of the Arc).
fn query_session_creator() -> Option<SessionCreator> {
    SESSION_CREATOR
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

/// Global PTY registry. Set once at startup so the bridge can spawn
/// PTYs in response to terminal:create envelopes (SESS-017 / FIX 2).
static PTY_REGISTRY: RwLock<Option<Arc<crate::PtyRegistry>>> = RwLock::new(None);

/// Register the global PTY registry.
pub fn set_pty_registry(registry: Option<Arc<crate::PtyRegistry>>) {
    if let Ok(mut guard) = PTY_REGISTRY.write() {
        *guard = registry;
    }
}

/// Query the registered PTY registry.
fn query_pty_registry() -> Option<Arc<crate::PtyRegistry>> {
    PTY_REGISTRY
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

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

// ── SESS-015: Subordinate chunk forwarding channels ─────────────────────────

/// Sender type for subordinate chunks: (subordinate_session_id, chunk_json)
pub type SubordinateChunkTx = mpsc::UnboundedSender<(Uuid, serde_json::Value)>;

/// Receiver type for subordinate chunks
pub type SubordinateChunkRx = mpsc::UnboundedReceiver<(Uuid, serde_json::Value)>;

/// Global per-session subordinate chunk senders.
///
/// When a subordinate session is spawned, a forwarding task subscribes to the
/// subordinate's supervisor_broadcast and sends chunks here. The parent's relay
/// loop reads from the corresponding receiver and sends them over WebSocket
/// with the subordinate's session_id.
static SUBORDINATE_CHUNK_SENDERS: once_cell::sync::Lazy<
    RwLock<HashMap<Uuid, Vec<SubordinateChunkTx>>>,
> = once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a subordinate chunk channel for a parent session.
///
/// Returns the receiver that the relay loop should select! on.
/// Multiple relay connections for the same session each get their own channel.
pub fn register_subordinate_chunk_channel(parent_session_id: Uuid) -> SubordinateChunkRx {
    let (tx, rx) = mpsc::unbounded_channel();
    if let Ok(mut guard) = SUBORDINATE_CHUNK_SENDERS.write() {
        guard.entry(parent_session_id).or_default().push(tx);
    }
    rx
}

/// Get a clone of all subordinate chunk senders for a parent session.
///
/// Used by agent_manager_handler to get a sender for forwarding subordinate
/// chunks to the parent's relay loop.
pub fn get_subordinate_chunk_senders(parent_session_id: Uuid) -> Vec<SubordinateChunkTx> {
    SUBORDINATE_CHUNK_SENDERS
        .read()
        .ok()
        .and_then(|guard| guard.get(&parent_session_id).cloned())
        .unwrap_or_default()
}

/// Remove all subordinate chunk senders for a session.
fn remove_subordinate_chunk_senders(session_id: Uuid) {
    if let Ok(mut guard) = SUBORDINATE_CHUNK_SENDERS.write() {
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
///
/// SESS-015: If the chunk JSON contains a `_relay_session_id` field, that
/// value is used as the envelope's session_id instead of the relay-level
/// `session_id` parameter. This allows subordinate session chunks forwarded
/// through the parent's broadcast to retain their correct session identity.
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
            // SESS-015: Prefer chunk-level session_id for subordinate forwarding
            let effective_session_id = chunk_json
                .get("_relay_session_id")
                .and_then(|v| v.as_str())
                .unwrap_or(session_id);

            let env = Envelope::relay_chunk(
                instance_id,
                effective_session_id,
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
///
/// SESS-018: `connection_owner` identifies the WebSocket connection owner and
/// is used to (a) scope `OUTBOUND_CONTROL_SENDERS` lookups for the PTY reader
/// task spawned in `TerminalCreate` and (b) act as the fallback routing target
/// when an envelope's `session_id` has no registered `BridgeSessionContext`.
pub async fn handle_multiplexed_inbound(
    text: &str,
    connection_owner: Uuid,
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
            session_id,
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

            // SESS-018: Route to the per-session injector registered via
            // `set_bridge_session_context(sid, ...)` when one exists for the
            // envelope's session_id. Fall back to the parameter `input_injector`
            // (the supervisor/connection-owner) for legacy single-session
            // traffic and when no context is registered for the target id.
            let per_session_injector = Uuid::parse_str(&session_id)
                .ok()
                .and_then(crate::get_bridge_session_context)
                .map(|ctx| ctx.input_injector.clone());

            if let Some(injector) = per_session_injector {
                injector(injected);
            } else {
                input_injector(injected);
            }
            Ok(None)
        }
        InboundAction::SessionControl {
            session_id,
            action,
            response,
        } => {
            // SESS-018: Same per-session dispatch as SessionInput. Only fall
            // back to the parameter `control_handler` when no per-session
            // context has its own `control_handler` registered.
            let per_session_handler = Uuid::parse_str(&session_id)
                .ok()
                .and_then(crate::get_bridge_session_context)
                .and_then(|ctx| ctx.control_handler.clone());

            if let Some(handler) = per_session_handler {
                tracing::info!("Handling control action (per-session): {}", action);
                handler(&action, response.as_deref());
            } else if let Some(handler) = control_handler {
                tracing::info!("Handling control action: {}", action);
                handler(&action, response.as_deref());
            } else {
                tracing::warn!(
                    "Received control '{}' but no handler configured",
                    action
                );
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
        InboundAction::SessionCreate { request_id } => {
            // SESS-017 FIX 1: Invoke the registered SessionCreator and respond
            // with a session:created envelope so the dashboard's "+ > New fspec
            // Session" click can actually create a session.
            let metadata = get_instance_metadata();
            let instance_id = metadata.name;

            match query_session_creator() {
                Some(creator) => match creator() {
                    Ok(new_session_id) => {
                        tracing::info!(
                            "session:create handled — new session {}",
                            new_session_id
                        );
                        Ok(Some(Envelope::session_created(
                            &instance_id,
                            &request_id,
                            &new_session_id,
                        )))
                    }
                    Err(e) => {
                        tracing::warn!(
                            "session:create failed: {} — sending error envelope",
                            e
                        );
                        // Reply with a session:created envelope carrying empty
                        // session_id + an error field so the dashboard can
                        // surface the failure rather than hang forever.
                        Ok(Some(Envelope {
                            service: Service::Session,
                            msg_type: "created".to_string(),
                            instance_id: Some(instance_id),
                            session_id: None,
                            terminal_id: None,
                            request_id: Some(request_id),
                            data: Some(serde_json::json!({
                                "session_id": "",
                                "error": e,
                            })),
                        }))
                    }
                },
                None => {
                    tracing::warn!(
                        "session:create received but no SessionCreator registered"
                    );
                    Ok(Some(Envelope {
                        service: Service::Session,
                        msg_type: "created".to_string(),
                        instance_id: Some(instance_id),
                        session_id: None,
                        terminal_id: None,
                        request_id: Some(request_id),
                        data: Some(serde_json::json!({
                            "session_id": "",
                            "error": "No SessionCreator registered on bridge",
                        })),
                    }))
                }
            }
        }
        InboundAction::TerminalCreate { request_id, cols, rows, shell, cwd } => {
            // SESS-017 FIX 2: Spawn a PTY via the registered PtyRegistry and
            // respond with a terminal:created envelope. Without this the
            // dashboard's "+ > New Terminal" click hangs forever.
            //
            // SESS-018: After successfully creating the PTY, spawn a reader
            // task that drains the PTY master and forwards each chunk as a
            // `terminal:data` envelope through OUTBOUND_CONTROL_SENDERS keyed
            // by the connection owner (the Uuid of this relay connection).
            let metadata = get_instance_metadata();
            let instance_id = metadata.name;

            match query_pty_registry() {
                Some(registry) => {
                    let opts = crate::CreateTerminalOpts { cols, rows, shell, cwd };
                    match crate::create_terminal(&registry, &opts) {
                        Ok((terminal_id, entry)) => {
                            tracing::info!(
                                "terminal:create handled — new terminal {}",
                                terminal_id
                            );
                            spawn_pty_reader_task(
                                connection_owner,
                                instance_id.clone(),
                                terminal_id.clone(),
                                entry,
                            );
                            Ok(Some(Envelope::terminal_created(
                                &instance_id,
                                &request_id,
                                &terminal_id,
                            )))
                        }
                        Err(e) => {
                            tracing::warn!("terminal:create failed: {}", e);
                            Ok(Some(Envelope {
                                service: Service::Terminal,
                                msg_type: "created".to_string(),
                                instance_id: Some(instance_id),
                                session_id: None,
                                terminal_id: None,
                                request_id: Some(request_id),
                                data: Some(serde_json::json!({
                                    "terminal_id": "",
                                    "error": e,
                                })),
                            }))
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "terminal:create received but no PtyRegistry registered"
                    );
                    Ok(Some(Envelope {
                        service: Service::Terminal,
                        msg_type: "created".to_string(),
                        instance_id: Some(instance_id),
                        session_id: None,
                        terminal_id: None,
                        request_id: Some(request_id),
                        data: Some(serde_json::json!({
                            "terminal_id": "",
                            "error": "No PtyRegistry registered on bridge",
                        })),
                    }))
                }
            }
        }
        InboundAction::TerminalInput { terminal_id, base64_data } => {
            // SESS-018: Decode the base64 payload and write the bytes to the
            // PTY's stdin via PtyRegistry. The shell echoes typed characters
            // on its output stream, which the spawned reader task forwards as
            // terminal:data envelopes.
            use base64::Engine;

            let Some(registry) = query_pty_registry() else {
                tracing::warn!(
                    "terminal:input received but no PtyRegistry registered"
                );
                return Ok(None);
            };
            let Some(entry) = registry.get(&terminal_id) else {
                tracing::warn!(
                    "terminal:input for unknown terminal {}",
                    terminal_id
                );
                return Ok(None);
            };
            let bytes = match base64::engine::general_purpose::STANDARD.decode(&base64_data) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        "terminal:input base64 decode failed for {}: {}",
                        terminal_id,
                        e
                    );
                    return Ok(None);
                }
            };
            if let Err(e) = crate::write_terminal_input(&entry, &bytes).await {
                tracing::warn!("terminal:input write failed: {}", e);
            }
            Ok(None)
        }
        InboundAction::TerminalResize { terminal_id, cols, rows } => {
            // SESS-018: Resize the PTY via PtyRegistry so the shell receives
            // SIGWINCH and reflows output to the new dimensions.
            let Some(registry) = query_pty_registry() else {
                tracing::warn!(
                    "terminal:resize received but no PtyRegistry registered"
                );
                return Ok(None);
            };
            let Some(entry) = registry.get(&terminal_id) else {
                tracing::warn!(
                    "terminal:resize for unknown terminal {}",
                    terminal_id
                );
                return Ok(None);
            };
            if let Err(e) = crate::resize_terminal(&entry, cols, rows).await {
                tracing::warn!("terminal:resize failed: {}", e);
            }
            Ok(None)
        }
        InboundAction::TerminalDestroy { terminal_id, request_id } => {
            // SESS-018: Kill the PTY process, remove it from the registry, and
            // respond with a terminal:destroyed envelope so the dashboard can
            // unmount the tab. Always responds — even if the registry is
            // missing or the terminal is unknown — so the client does not
            // hang on the close handshake.
            let metadata = get_instance_metadata();
            let instance_id = metadata.name;

            match query_pty_registry() {
                Some(registry) => {
                    if let Err(e) = crate::destroy_terminal(&registry, &terminal_id).await {
                        tracing::warn!(
                            "terminal:destroy failed for {}: {}",
                            terminal_id,
                            e
                        );
                    }
                }
                None => {
                    tracing::warn!(
                        "terminal:destroy received but no PtyRegistry registered"
                    );
                }
            }

            Ok(Some(Envelope::terminal_destroyed(
                &instance_id,
                &request_id,
                &terminal_id,
            )))
        }
        InboundAction::Unknown { service, msg_type } => {
            tracing::warn!("Unknown inbound: service={}, type={}", service, msg_type);
            Ok(None)
        }
    }
}

// ── Relay task ──────────────────────────────────────────────────────────────

/// Spawn a background reader that drains a PTY master and emits
/// `terminal:data` envelopes through `OUTBOUND_CONTROL_SENDERS` keyed by
/// the connection owner.
///
/// SESS-018: `TerminalCreate` must not only spawn the shell but also wire
/// its output back to the dashboard. Each read chunk is base64-encoded,
/// wrapped in a `terminal:data` envelope, and fanned out to every registered
/// outbound control sender for the connection owner. The reader exits on
/// EOF, read error, or when no outbound senders remain (the connection
/// owner has disconnected and any further output would be lost anyway).
///
/// The outer task clones the PTY reader under an async lock, then hands the
/// synchronous `Read` object to a blocking thread so the sync read loop
/// does not stall the tokio runtime.
fn spawn_pty_reader_task(
    connection_owner: Uuid,
    instance_id: String,
    terminal_id: String,
    entry: Arc<crate::PtyEntry>,
) {
    tokio::spawn(async move {
        // Clone the blocking reader under the async master lock. We drop the
        // guard before spawn_blocking so the write path (which also locks
        // the master for resize) is not serialized behind the read loop.
        let reader = {
            let master = entry.master.lock().await;
            match master.try_clone_reader() {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        "failed to clone PTY reader for {}: {}",
                        terminal_id,
                        e
                    );
                    return;
                }
            }
        };

        let terminal_id_for_task = terminal_id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            use base64::Engine;
            use std::io::Read;

            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF — shell exited
                    Ok(n) => {
                        let b64 = base64::engine::general_purpose::STANDARD
                            .encode(&buf[..n]);
                        let env = Envelope::terminal_data(
                            &instance_id,
                            &terminal_id_for_task,
                            &b64,
                        );

                        // Snapshot senders for this connection_owner and fan out.
                        let senders: Vec<OutboundControlTx> =
                            match OUTBOUND_CONTROL_SENDERS.read() {
                                Ok(guard) => guard
                                    .get(&connection_owner)
                                    .cloned()
                                    .unwrap_or_default(),
                                Err(_) => break,
                            };
                        if senders.is_empty() {
                            // Connection owner gone — terminal output is
                            // orphaned, drop the loop so the PTY can be
                            // collected on shutdown.
                            break;
                        }
                        for tx in senders {
                            let _ = tx.send(env.clone());
                        }
                    }
                    Err(_) => break,
                }
            }
            tracing::debug!(
                "PTY reader for {} exiting",
                terminal_id_for_task
            );
        })
        .await;
    });
}

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
        // AMGR-017: instrumented hot-loop marker — sub-1ns cost when no profile session is active.
        // This wraps the reconnect loop; a high call_count during a profile window indicates the
        // bridge is flapping reconnect attempts.
        crate::profile_scope!("relay_loop::reconnect_iter");
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

    // SESS-015: Register subordinate chunk channel for this relay connection
    let mut subordinate_rx = register_subordinate_chunk_channel(session_id);

    // Spawn inbound handler
    let inbound_url = url.to_string();
    let inbound_shutdown_tx = shutdown_tx.clone();
    let inbound_control_handler = control_handler.clone();
    let inbound_command_emitter = command_emitter.clone();
    let inbound_pending_commands = pending_commands.clone();
    // SESS-017: Inbound responses (e.g. session:created, terminal:created,
    // pong) are forwarded to the outbound write loop via control_tx.
    let inbound_response_tx = {
        let guard = OUTBOUND_CONTROL_SENDERS.read().ok();
        guard
            .as_ref()
            .and_then(|g| g.get(&session_id))
            .and_then(|v| v.last().cloned())
    };
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
                            // SESS-017: Forward the response envelope to the
                            // outbound writer via the control channel.
                            if let Some(tx) = &inbound_response_tx {
                                if let Err(e) = tx.send(response_env) {
                                    tracing::warn!(
                                        "Failed to forward inbound response: {}",
                                        e
                                    );
                                }
                            } else {
                                tracing::warn!(
                                    "Inbound produced response but no outbound channel available"
                                );
                            }
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
                            remove_subordinate_chunk_senders(session_id);
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
            // SESS-015: Subordinate chunk forwarding — chunks from subordinate
            // sessions arrive here tagged with _relay_session_id
            Some((sub_session_id, chunk_json)) = subordinate_rx.recv() => {
                let sub_session_id_str = sub_session_id.to_string();
                let action = process_outbound_envelope(
                    &chunk_json,
                    &instance_id,
                    &sub_session_id_str,
                    None, // Subordinate chunks don't participate in fspec command protocol
                );

                let envelope = match action {
                    OutboundEnvelopeAction::RelayChunk(env) => env,
                    OutboundEnvelopeAction::Skip => continue,
                    OutboundEnvelopeAction::CommandResponse(_) => continue,
                };

                let msg_json = match serde_json::to_string(&envelope) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::warn!("Failed to serialize subordinate envelope: {}", e);
                        continue;
                    }
                };

                if let Err(e) = ws_write.send(Message::Text(msg_json.into())).await {
                    tracing::warn!("Failed to send subordinate chunk to WebSocket: {}", e);
                    remove_outbound_controls(session_id);
                    remove_subordinate_chunk_senders(session_id);
                    return Err(format!("Send failed: {e}"));
                }
            }
        }
    }

    remove_outbound_controls(session_id);
    remove_subordinate_chunk_senders(session_id);
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
    use serial_test::serial;

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

    // =========================================================================
    // Feature: spec/features/session-tab-creation-bridge-handlers.feature
    //
    // Scenario: Bridge handles session:create envelope and responds with session:created
    // =========================================================================

    /// @step Given the fspec bridge has a registered SessionCreator callback
    /// @step And a session:create envelope arrives with request_id "req-1" and instance_id "proj"
    /// @step When the bridge routes the inbound envelope
    /// @step Then the route should produce a SessionCreate action with request_id "req-1"
    /// @step And the SessionCreator callback should be invoked
    /// @step And the bridge should send back a session:created envelope on the outbound channel
    /// @step And the response envelope should contain the new session_id
    /// @step And the response envelope should carry request_id "req-1"
    #[tokio::test]
    async fn test_handle_inbound_session_create_invokes_creator_and_responds() {
        // @step Given the fspec bridge has a registered SessionCreator callback
        let invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let creator: SessionCreator = Arc::new(move || {
            invoked_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok("sess-new-123".to_string())
        });
        set_session_creator(Some(creator));

        // @step And a session:create envelope arrives with request_id "req-1" and instance_id "proj"
        let text = r#"{
            "service": "session",
            "type": "create",
            "instance_id": "proj",
            "request_id": "req-1"
        }"#;

        let injector: InputInjector = Arc::new(|_| {});

        // @step When the bridge routes the inbound envelope
        let result = handle_multiplexed_inbound(
            text,
            Uuid::new_v4(),
            injector,
            None,
            None,
            None,
        )
        .await;

        // @step And the SessionCreator callback should be invoked
        assert!(
            invoked.load(std::sync::atomic::Ordering::SeqCst),
            "SessionCreator should have been invoked"
        );

        // @step And the bridge should send back a session:created envelope on the outbound channel
        // @step And the response envelope should contain the new session_id
        // @step And the response envelope should carry request_id "req-1"
        match result {
            Ok(Some(env)) => {
                assert_eq!(env.service, Service::Session);
                assert_eq!(env.msg_type, "created");
                assert_eq!(env.request_id.as_deref(), Some("req-1"));
                let data = env.data.as_ref().expect("data");
                assert_eq!(data["session_id"], "sess-new-123");
            }
            other => panic!("Expected session:created response, got {other:?}"),
        }

        // Cleanup global state
        set_session_creator(None);
    }

    // =========================================================================
    // Scenario: Bridge handles terminal:create envelope and spawns PTY
    // =========================================================================

    /// @step Given the fspec bridge has a registered PtyRegistry
    /// @step And a terminal:create envelope arrives with request_id "req-2", cols 80, rows 24
    /// @step When the bridge routes the inbound envelope
    /// @step Then a TerminalCreate action should be produced with request_id "req-2"
    /// @step And the bridge should spawn a PTY via the registry
    /// @step And the bridge should send back a terminal:created envelope on the outbound channel
    /// @step And the response envelope should contain the spawned terminal_id
    /// @step And the response envelope should carry request_id "req-2"
    #[tokio::test]
    #[serial]
    async fn test_handle_inbound_terminal_create_spawns_pty_and_responds() {
        // @step Given the fspec bridge has a registered PtyRegistry
        let registry = Arc::new(crate::PtyRegistry::new());
        set_pty_registry(Some(registry.clone()));

        // @step And a terminal:create envelope arrives with request_id "req-2", cols 80, rows 24
        let text = r#"{
            "service": "terminal",
            "type": "create",
            "instance_id": "proj",
            "request_id": "req-2",
            "data": {"cols": 80, "rows": 24}
        }"#;

        let injector: InputInjector = Arc::new(|_| {});

        // @step When the bridge routes the inbound envelope
        let result = handle_multiplexed_inbound(
            text,
            Uuid::new_v4(),
            injector,
            None,
            None,
            None,
        )
        .await;

        // @step And the bridge should spawn a PTY via the registry
        assert!(!registry.is_empty(), "PtyRegistry should contain spawned PTY");

        // @step And the bridge should send back a terminal:created envelope on the outbound channel
        // @step And the response envelope should contain the spawned terminal_id
        // @step And the response envelope should carry request_id "req-2"
        match result {
            Ok(Some(env)) => {
                assert_eq!(env.service, Service::Terminal);
                assert_eq!(env.msg_type, "created");
                assert_eq!(env.request_id.as_deref(), Some("req-2"));
                let term_id = env.terminal_id.as_deref().expect("terminal_id");
                assert!(!term_id.is_empty(), "terminal_id must be non-empty");
                assert!(
                    registry.get(term_id).is_some(),
                    "registry must contain the new terminal"
                );
            }
            other => panic!("Expected terminal:created response, got {other:?}"),
        }

        // Cleanup
        registry.shutdown_all().await;
        set_pty_registry(None);
    }

    // =========================================================================
    // Feature: spec/features/bridge-multi-session-routing-and-terminal-io.feature
    //
    // SESS-018: Multi-session routing + terminal I/O wiring
    // =========================================================================

    /// Tests for SESS-018 must verify that session:input is routed to the
    /// correct injector based on the envelope's session_id, not the closure
    /// parameter. These tests are the red phase — they fail against the
    /// current implementation which discards session_id and always uses the
    /// fallback closure.
    /// @step Given the dashboard is connected to the fspec instance via the relay
    /// @step And the user has an existing fspec session tab #1
    /// @step When the user clicks the + dropdown and selects "New fspec Session"
    /// @step Then the bridge should spawn a new codelet session with a unique session_id
    /// @step And the bridge should register a BridgeSessionContext for the new session_id
    /// @step And the bridge should send a session:created response carrying the new session_id
    /// @step And a metadataUpdate envelope should list both sessions
    /// @step And a new tab #2 should appear in the dashboard session tab bar
    /// @step When the user activates tab #2 and sends the message "what is 4 + 3?"
    /// @step Then the bridge should route the session:input envelope to the new session's input_injector
    /// @step And the new session's agent_loop should receive the message
    /// @step And the response chunks should stream only into tab #2
    /// @step And tab #1 should remain unchanged
    #[tokio::test]
    async fn test_sess018_session_input_routes_by_envelope_session_id() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Given two registered injectors for two different session ids
        let supervisor_id = Uuid::new_v4();
        let new_session_id = Uuid::new_v4();

        let supervisor_hits = Arc::new(AtomicUsize::new(0));
        let new_hits = Arc::new(AtomicUsize::new(0));

        // Supervisor injector (fallback passed to handle_multiplexed_inbound)
        let supervisor_hits_clone = supervisor_hits.clone();
        let supervisor_injector: InputInjector = Arc::new(move |_input: InjectedInput| {
            supervisor_hits_clone.fetch_add(1, Ordering::SeqCst);
        });

        // New session injector (registered via BRIDGE_SESSION_CONTEXTS)
        let new_hits_clone = new_hits.clone();
        let new_injector: InputInjector = Arc::new(move |_input: InjectedInput| {
            new_hits_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Register a BridgeSessionContext for the new session
        // A dummy broadcast_rx_factory is sufficient for routing tests
        let (_dummy_tx, _dummy_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(1);
        let dummy_tx_for_factory = _dummy_tx.clone();
        let broadcast_rx_factory: crate::BroadcastReceiverFactory =
            Arc::new(move || dummy_tx_for_factory.subscribe());
        crate::set_bridge_session_context(
            new_session_id,
            broadcast_rx_factory,
            new_injector,
            None,
            None,
        );

        // When a session:input envelope targets the NEW session
        let text = format!(
            r#"{{"service":"session","type":"input","session_id":"{new_session_id}","data":{{"message":"what is 4 + 3?"}}}}"#,
        );
        let result = handle_multiplexed_inbound(
            &text,
            supervisor_id,
            supervisor_injector,
            None,
            None,
            None,
        )
        .await;

        assert!(result.is_ok(), "handle_multiplexed_inbound must succeed");

        // Then the new session's injector must be called — NOT the supervisor's
        assert_eq!(
            new_hits.load(Ordering::SeqCst),
            1,
            "new session injector should receive the input"
        );
        assert_eq!(
            supervisor_hits.load(Ordering::SeqCst),
            0,
            "supervisor injector must NOT receive input targeted at another session"
        );

        crate::remove_bridge_session_context(new_session_id);
    }

    /// @step Given the dashboard has the original supervisor tab #1 and a newly-created tab #2
    /// @step When the user sends "are you there?" in tab #1
    /// @step Then the bridge should route the session:input envelope to the supervisor's input_injector
    /// @step And the supervisor session should receive the message and respond
    /// @step And the response should stream only into tab #1
    /// @step And tab #2 should remain untouched
    #[tokio::test]
    async fn test_sess018_session_input_without_context_falls_back_to_parameter_injector() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let supervisor_id = Uuid::new_v4();
        let supervisor_hits = Arc::new(AtomicUsize::new(0));
        let supervisor_hits_clone = supervisor_hits.clone();
        let supervisor_injector: InputInjector = Arc::new(move |_input: InjectedInput| {
            supervisor_hits_clone.fetch_add(1, Ordering::SeqCst);
        });

        // No BridgeSessionContext registered for this session_id
        let unregistered_sid = Uuid::new_v4();
        let text = format!(
            r#"{{"service":"session","type":"input","session_id":"{unregistered_sid}","data":{{"message":"hi"}}}}"#,
        );
        let result = handle_multiplexed_inbound(
            &text,
            supervisor_id,
            supervisor_injector,
            None,
            None,
            None,
        )
        .await;
        assert!(result.is_ok());

        // Fallback to the parameter injector
        assert_eq!(
            supervisor_hits.load(Ordering::SeqCst),
            1,
            "fallback injector must be invoked when no per-session context exists"
        );
    }

    /// @step Given the dashboard has two active fspec session tabs #1 and #2
    /// @step And tab #2's agent is currently generating a response
    /// @step When the user clicks the Interrupt button on tab #2
    /// @step Then the bridge should receive a session:control envelope with session_id of tab #2
    /// @step And the bridge should look up the BridgeSessionContext for tab #2's session_id
    /// @step And the bridge should invoke that context's control_handler with action "interrupt"
    /// @step And only tab #2's session should stop generating
    /// @step And tab #1 should continue its response uninterrupted
    #[tokio::test]
    async fn test_sess018_session_control_routes_by_envelope_session_id() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let supervisor_id = Uuid::new_v4();
        let new_session_id = Uuid::new_v4();

        let supervisor_hits = Arc::new(AtomicUsize::new(0));
        let new_hits = Arc::new(AtomicUsize::new(0));

        let supervisor_hits_clone = supervisor_hits.clone();
        let supervisor_handler: ControlHandler =
            Arc::new(move |_action: &str, _response: Option<&str>| {
                supervisor_hits_clone.fetch_add(1, Ordering::SeqCst);
            });

        let new_hits_clone = new_hits.clone();
        let new_handler: ControlHandler =
            Arc::new(move |_action: &str, _response: Option<&str>| {
                new_hits_clone.fetch_add(1, Ordering::SeqCst);
            });

        // Register context for new session with its own control handler
        let (_dummy_tx, _dummy_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(1);
        let dummy_tx_for_factory = _dummy_tx.clone();
        let broadcast_rx_factory: crate::BroadcastReceiverFactory =
            Arc::new(move || dummy_tx_for_factory.subscribe());
        let injector: InputInjector = Arc::new(|_| {});
        crate::set_bridge_session_context(
            new_session_id,
            broadcast_rx_factory,
            injector,
            Some(new_handler),
            None,
        );

        let text = format!(
            r#"{{"service":"session","type":"control","session_id":"{new_session_id}","data":{{"action":"interrupt"}}}}"#,
        );
        let fallback_injector: InputInjector = Arc::new(|_| {});
        let result = handle_multiplexed_inbound(
            &text,
            supervisor_id,
            fallback_injector,
            Some(supervisor_handler),
            None,
            None,
        )
        .await;
        assert!(result.is_ok());

        assert_eq!(
            new_hits.load(Ordering::SeqCst),
            1,
            "new session control handler should receive the interrupt"
        );
        assert_eq!(
            supervisor_hits.load(Ordering::SeqCst),
            0,
            "supervisor control handler must NOT be invoked for another session's interrupt"
        );

        crate::remove_bridge_session_context(new_session_id);
    }

    /// @step Given a Terminal tab is open with a live shell prompt
    /// @step When the user types "ls" and presses Enter
    /// @step Then the dashboard should send a terminal:input envelope with base64-encoded "ls\n"
    /// @step And the bridge should decode the base64 payload
    /// @step And the bridge should invoke PtyRegistry::write_terminal_input with the decoded bytes
    /// @step And the shell should execute ls and print the directory listing
    /// @step And the directory listing should stream back as terminal:data envelopes
    /// @step And the output should render inside the Terminal tab
    #[tokio::test]
    #[serial]
    async fn test_sess018_terminal_input_writes_to_pty() {
        use base64::Engine;
        use std::io::Read;
        use std::time::Duration;

        // Given a registered PtyRegistry with one active terminal
        let registry = Arc::new(crate::PtyRegistry::new());
        set_pty_registry(Some(registry.clone()));

        let (term_id, entry) = crate::create_terminal(
            &registry,
            &crate::CreateTerminalOpts {
                cols: 80,
                rows: 24,
                shell: None,
                cwd: Some("/tmp".to_string()),
            },
        )
        .expect("create terminal");

        // Clone the reader so we can verify the written bytes hit the PTY
        // (the shell echoes typed characters back on its output stream).
        let reader = {
            let master = entry.master.lock().await;
            master.try_clone_reader().expect("clone reader")
        };

        // When a terminal:input envelope arrives with a sentinel string
        let sentinel = "echo SESS018_SENTINEL_OUTPUT\n";
        let b64 = base64::engine::general_purpose::STANDARD.encode(sentinel);
        let text = format!(
            r#"{{"service":"terminal","type":"input","terminal_id":"{term_id}","data":{{"base64":"{b64}"}}}}"#,
        );

        let injector: InputInjector = Arc::new(|_| {});
        let result = handle_multiplexed_inbound(
            &text,
            Uuid::new_v4(),
            injector,
            None,
            None,
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "terminal:input must dispatch cleanly, got: {result:?}",
        );

        // Then — and this is the critical assertion — the sentinel bytes
        // must actually have been written to the PTY. We verify this by
        // reading from the cloned reader: a working shell echoes typed
        // characters and prints the result of `echo`. If the handler is
        // still the unwired `Ok(None)` stub, nothing is written and the
        // reader will block until the timeout.
        let saw_sentinel = tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut accumulated = String::new();
            // Poll-read for up to ~2 seconds
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            let mut buf = [0u8; 4096];
            while std::time::Instant::now() < deadline {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        accumulated.push_str(&String::from_utf8_lossy(&buf[..n]));
                        if accumulated.contains("SESS018_SENTINEL_OUTPUT") {
                            return true;
                        }
                    }
                    Err(_) => break,
                }
            }
            accumulated.contains("SESS018_SENTINEL_OUTPUT")
        })
        .await
        .unwrap_or(false);

        assert!(
            saw_sentinel,
            "expected PTY output to contain the sentinel — this proves terminal:input was actually written to the shell's stdin"
        );

        registry.shutdown_all().await;
        set_pty_registry(None);
    }

    /// @step Given a Terminal tab is open with cols=80 rows=24
    /// @step When the user resizes the browser pane so xterm.js fits to cols=120 rows=40
    /// @step Then the dashboard should send a terminal:resize envelope with cols=120 rows=40
    /// @step And the bridge should invoke PtyRegistry::resize_terminal with the new dimensions
    /// @step And the shell should be notified via SIGWINCH
    /// @step And running "stty size" inside the shell should report "40 120"
    #[tokio::test]
    #[serial]
    async fn test_sess018_terminal_resize_updates_pty_size() {
        let registry = Arc::new(crate::PtyRegistry::new());
        set_pty_registry(Some(registry.clone()));

        let (term_id, entry) = crate::create_terminal(
            &registry,
            &crate::CreateTerminalOpts {
                cols: 80,
                rows: 24,
                shell: None,
                cwd: Some("/tmp".to_string()),
            },
        )
        .expect("create terminal");

        let text = format!(
            r#"{{"service":"terminal","type":"resize","terminal_id":"{term_id}","data":{{"cols":120,"rows":40}}}}"#,
        );
        let injector: InputInjector = Arc::new(|_| {});
        let result = handle_multiplexed_inbound(
            &text,
            Uuid::new_v4(),
            injector,
            None,
            None,
            None,
        )
        .await;
        assert!(result.is_ok(), "terminal:resize must succeed");

        let size = entry.size.lock().await;
        assert_eq!(size.cols, 120, "PTY cols should be updated to 120");
        assert_eq!(size.rows, 40, "PTY rows should be updated to 40");
        drop(size);

        registry.shutdown_all().await;
        set_pty_registry(None);
    }

    /// @step Given a Terminal tab is open with a running shell
    /// @step When the user clicks the X button on the Terminal tab
    /// @step Then the dashboard should send a terminal:destroy envelope with a request_id
    /// @step And the bridge should invoke PtyRegistry::destroy_terminal
    /// @step And the shell process should be killed
    /// @step And the terminal should be removed from the PtyRegistry
    /// @step And the bridge should reply with a terminal:destroyed envelope carrying the same request_id
    #[tokio::test]
    #[serial]
    async fn test_sess018_terminal_destroy_removes_and_responds() {
        let registry = Arc::new(crate::PtyRegistry::new());
        set_pty_registry(Some(registry.clone()));

        let (term_id, _entry) = crate::create_terminal(
            &registry,
            &crate::CreateTerminalOpts {
                cols: 80,
                rows: 24,
                shell: None,
                cwd: Some("/tmp".to_string()),
            },
        )
        .expect("create terminal");

        assert_eq!(registry.len(), 1);

        let text = format!(
            r#"{{"service":"terminal","type":"destroy","terminal_id":"{term_id}","request_id":"destroy-1"}}"#,
        );
        let injector: InputInjector = Arc::new(|_| {});
        let result = handle_multiplexed_inbound(
            &text,
            Uuid::new_v4(),
            injector,
            None,
            None,
            None,
        )
        .await;

        match result {
            Ok(Some(env)) => {
                assert_eq!(env.service, Service::Terminal);
                assert_eq!(env.msg_type, "destroyed");
                assert_eq!(env.request_id.as_deref(), Some("destroy-1"));
                assert_eq!(env.terminal_id.as_deref(), Some(term_id.as_str()));
            }
            other => panic!("Expected terminal:destroyed response, got {other:?}"),
        }

        assert_eq!(registry.len(), 0, "PTY should have been removed");
        set_pty_registry(None);
    }

    /// @step Given the dashboard is connected to the fspec instance via the relay
    /// @step When the user clicks the + dropdown and selects "New Terminal"
    /// @step Then the bridge should spawn a new PTY via PtyRegistry::create_terminal
    /// @step And the bridge should send a terminal:created response carrying the new terminal_id
    /// @step And the bridge should spawn a background reader task on the PTY master
    /// @step And the background reader task should forward each chunk of PTY output as a terminal:data envelope
    /// @step And the dashboard should receive the terminal:data envelopes and write them to xterm.js
    /// @step And a live shell prompt should render inside the new Terminal tab
    #[tokio::test]
    #[serial]
    async fn test_sess018_terminal_create_spawns_reader_emitting_data_envelopes() {
        use std::time::Duration;
        use tokio::sync::mpsc;

        // Given a registered OutboundControlTx that the reader task will drain
        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<Envelope>();
        // SESS-018: register the outbound tx under a synthesized connection owner
        // so the handler can look it up via OUTBOUND_CONTROL_SENDERS (same plumbing
        // real connections use at connect_and_relay:792).
        let connection_owner = Uuid::new_v4();
        register_outbound_control(connection_owner, outbound_tx);

        let registry = Arc::new(crate::PtyRegistry::new());
        set_pty_registry(Some(registry.clone()));

        // When a terminal:create envelope arrives
        let text = r#"{
            "service": "terminal",
            "type": "create",
            "instance_id": "proj",
            "request_id": "create-1",
            "data": {"cols": 80, "rows": 24}
        }"#;
        let injector: InputInjector = Arc::new(|_| {});
        let result = handle_multiplexed_inbound(
            text,
            connection_owner,
            injector,
            None,
            None,
            None,
        )
        .await;

        match result {
            Ok(Some(env)) => assert_eq!(env.msg_type, "created"),
            other => panic!("Expected terminal:created, got {other:?}"),
        }

        // Then the reader task must push at least one terminal:data envelope
        // Most shells emit a prompt (e.g. "$ ") on startup — that's our signal.
        let mut saw_data = false;
        for _ in 0..20 {
            match tokio::time::timeout(Duration::from_millis(250), outbound_rx.recv()).await {
                Ok(Some(env)) => {
                    if env.service == Service::Terminal && env.msg_type == "data" {
                        saw_data = true;
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }
        assert!(
            saw_data,
            "expected at least one terminal:data envelope from the spawned PTY reader"
        );

        registry.shutdown_all().await;
        set_pty_registry(None);
        remove_outbound_controls(connection_owner);
    }
}
