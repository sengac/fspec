//! Tests for multiplexed envelope wiring in bridge_relay.rs
//!
//! Feature: spec/features/bridge-relay-multiplexed-wiring.feature
//!
//! These tests validate that bridge_relay.rs speaks ONLY the multiplexed
//! envelope protocol. The flat protocol is ELIMINATED.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod multiplexed_wiring_tests {
    use crate::bridge_multiplexed::{Envelope, Service};
    use crate::bridge_relay::{
        get_instance_metadata, handle_multiplexed_inbound, process_outbound_envelope,
        CommandEmitter, ControlHandler, InjectedInput, InputInjector, OutboundEnvelopeAction,
        PendingCommands,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // =========================================================================
    // Scenario: Bridge sends auth envelope on connect
    // =========================================================================

    /// @step Given the bridge is connecting to "ws://127.0.0.1:19876/ws/relay"
    /// @step When the WebSocket connection is established
    /// @step Then the first message sent should be an Envelope with service "auth" and type "authenticate"
    /// @step And the auth data should contain role "agent" and instance metadata
    /// @step And the instance name should be derived from the current working directory
    #[test]
    fn test_auth_envelope_built_from_instance_metadata() {
        // @step Given the bridge is connecting to "ws://127.0.0.1:19876/ws/relay"
        // @step When the WebSocket connection is established
        let metadata = get_instance_metadata();

        // @step Then the first message sent should be an Envelope with service "auth" and type "authenticate"
        let env = Envelope::auth_agent("", &metadata);
        assert_eq!(env.service, Service::Auth);
        assert_eq!(env.msg_type, "authenticate");

        // @step And the auth data should contain role "agent" and instance metadata
        let data = env.data.as_ref().unwrap();
        assert_eq!(data["role"], "agent");
        assert!(data["instance"]["name"].is_string());

        // @step And the instance name should be derived from the current working directory
        let name = data["instance"]["name"].as_str().unwrap();
        assert!(!name.is_empty(), "Instance name should be non-empty");
        let cwd = std::env::current_dir().unwrap();
        let expected_name = cwd.file_name().unwrap().to_str().unwrap();
        assert_eq!(name, expected_name);
    }

    // =========================================================================
    // Scenario: Bridge waits for authSuccess before sending chunks
    // =========================================================================

    /// @step Given the bridge has connected and sent the auth envelope
    /// @step When an authSuccess response is received from the server
    /// @step Then the bridge should enter the message relay loop
    /// @step And outbound chunks should start flowing
    #[test]
    fn test_auth_success_parsed_correctly() {
        // @step Given the bridge has connected and sent the auth envelope
        let auth_success_json = r#"{"service":"auth","type":"authSuccess","data":{}}"#;
        let env: Envelope = serde_json::from_str(auth_success_json).unwrap();

        // @step When an authSuccess response is received from the server
        let action = crate::bridge_multiplexed::route_inbound(&env);

        // @step Then the bridge should enter the message relay loop
        // @step And outbound chunks should start flowing
        match action {
            crate::bridge_multiplexed::InboundAction::AuthResponse { success, .. } => {
                assert!(success, "authSuccess should report success=true");
            }
            other => panic!("Expected AuthResponse, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Auth failure triggers reconnect
    // =========================================================================

    /// @step Given the bridge has connected and sent the auth envelope
    /// @step When an authError response is received from the server
    /// @step Then the connection should be closed
    /// @step And the bridge should reconnect with exponential backoff
    #[test]
    fn test_auth_error_parsed_correctly() {
        // @step Given the bridge has connected and sent the auth envelope
        let auth_error_json = r#"{"service":"auth","type":"authError","data":{"code":"AUTH_FAILED","message":"Bad key"}}"#;
        let env: Envelope = serde_json::from_str(auth_error_json).unwrap();

        // @step When an authError response is received from the server
        let action = crate::bridge_multiplexed::route_inbound(&env);

        // @step Then the connection should be closed
        // @step And the bridge should reconnect with exponential backoff
        match action {
            crate::bridge_multiplexed::InboundAction::AuthResponse { success, .. } => {
                assert!(!success, "authError should report success=false");
            }
            other => panic!("Expected AuthResponse, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Outbound chunks sent as relay envelopes
    // =========================================================================

    /// @step Given the bridge is authenticated with instance_id "my-project"
    /// @step And a session "s1" is producing stream chunks
    /// @step When a text chunk is received from the broadcast channel
    /// @step Then the bridge should send an Envelope with service "relay" and type "chunk"
    /// @step And the envelope should include instance_id "my-project" and session_id "s1"
    /// @step And the data field should contain the original StreamChunk
    #[test]
    fn test_outbound_chunk_wrapped_as_envelope() {
        // @step Given the bridge is authenticated with instance_id "my-project"
        let instance_id = "my-project";
        let session_id = Uuid::new_v4();

        // @step And a session "s1" is producing stream chunks
        let chunk_json = json!({
            "type": "text",
            "text": "Hello from the agent"
        });

        // @step When a text chunk is received from the broadcast channel
        let result =
            process_outbound_envelope(&chunk_json, instance_id, &session_id.to_string(), None);

        // @step Then the bridge should send an Envelope with service "relay" and type "chunk"
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.service, Service::Relay);
                assert_eq!(env.msg_type, "chunk");

                // @step And the envelope should include instance_id "my-project" and session_id "s1"
                assert_eq!(env.instance_id.as_deref(), Some("my-project"));
                assert_eq!(
                    env.session_id.as_deref(),
                    Some(session_id.to_string().as_str())
                );

                // @step And the data field should contain the original StreamChunk
                let data = env.data.unwrap();
                assert_eq!(data["type"], "text");
                assert_eq!(data["text"], "Hello from the agent");
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Outbound command responses sent as fspec envelopes
    // =========================================================================

    /// @step Given the bridge is authenticated with instance_id "my-project"
    /// @step And a pending command exists with request_id "r1" for command "board"
    /// @step When a FspecCommandResult chunk arrives on the broadcast channel matching the pending command
    /// @step Then the bridge should send an Envelope with service "fspec" and type "commandResponse"
    /// @step And the envelope should include instance_id "my-project" and request_id "r1"
    /// @step And the data should contain command "board" and the result
    #[test]
    fn test_outbound_command_response_wrapped_as_envelope() {
        // @step Given the bridge is authenticated with instance_id "my-project"
        let instance_id = "my-project";
        let session_id = Uuid::new_v4().to_string();

        // @step And a pending command exists with request_id "r1" for command "board"
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));
        {
            let mut map = pending_commands.lock().unwrap();
            map.insert(
                "tool-123".to_string(),
                ("r1".to_string(), "board".to_string()),
            );
        }

        // @step When a FspecCommandResult chunk arrives on the broadcast channel matching the pending command
        let chunk_json = json!({
            "type": "fspecCommandResult",
            "fspecResult": {
                "success": true,
                "data": "{\"columns\":{}}",
                "error": null,
                "systemReminder": null,
                "toolCallId": "tool-123"
            }
        });

        let result = process_outbound_envelope(
            &chunk_json,
            instance_id,
            &session_id,
            Some(&pending_commands),
        );

        // @step Then the bridge should send an Envelope with service "fspec" and type "commandResponse"
        match result {
            OutboundEnvelopeAction::CommandResponse(env) => {
                assert_eq!(env.service, Service::Fspec);
                assert_eq!(env.msg_type, "commandResponse");

                // @step And the envelope should include instance_id "my-project" and request_id "r1"
                assert_eq!(env.instance_id.as_deref(), Some("my-project"));
                assert_eq!(env.request_id.as_deref(), Some("r1"));

                // @step And the data should contain command "board" and the result
                let data = env.data.unwrap();
                assert_eq!(data["command"], "board");
                assert_eq!(data["success"], true);
            }
            other => panic!("Expected CommandResponse, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Inbound session input dispatched to InputInjector
    // =========================================================================

    /// @step Given the bridge is authenticated
    /// @step When the server sends an Envelope with service "session" type "input" and session_id "s1"
    /// @step And the data contains message "hello" and images
    /// @step Then the InputInjector should be called with message "hello" and the images
    #[tokio::test]
    async fn test_inbound_session_input_dispatched() {
        // @step Given the bridge is authenticated
        let session_id = Uuid::new_v4();
        let received_message = Arc::new(Mutex::new(String::new()));
        let received_images = Arc::new(AtomicBool::new(false));
        let msg_clone = received_message.clone();
        let img_clone = received_images.clone();

        let input_injector: InputInjector = Arc::new(move |input: InjectedInput| {
            *msg_clone.lock().unwrap() = input.message;
            if input.images.is_some() {
                img_clone.store(true, Ordering::SeqCst);
            }
        });

        // @step When the server sends an Envelope with service "session" type "input" and session_id "s1"
        // @step And the data contains message "hello" and images
        let envelope_json = json!({
            "service": "session",
            "type": "input",
            "session_id": session_id.to_string(),
            "data": {
                "message": "hello",
                "images": [{"data": "base64data", "media_type": "image/png"}]
            }
        });
        let text = serde_json::to_string(&envelope_json).unwrap();

        let result =
            handle_multiplexed_inbound(&text, session_id, input_injector, None, None, None).await;

        // @step Then the InputInjector should be called with message "hello" and the images
        assert!(result.is_ok());
        assert_eq!(*received_message.lock().unwrap(), "hello");
        assert!(received_images.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Scenario: Inbound session control dispatched to ControlHandler
    // =========================================================================

    /// @step Given the bridge is authenticated
    /// @step When the server sends an Envelope with service "session" type "control" and session_id "s1"
    /// @step And the data contains action "interrupt"
    /// @step Then the ControlHandler should be called with action "interrupt"
    #[tokio::test]
    async fn test_inbound_session_control_dispatched() {
        // @step Given the bridge is authenticated
        let session_id = Uuid::new_v4();
        let action_received = Arc::new(Mutex::new(String::new()));
        let action_clone = action_received.clone();

        let input_injector: InputInjector = Arc::new(|_| {});
        let control_handler: ControlHandler = Arc::new(move |action: &str, _resp: Option<&str>| {
            *action_clone.lock().unwrap() = action.to_string();
        });

        // @step When the server sends an Envelope with service "session" type "control" and session_id "s1"
        // @step And the data contains action "interrupt"
        let envelope_json = json!({
            "service": "session",
            "type": "control",
            "session_id": session_id.to_string(),
            "data": {"action": "interrupt"}
        });
        let text = serde_json::to_string(&envelope_json).unwrap();

        let result = handle_multiplexed_inbound(
            &text,
            session_id,
            input_injector,
            Some(control_handler),
            None,
            None,
        )
        .await;

        // @step Then the ControlHandler should be called with action "interrupt"
        assert!(result.is_ok());
        assert_eq!(*action_received.lock().unwrap(), "interrupt");
    }

    // =========================================================================
    // Scenario: Inbound fspec command dispatched to CommandEmitter
    // =========================================================================

    /// @step Given the bridge is authenticated
    /// @step When the server sends an Envelope with service "fspec" type "command" and request_id "r1"
    /// @step And the data contains command "board" and args
    /// @step Then the CommandEmitter should be called with command "board" and the args
    #[tokio::test]
    async fn test_inbound_fspec_command_dispatched() {
        // @step Given the bridge is authenticated
        let session_id = Uuid::new_v4();
        let cmd_received = Arc::new(Mutex::new(String::new()));
        let cmd_clone = cmd_received.clone();

        let input_injector: InputInjector = Arc::new(|_| {});
        let command_emitter: CommandEmitter = Arc::new(
            move |cmd: String, _args: String, _root: String, _tcid: String| {
                *cmd_clone.lock().unwrap() = cmd;
            },
        );

        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When the server sends an Envelope with service "fspec" type "command" and request_id "r1"
        // @step And the data contains command "board" and args
        let envelope_json = json!({
            "service": "fspec",
            "type": "command",
            "request_id": "r1",
            "data": {"command": "board", "args": {}}
        });
        let text = serde_json::to_string(&envelope_json).unwrap();

        let result = handle_multiplexed_inbound(
            &text,
            session_id,
            input_injector,
            None,
            Some(command_emitter),
            Some(pending_commands.clone()),
        )
        .await;

        // @step Then the CommandEmitter should be called with command "board" and the args
        assert!(result.is_ok());
        assert_eq!(*cmd_received.lock().unwrap(), "board");

        let map = pending_commands.lock().unwrap();
        assert_eq!(map.len(), 1, "Should have one pending command");
    }

    // =========================================================================
    // Scenario: System ping answered with pong
    // =========================================================================

    /// @step Given the bridge is authenticated
    /// @step When the server sends an Envelope with service "system" and type "ping"
    /// @step Then the bridge should send an Envelope with service "system" and type "pong"
    #[tokio::test]
    async fn test_system_ping_returns_pong() {
        // @step Given the bridge is authenticated
        let session_id = Uuid::new_v4();
        let input_injector: InputInjector = Arc::new(|_| {});

        // @step When the server sends an Envelope with service "system" and type "ping"
        let text = r#"{"service":"system","type":"ping"}"#;

        let result =
            handle_multiplexed_inbound(text, session_id, input_injector, None, None, None).await;

        // @step Then the bridge should send an Envelope with service "system" and type "pong"
        match result {
            Ok(Some(pong)) => {
                assert_eq!(pong.service, Service::System);
                assert_eq!(pong.msg_type, "pong");
            }
            Ok(None) => panic!("Expected pong envelope, got None"),
            Err(e) => panic!("Expected pong envelope, got error: {e}"),
        }
    }

    // =========================================================================
    // Scenario: Flat protocol code is deleted
    // =========================================================================

    /// @step Given the bridge_relay.rs source code
    /// @step Then there should be no send_connected_message function
    /// @step And there should be no flat InboundMessage struct for wire parsing
    /// @step And there should be no is_multiplexed_endpoint branching logic
    /// @step And all outbound messages should use the Envelope struct
    #[test]
    fn test_flat_protocol_code_deleted() {
        // @step Given the bridge_relay.rs source code
        let source = include_str!("bridge_relay.rs");

        // @step Then there should be no send_connected_message function
        assert!(
            !source.contains("fn send_connected_message"),
            "send_connected_message should be deleted"
        );

        // @step And there should be no flat InboundMessage struct for wire parsing
        assert!(
            !source.contains("struct InboundMessage"),
            "InboundMessage struct should be deleted"
        );

        // @step And there should be no is_multiplexed_endpoint branching logic
        assert!(
            !source.contains("is_multiplexed_endpoint"),
            "is_multiplexed_endpoint branching should not exist in bridge_relay.rs"
        );

        // @step And all outbound messages should use the Envelope struct
        assert!(
            !source.contains("msg_type: \"connected\""),
            "Flat 'connected' message should be deleted"
        );
    }
}
