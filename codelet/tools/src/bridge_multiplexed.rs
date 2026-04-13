//! Multiplexed WebSocket Protocol Types for bridge_relay.rs
//!
//! Defines the envelope format used by the fspec-pro relay gateway.
//! When connecting to a fspec-pro endpoint (detected by URL), bridge_relay
//! wraps all messages in the multiplexed envelope instead of the flat format.
//!
//! Feature: spec/features/bridge-multiplexed-protocol.feature

use serde::{Deserialize, Serialize};

// ── Service enum ────────────────────────────────────────────────────────────

/// The 6 service types in the multiplexed protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    Auth,
    Relay,
    Fspec,
    Session,
    Terminal,
    System,
}

// ── Envelope ────────────────────────────────────────────────────────────────

/// A multiplexed protocol envelope.
///
/// Every WebSocket text frame follows this structure:
/// `{service, type, instance_id?, session_id?, terminal_id?, request_id?, data?}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub service: Service,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ── Instance metadata ───────────────────────────────────────────────────────

/// Metadata reported by the agent during auth handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<serde_json::Value>,
}

// ── Builder helpers ─────────────────────────────────────────────────────────

impl Envelope {
    /// Build an auth envelope for agent authentication.
    pub fn auth_agent(api_key: &str, instance: &InstanceMetadata) -> Self {
        Self {
            service: Service::Auth,
            msg_type: "authenticate".to_string(),
            instance_id: None,
            session_id: None,
            terminal_id: None,
            request_id: None,
            data: Some(serde_json::json!({
                "role": "agent",
                "api_key": api_key,
                "instance": instance,
            })),
        }
    }

    /// Build a relay chunk envelope (outbound session stream).
    pub fn relay_chunk(
        instance_id: &str,
        session_id: &str,
        chunk: serde_json::Value,
    ) -> Self {
        Self {
            service: Service::Relay,
            msg_type: "chunk".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: Some(session_id.to_string()),
            terminal_id: None,
            request_id: None,
            data: Some(chunk),
        }
    }

    /// Build an fspec commandResponse envelope.
    pub fn fspec_command_response(
        instance_id: &str,
        request_id: &str,
        command: &str,
        success: bool,
        result: serde_json::Value,
        error: Option<&str>,
    ) -> Self {
        let mut resp = serde_json::json!({
            "command": command,
            "success": success,
            "result": result,
        });
        if let Some(err) = error {
            resp["error"] = serde_json::Value::String(err.to_string());
        }
        Self {
            service: Service::Fspec,
            msg_type: "commandResponse".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: None,
            request_id: Some(request_id.to_string()),
            data: Some(resp),
        }
    }

    /// Build a terminal data envelope (PTY stdout, base64-encoded).
    pub fn terminal_data(
        instance_id: &str,
        terminal_id: &str,
        base64_data: &str,
    ) -> Self {
        Self {
            service: Service::Terminal,
            msg_type: "data".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: Some(terminal_id.to_string()),
            request_id: None,
            data: Some(serde_json::json!({ "base64": base64_data })),
        }
    }

    /// Build a terminal created response.
    pub fn terminal_created(
        instance_id: &str,
        request_id: &str,
        terminal_id: &str,
    ) -> Self {
        Self {
            service: Service::Terminal,
            msg_type: "created".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: Some(terminal_id.to_string()),
            request_id: Some(request_id.to_string()),
            data: Some(serde_json::json!({ "terminal_id": terminal_id })),
        }
    }

    /// Build a terminal destroyed response.
    pub fn terminal_destroyed(
        instance_id: &str,
        request_id: &str,
        terminal_id: &str,
    ) -> Self {
        Self {
            service: Service::Terminal,
            msg_type: "destroyed".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: Some(terminal_id.to_string()),
            request_id: Some(request_id.to_string()),
            data: None,
        }
    }

    /// Build a terminal exited notification.
    pub fn terminal_exited(
        instance_id: &str,
        terminal_id: &str,
        exit_code: i32,
    ) -> Self {
        Self {
            service: Service::Terminal,
            msg_type: "exited".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: Some(terminal_id.to_string()),
            request_id: None,
            data: Some(serde_json::json!({ "exit_code": exit_code })),
        }
    }

    /// Build a system pong response.
    pub fn system_pong() -> Self {
        Self {
            service: Service::System,
            msg_type: "pong".to_string(),
            instance_id: None,
            session_id: None,
            terminal_id: None,
            request_id: None,
            data: None,
        }
    }

    /// Build a relay metadataUpdate envelope for live session/model changes.
    pub fn relay_metadata_update(
        instance_id: &str,
        data: serde_json::Value,
    ) -> Self {
        Self {
            service: Service::Relay,
            msg_type: "metadataUpdate".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: None,
            terminal_id: None,
            request_id: None,
            data: Some(data),
        }
    }

    /// Build a session:created response envelope.
    ///
    /// SESS-017: Sent in response to a session:create request after the
    /// SessionCreator callback has spawned a new codelet session.
    pub fn session_created(
        instance_id: &str,
        request_id: &str,
        session_id: &str,
    ) -> Self {
        Self {
            service: Service::Session,
            msg_type: "created".to_string(),
            instance_id: Some(instance_id.to_string()),
            session_id: Some(session_id.to_string()),
            terminal_id: None,
            request_id: Some(request_id.to_string()),
            data: Some(serde_json::json!({ "session_id": session_id })),
        }
    }
}

/// Detect whether a URL targets a fspec-pro multiplexed endpoint.
///
/// Returns true if the URL path ends with `/ws/relay` or the URL
/// contains a `protocol=multiplexed` query parameter.
pub fn is_multiplexed_endpoint(url: &str) -> bool {
    // Check path suffix
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.path().ends_with("/ws/relay") {
            return true;
        }
        // Check query parameter
        for (key, value) in parsed.query_pairs() {
            if key == "protocol" && value == "multiplexed" {
                return true;
            }
        }
    }
    false
}

// ── Inbound message routing ─────────────────────────────────────────────────

/// Result of routing an inbound multiplexed envelope.
#[derive(Debug)]
pub enum InboundAction {
    /// Session input: inject message + optional images.
    SessionInput {
        session_id: String,
        message: String,
        images: Option<Vec<crate::bridge_relay::ImageData>>,
    },
    /// Session control: interrupt, clear, pause_response.
    SessionControl {
        session_id: String,
        action: String,
        response: Option<String>,
    },
    /// Session create: spawn a new codelet session and respond with session:created.
    /// SESS-017: Dashboard "+ > New fspec Session" sends this; the bridge must
    /// invoke the registered SessionCreator and emit a `session:created` response.
    SessionCreate {
        request_id: String,
    },
    /// fspec command execution request.
    FspecCommand {
        request_id: String,
        command: String,
        args_json: String,
    },
    /// Terminal create request.
    TerminalCreate {
        request_id: String,
        cols: u16,
        rows: u16,
        shell: Option<String>,
        cwd: Option<String>,
    },
    /// Terminal input (stdin).
    TerminalInput {
        terminal_id: String,
        base64_data: String,
    },
    /// Terminal resize.
    TerminalResize {
        terminal_id: String,
        cols: u16,
        rows: u16,
    },
    /// Terminal destroy.
    TerminalDestroy {
        terminal_id: String,
        request_id: String,
    },
    /// System ping → respond with pong.
    SystemPing,
    /// Auth response from server (success or error).
    AuthResponse {
        success: bool,
        data: Option<serde_json::Value>,
    },
    /// Unknown or unhandled message — log and skip.
    Unknown {
        service: String,
        msg_type: String,
    },
}

/// Route an inbound multiplexed envelope to the appropriate action.
pub fn route_inbound(envelope: &Envelope) -> InboundAction {
    match &envelope.service {
        Service::Auth => {
            let success = envelope.msg_type == "authSuccess";
            InboundAction::AuthResponse {
                success,
                data: envelope.data.clone(),
            }
        }
        Service::Session => match envelope.msg_type.as_str() {
            "input" => {
                let session_id = envelope
                    .session_id
                    .clone()
                    .unwrap_or_default();
                let data = envelope.data.as_ref();
                let message = data
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let images = data
                    .and_then(|d| d.get("images"))
                    .and_then(|imgs| {
                        serde_json::from_value::<Vec<crate::bridge_relay::ImageData>>(
                            imgs.clone(),
                        )
                        .ok()
                    });
                InboundAction::SessionInput {
                    session_id,
                    message,
                    images,
                }
            }
            "control" => {
                let session_id = envelope
                    .session_id
                    .clone()
                    .unwrap_or_default();
                let data = envelope.data.as_ref();
                let action = data
                    .and_then(|d| d.get("action"))
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let response = data
                    .and_then(|d| d.get("response"))
                    .and_then(|r| r.as_str())
                    .map(std::string::ToString::to_string);
                InboundAction::SessionControl {
                    session_id,
                    action,
                    response,
                }
            }
            "create" => {
                // SESS-017: Route session:create to a SessionCreate action
                // so the bridge can spawn a new codelet session via the
                // registered SessionCreator and respond with session:created.
                let request_id = envelope
                    .request_id
                    .clone()
                    .unwrap_or_default();
                InboundAction::SessionCreate { request_id }
            }
            other => InboundAction::Unknown {
                service: "session".to_string(),
                msg_type: other.to_string(),
            },
        },
        Service::Fspec => {
            if envelope.msg_type == "command" {
                let data = envelope.data.as_ref();
                let request_id = envelope
                    .request_id
                    .clone()
                    .unwrap_or_default();
                let command = data
                    .and_then(|d| d.get("command"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_json = data
                    .and_then(|d| d.get("args"))
                    .map(std::string::ToString::to_string)
                    .unwrap_or_else(|| "{}".to_string());
                InboundAction::FspecCommand {
                    request_id,
                    command,
                    args_json,
                }
            } else {
                InboundAction::Unknown {
                    service: "fspec".to_string(),
                    msg_type: envelope.msg_type.clone(),
                }
            }
        }
        Service::Terminal => match envelope.msg_type.as_str() {
            "create" => {
                let data = envelope.data.as_ref();
                let request_id = envelope
                    .request_id
                    .clone()
                    .unwrap_or_default();
                let cols = data
                    .and_then(|d| d.get("cols"))
                    .and_then(crate::facade::param_extract::value_as_u64_lenient)
                    .unwrap_or(80) as u16;
                let rows = data
                    .and_then(|d| d.get("rows"))
                    .and_then(crate::facade::param_extract::value_as_u64_lenient)
                    .unwrap_or(24) as u16;
                let shell = data
                    .and_then(|d| d.get("shell"))
                    .and_then(|s| s.as_str())
                    .map(std::string::ToString::to_string);
                let cwd = data
                    .and_then(|d| d.get("cwd"))
                    .and_then(|c| c.as_str())
                    .map(std::string::ToString::to_string);
                InboundAction::TerminalCreate {
                    request_id,
                    cols,
                    rows,
                    shell,
                    cwd,
                }
            }
            "input" => {
                let terminal_id = envelope
                    .terminal_id
                    .clone()
                    .unwrap_or_default();
                let base64_data = envelope
                    .data
                    .as_ref()
                    .and_then(|d| d.get("base64"))
                    .and_then(|b| b.as_str())
                    .unwrap_or("")
                    .to_string();
                InboundAction::TerminalInput {
                    terminal_id,
                    base64_data,
                }
            }
            "resize" => {
                let terminal_id = envelope
                    .terminal_id
                    .clone()
                    .unwrap_or_default();
                let data = envelope.data.as_ref();
                let cols = data
                    .and_then(|d| d.get("cols"))
                    .and_then(crate::facade::param_extract::value_as_u64_lenient)
                    .unwrap_or(80) as u16;
                let rows = data
                    .and_then(|d| d.get("rows"))
                    .and_then(crate::facade::param_extract::value_as_u64_lenient)
                    .unwrap_or(24) as u16;
                InboundAction::TerminalResize {
                    terminal_id,
                    cols,
                    rows,
                }
            }
            "destroy" => {
                let terminal_id = envelope
                    .terminal_id
                    .clone()
                    .unwrap_or_default();
                let request_id = envelope
                    .request_id
                    .clone()
                    .unwrap_or_default();
                InboundAction::TerminalDestroy {
                    terminal_id,
                    request_id,
                }
            }
            other => InboundAction::Unknown {
                service: "terminal".to_string(),
                msg_type: other.to_string(),
            },
        },
        Service::System => {
            if envelope.msg_type == "ping" {
                InboundAction::SystemPing
            } else {
                InboundAction::Unknown {
                    service: "system".to_string(),
                    msg_type: envelope.msg_type.clone(),
                }
            }
        }
        Service::Relay => InboundAction::Unknown {
            service: "relay".to_string(),
            msg_type: envelope.msg_type.clone(),
        },
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // =========================================================================
    // Feature: spec/features/bridge-multiplexed-protocol.feature
    //
    // Scenario: Authenticate with relay server using multiplexed protocol
    // =========================================================================

    /// @step Given the bridge relay is configured with a valid api_key and instance metadata
    /// @step When the bridge connects to a fspec-pro relay server WebSocket endpoint
    /// @step Then it should send an auth envelope with service "auth" and type "authenticate"
    #[test]
    fn test_auth_envelope_structure() {
        let metadata = InstanceMetadata {
            name: "my-project".to_string(),
            path: Some("/home/user/project".to_string()),
            version: Some("1.2.0".to_string()),
            os: Some("linux".to_string()),
            provider: Some("anthropic".to_string()),
            model: Some("claude-4-sonnet".to_string()),
            sessions: vec![],
        };

        let env = Envelope::auth_agent("test-api-key", &metadata);

        // @step Then it should send an auth envelope with service "auth" and type "authenticate"
        assert_eq!(env.service, Service::Auth);
        assert_eq!(env.msg_type, "authenticate");

        // @step And the auth data should contain role "agent" and the api_key
        let data = env.data.as_ref().unwrap();
        assert_eq!(data["role"], "agent");
        assert_eq!(data["api_key"], "test-api-key");

        // @step And the auth data should contain instance metadata with name, path, version, os, provider, and model
        let instance = &data["instance"];
        assert_eq!(instance["name"], "my-project");
        assert_eq!(instance["path"], "/home/user/project");
        assert_eq!(instance["version"], "1.2.0");
        assert_eq!(instance["os"], "linux");
        assert_eq!(instance["provider"], "anthropic");
        assert_eq!(instance["model"], "claude-4-sonnet");
    }

    /// @step And it should wait for an authSuccess response before sending any other messages
    #[test]
    fn test_auth_success_routing() {
        let json = r#"{"service":"auth","type":"authSuccess","data":{"instances":[]}}"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::AuthResponse { success, .. } => {
                assert!(success, "authSuccess should set success=true");
            }
            other => panic!("Expected AuthResponse, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Auth failure triggers reconnection with backoff
    // =========================================================================

    /// @step Given the bridge relay is configured with an invalid api_key
    /// @step When the bridge connects and sends the auth envelope
    /// @step Then it should receive an authError with code "AUTH_FAILED"
    /// @step And the connection should be closed
    /// @step And the bridge should reconnect with exponential backoff
    #[test]
    fn test_auth_error_routing() {
        let json = r#"{"service":"auth","type":"authError","data":{"code":"AUTH_FAILED","message":"Invalid API key"}}"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::AuthResponse { success, data } => {
                // @step Then it should receive an authError with code "AUTH_FAILED"
                assert!(!success, "authError should set success=false");
                let d = data.unwrap();
                assert_eq!(d["code"], "AUTH_FAILED");
                // @step And the connection should be closed
                // Connection close is handled by the relay loop based on success=false
                // @step And the bridge should reconnect with exponential backoff
                // Reconnection logic is handled by relay_loop — existing bridge_relay.rs behavior
            }
            other => panic!("Expected AuthResponse, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Forward session chunks as multiplexed relay envelopes
    // =========================================================================

    /// @step Given the bridge relay is authenticated with instance_id "my-project"
    /// @step And a session "abc-123" is producing stream chunks
    /// @step When a text chunk is received from the broadcast channel
    /// @step Then the bridge should send a multiplexed envelope with service "relay" and type "chunk"
    #[test]
    fn test_relay_chunk_envelope() {
        let chunk = serde_json::json!({"type": "text", "text": "Hello"});
        let env = Envelope::relay_chunk("my-project", "abc-123", chunk.clone());

        assert_eq!(env.service, Service::Relay);
        assert_eq!(env.msg_type, "chunk");

        // @step And the envelope should include instance_id "my-project" and session_id "abc-123"
        assert_eq!(env.instance_id.as_deref(), Some("my-project"));
        assert_eq!(env.session_id.as_deref(), Some("abc-123"));

        // @step And the data field should contain the original StreamChunk
        assert_eq!(env.data.unwrap(), chunk);
    }

    // =========================================================================
    // Scenario: Handle inbound session input with images
    // =========================================================================

    /// @step Given the bridge relay is authenticated and processing inbound messages
    /// @step When a message arrives with service "session", type "input", and session_id "abc-123"
    /// @step And the data contains message "Hello agent" and an images array
    /// @step Then the input_injector should be called with an InjectedInput containing the message and images
    #[test]
    fn test_route_session_input_with_images() {
        let json = r#"{
            "service": "session",
            "type": "input",
            "session_id": "abc-123",
            "data": {
                "message": "Hello agent",
                "images": [{"data": "base64...", "media_type": "image/png"}]
            }
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::SessionInput { session_id, message, images } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(message, "Hello agent");
                let imgs = images.unwrap();
                assert_eq!(imgs.len(), 1);
                assert_eq!(imgs[0].data, "base64...");
                assert_eq!(imgs[0].media_type, "image/png");
            }
            other => panic!("Expected SessionInput, got {other:?}"),
        }
    }

    /// @step Test session input without images (backward compatibility)
    #[test]
    fn test_route_session_input_without_images() {
        let json = r#"{
            "service": "session",
            "type": "input",
            "session_id": "abc-123",
            "data": {"message": "Just text"}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::SessionInput { message, images, .. } => {
                assert_eq!(message, "Just text");
                assert!(images.is_none());
            }
            other => panic!("Expected SessionInput, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Handle inbound session control action
    // =========================================================================

    /// @step Given the bridge relay is authenticated and processing inbound messages
    /// @step When a message arrives with service "session", type "control", and session_id "abc-123"
    /// @step And the data contains action "interrupt"
    /// @step Then the control_handler should be called with action "interrupt"
    #[test]
    fn test_route_session_control_interrupt() {
        let json = r#"{
            "service": "session",
            "type": "control",
            "session_id": "abc-123",
            "data": {"action": "interrupt"}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::SessionControl { session_id, action, response } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(action, "interrupt");
                assert!(response.is_none());
            }
            other => panic!("Expected SessionControl, got {other:?}"),
        }
    }

    /// @step Test pause_response control with response value
    #[test]
    fn test_route_session_control_pause_response() {
        let json = r#"{
            "service": "session",
            "type": "control",
            "session_id": "abc-123",
            "data": {"action": "pause_response", "response": "allow_once"}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::SessionControl { action, response, .. } => {
                assert_eq!(action, "pause_response");
                assert_eq!(response.as_deref(), Some("allow_once"));
            }
            other => panic!("Expected SessionControl, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Execute fspec command and return response
    // =========================================================================

    /// @step Given the bridge relay is authenticated with instance_id "my-project"
    /// @step And a command_emitter is configured
    /// @step When a message arrives with service "fspec", type "command", and request_id "r1"
    /// @step And the data contains command "board" and args "{}"
    /// @step Then the command_emitter should fire with the command and args
    /// @step And when the FspecCommandResult comes back on the broadcast channel
    /// @step Then the bridge should send a commandResponse envelope with service "fspec" and request_id "r1"
    /// @step And the response data should contain command "board", success, and the result
    #[test]
    fn test_route_fspec_command() {
        // @step When a message arrives with service "fspec", type "command", and request_id "r1"
        let json = r#"{
            "service": "fspec",
            "type": "command",
            "request_id": "r1",
            "data": {"command": "board", "args": {}}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        // @step Then the command_emitter should fire with the command and args
        match action {
            InboundAction::FspecCommand { request_id, command, args_json } => {
                assert_eq!(request_id, "r1");
                assert_eq!(command, "board");
                assert!(!args_json.is_empty());
            }
            other => panic!("Expected FspecCommand, got {other:?}"),
        }

        // @step And when the FspecCommandResult comes back on the broadcast channel
        // @step Then the bridge should send a commandResponse envelope with service "fspec" and request_id "r1"
        let result = serde_json::json!({"columns": {}});
        let response_env = Envelope::fspec_command_response(
            "my-project", "r1", "board", true, result, None,
        );
        assert_eq!(response_env.service, Service::Fspec);
        assert_eq!(response_env.msg_type, "commandResponse");
        assert_eq!(response_env.request_id.as_deref(), Some("r1"));

        // @step And the response data should contain command "board", success, and the result
        let data = response_env.data.unwrap();
        assert_eq!(data["command"], "board");
        assert_eq!(data["success"], true);
    }

    /// @step Test fspec commandResponse envelope builder
    #[test]
    fn test_fspec_command_response_envelope() {
        let result = serde_json::json!({"columns": {}});
        let env = Envelope::fspec_command_response(
            "my-project", "r1", "board", true, result, None,
        );

        assert_eq!(env.service, Service::Fspec);
        assert_eq!(env.msg_type, "commandResponse");
        assert_eq!(env.instance_id.as_deref(), Some("my-project"));
        assert_eq!(env.request_id.as_deref(), Some("r1"));
        let data = env.data.unwrap();
        assert_eq!(data["command"], "board");
        assert_eq!(data["success"], true);
    }

    /// @step Test fspec commandResponse with error
    #[test]
    fn test_fspec_command_response_with_error() {
        let env = Envelope::fspec_command_response(
            "proj", "r2", "bad-cmd", false,
            serde_json::Value::Null,
            Some("Command not found"),
        );

        let data = env.data.unwrap();
        assert_eq!(data["success"], false);
        assert_eq!(data["error"], "Command not found");
    }

    // =========================================================================
    // Scenario: Create terminal on agent via PTY
    // =========================================================================

    /// @step Given the bridge relay is authenticated with instance_id "my-project"
    /// @step And the PtyRegistry is initialized
    /// @step When a message arrives with service "terminal", type "create", and request_id "t1"
    /// @step And the data contains cols 80 and rows 24
    #[test]
    fn test_route_terminal_create() {
        let json = r#"{
            "service": "terminal",
            "type": "create",
            "request_id": "t1",
            "data": {"cols": 80, "rows": 24}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::TerminalCreate { request_id, cols, rows, shell, cwd } => {
                assert_eq!(request_id, "t1");
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
                assert!(shell.is_none());
                assert!(cwd.is_none());
            }
            other => panic!("Expected TerminalCreate, got {other:?}"),
        }
    }

    /// @step Test terminal create with custom shell and cwd
    #[test]
    fn test_route_terminal_create_with_options() {
        let json = r#"{
            "service": "terminal",
            "type": "create",
            "request_id": "t2",
            "data": {"cols": 120, "rows": 40, "shell": "/bin/bash", "cwd": "/tmp"}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::TerminalCreate { cols, rows, shell, cwd, .. } => {
                assert_eq!(cols, 120);
                assert_eq!(rows, 40);
                assert_eq!(shell.as_deref(), Some("/bin/bash"));
                assert_eq!(cwd.as_deref(), Some("/tmp"));
            }
            other => panic!("Expected TerminalCreate, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Write terminal input to PTY stdin
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "input", and terminal_id "T1"
    /// @step And the data contains a base64-encoded payload
    #[test]
    fn test_route_terminal_input() {
        let json = r#"{
            "service": "terminal",
            "type": "input",
            "terminal_id": "T1",
            "data": {"base64": "bHMK"}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::TerminalInput { terminal_id, base64_data } => {
                assert_eq!(terminal_id, "T1");
                assert_eq!(base64_data, "bHMK");
            }
            other => panic!("Expected TerminalInput, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Resize terminal PTY
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "resize", and terminal_id "T1"
    /// @step And the data contains cols 120 and rows 40
    #[test]
    fn test_route_terminal_resize() {
        let json = r#"{
            "service": "terminal",
            "type": "resize",
            "terminal_id": "T1",
            "data": {"cols": 120, "rows": 40}
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::TerminalResize { terminal_id, cols, rows } => {
                assert_eq!(terminal_id, "T1");
                assert_eq!(cols, 120);
                assert_eq!(rows, 40);
            }
            other => panic!("Expected TerminalResize, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Destroy terminal on command
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "destroy", terminal_id "T1", and request_id "d1"
    #[test]
    fn test_route_terminal_destroy() {
        let json = r#"{
            "service": "terminal",
            "type": "destroy",
            "terminal_id": "T1",
            "request_id": "d1"
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::TerminalDestroy { terminal_id, request_id } => {
                assert_eq!(terminal_id, "T1");
                assert_eq!(request_id, "d1");
            }
            other => panic!("Expected TerminalDestroy, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Respond to server ping with pong
    // =========================================================================

    /// @step Given the bridge relay is authenticated
    /// @step When a message arrives with service "system" and type "ping"
    /// @step Then the bridge should respond with service "system" and type "pong"
    #[test]
    fn test_route_system_ping() {
        let json = r#"{"service": "system", "type": "ping"}"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        assert!(matches!(action, InboundAction::SystemPing));
    }

    #[test]
    fn test_system_pong_envelope() {
        let env = Envelope::system_pong();
        assert_eq!(env.service, Service::System);
        assert_eq!(env.msg_type, "pong");
        assert!(env.data.is_none());
    }

    // =========================================================================
    // Terminal envelope builders
    // =========================================================================

    /// @step Then the agent should spawn a PTY shell process
    /// @step And send a "created" response with the generated terminal_id and request_id "t1"
    #[test]
    fn test_terminal_created_envelope() {
        let env = Envelope::terminal_created("my-project", "t1", "T1");
        assert_eq!(env.service, Service::Terminal);
        assert_eq!(env.msg_type, "created");
        assert_eq!(env.instance_id.as_deref(), Some("my-project"));
        assert_eq!(env.request_id.as_deref(), Some("t1"));
        assert_eq!(env.terminal_id.as_deref(), Some("T1"));
        let data = env.data.unwrap();
        assert_eq!(data["terminal_id"], "T1");
    }

    /// @step And start streaming PTY stdout as base64 data frames
    #[test]
    fn test_terminal_data_envelope() {
        let env = Envelope::terminal_data("my-project", "T1", "SGVsbG8=");
        assert_eq!(env.service, Service::Terminal);
        assert_eq!(env.msg_type, "data");
        assert_eq!(env.terminal_id.as_deref(), Some("T1"));
        let data = env.data.unwrap();
        assert_eq!(data["base64"], "SGVsbG8=");
    }

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When the shell process in terminal "T1" exits with code 0
    /// @step Then the bridge should send a terminal "exited" envelope with terminal_id "T1" and exit_code 0
    /// @step And the terminal should be removed from the PtyRegistry
    #[test]
    fn test_terminal_exited_envelope() {
        // @step When the shell process in terminal "T1" exits with code 0
        // @step Then the bridge should send a terminal "exited" envelope with terminal_id "T1" and exit_code 0
        let env = Envelope::terminal_exited("my-project", "T1", 0);
        assert_eq!(env.service, Service::Terminal);
        assert_eq!(env.msg_type, "exited");
        assert_eq!(env.terminal_id.as_deref(), Some("T1"));
        let data = env.data.unwrap();
        assert_eq!(data["exit_code"], 0);
        // @step And the terminal should be removed from the PtyRegistry
        // Registry removal is tested in bridge_pty.rs destroy_terminal tests
    }

    /// @step And send a "destroyed" response with terminal_id "T1" and request_id "d1"
    #[test]
    fn test_terminal_destroyed_envelope() {
        let env = Envelope::terminal_destroyed("my-project", "d1", "T1");
        assert_eq!(env.service, Service::Terminal);
        assert_eq!(env.msg_type, "destroyed");
        assert_eq!(env.terminal_id.as_deref(), Some("T1"));
        assert_eq!(env.request_id.as_deref(), Some("d1"));
    }

    // =========================================================================
    // Endpoint detection
    // =========================================================================

    #[test]
    fn test_is_multiplexed_ws_relay_path() {
        assert!(is_multiplexed_endpoint("ws://server:3001/ws/relay"));
        assert!(is_multiplexed_endpoint("wss://server.example.com/ws/relay"));
    }

    #[test]
    fn test_is_multiplexed_query_param() {
        assert!(is_multiplexed_endpoint("ws://server:3001/ws?protocol=multiplexed"));
    }

    #[test]
    fn test_not_multiplexed_plain_ws() {
        assert!(!is_multiplexed_endpoint("ws://server:3001/ws"));
        assert!(!is_multiplexed_endpoint("ws://telegram-relay.example.com/bridge"));
    }

    // =========================================================================
    // Round-trip serialization
    // =========================================================================

    #[test]
    fn test_envelope_round_trip() {
        let env = Envelope::relay_chunk("inst-1", "sess-1", serde_json::json!({"type": "text", "text": "Hi"}));
        let json = serde_json::to_string(&env).unwrap();
        let parsed: Envelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.service, Service::Relay);
        assert_eq!(parsed.msg_type, "chunk");
        assert_eq!(parsed.instance_id.as_deref(), Some("inst-1"));
        assert_eq!(parsed.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_envelope_optional_fields_omitted() {
        let env = Envelope::system_pong();
        let json = serde_json::to_string(&env).unwrap();
        assert!(!json.contains("instance_id"));
        assert!(!json.contains("session_id"));
        assert!(!json.contains("terminal_id"));
        assert!(!json.contains("request_id"));
        assert!(!json.contains("data"));
    }

    /// @step Test unknown service message is routed correctly
    #[test]
    fn test_route_unknown_session_type() {
        let json = r#"{"service": "session", "type": "unknown_type", "session_id": "s1"}"#;
        let env: Envelope = serde_json::from_str(json).unwrap();
        let action = route_inbound(&env);

        match action {
            InboundAction::Unknown { service, msg_type } => {
                assert_eq!(service, "session");
                assert_eq!(msg_type, "unknown_type");
            }
            other => panic!("Expected Unknown, got {other:?}"),
        }
    }

    /// Invalid service field should fail deserialization
    #[test]
    fn test_invalid_service_fails() {
        let json = r#"{"service": "invalid", "type": "foo"}"#;
        let result = serde_json::from_str::<Envelope>(json);
        assert!(result.is_err());
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
    #[test]
    fn test_route_session_create_produces_session_create_action() {
        // @step And a session:create envelope arrives with request_id "req-1" and instance_id "proj"
        let json = r#"{
            "service": "session",
            "type": "create",
            "instance_id": "proj",
            "request_id": "req-1"
        }"#;
        let env: Envelope = serde_json::from_str(json).unwrap();

        // @step When the bridge routes the inbound envelope
        let action = route_inbound(&env);

        // @step Then the route should produce a SessionCreate action with request_id "req-1"
        match action {
            InboundAction::SessionCreate { request_id } => {
                assert_eq!(request_id, "req-1");
            }
            other => panic!("Expected SessionCreate, got {other:?}"),
        }
    }

    /// @step And the response envelope should contain the new session_id
    /// @step And the response envelope should carry request_id "req-1"
    #[test]
    fn test_session_created_envelope_builder() {
        // The bridge needs an Envelope::session_created builder so it can respond
        // to session:create requests with the new session_id.
        let env = Envelope::session_created("proj", "req-1", "sess-new");

        assert_eq!(env.service, Service::Session);
        assert_eq!(env.msg_type, "created");
        assert_eq!(env.instance_id.as_deref(), Some("proj"));
        assert_eq!(env.request_id.as_deref(), Some("req-1"));
        let data = env.data.as_ref().unwrap();
        assert_eq!(data["session_id"], "sess-new");
    }
}
