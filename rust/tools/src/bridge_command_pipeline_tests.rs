//! Tests for the bridge command handling pipeline in bridge_relay.rs
//!
//! Feature: spec/features/bridge-command-handling-pipeline.feature
//!
//! Commands arrive as `fspec` envelopes (the flat InboundMessage protocol
//! was removed by the multiplexed relay), are handed to the session's
//! CommandEmitter, and their FspecCommandResult chunks go back out as
//! `commandResponse` envelopes.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod command_pipeline_tests {
    use crate::bridge_multiplexed::Service;
    use crate::bridge_relay::{
        handle_multiplexed_inbound, process_outbound_envelope, CommandEmitter, InputInjector,
        OutboundEnvelopeAction, PendingCommands,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    /// Captures formatted tracing output for the warning assertion.
    #[derive(Clone, Default)]
    struct CapturedLog(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for CapturedLog {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
        type Writer = CapturedLog;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// (command, args_json, project_root, tool_call_id) per emitter call.
    type EmitterCalls = Arc<Mutex<Vec<(String, String, String, String)>>>;

    fn command_envelope(request_id: &str) -> String {
        json!({
            "service": "fspec",
            "type": "command",
            "request_id": request_id,
            "data": {"command": "board", "args": {}}
        })
        .to_string()
    }

    fn no_input() -> InputInjector {
        Arc::new(|_| {})
    }

    #[tokio::test]
    async fn test_fspec_command_envelope_emits_fspec_command_request() {
        // @step Given a bridge relay is connected with a command_emitter configured
        let calls: EmitterCalls = Arc::default();
        let recorded = calls.clone();
        let command_emitter: CommandEmitter =
            Arc::new(move |command, args_json, project_root, tool_call_id| {
                recorded
                    .lock()
                    .unwrap()
                    .push((command, args_json, project_root, tool_call_id));
            });

        // @step And a pending_commands map is initialized for the connection
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When the relay receives an Envelope with service "fspec", type "command", request_id "r1", and data command "board" with args {}
        let result = handle_multiplexed_inbound(
            &command_envelope("r1"),
            Uuid::new_v4(),
            no_input(),
            None,
            Some(command_emitter),
            Some(pending_commands.clone()),
        )
        .await;
        assert!(matches!(result, Ok(None)), "got {result:?}");

        // @step Then a UUID tool_call_id should be generated
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "the emitter should be called once");
        let (command, args_json, project_root, tool_call_id) = &calls[0];
        assert!(
            Uuid::parse_str(tool_call_id).is_ok(),
            "tool_call_id should be a UUID, got {tool_call_id:?}"
        );

        // @step And the pending_commands map should contain a mapping from the tool_call_id to request_id "r1" and command "board"
        let map = pending_commands.lock().unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get(tool_call_id),
            Some(&("r1".to_string(), "board".to_string()))
        );

        // @step And the command_emitter should be called with command "board", args_json "{}", a project_root from current_dir, and the generated tool_call_id
        assert_eq!(command, "board");
        assert_eq!(args_json, "{}");
        assert_eq!(
            project_root,
            &std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
    }

    #[tokio::test]
    async fn test_fspec_command_envelope_without_command_emitter() {
        let log = CapturedLog::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(log.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::WARN)
            .finish();
        let _subscriber = tracing::subscriber::set_default(subscriber);

        // @step Given a bridge relay is connected without a command_emitter
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When the relay receives an Envelope with service "fspec", type "command", request_id "r2", and data command "board" with args {}
        let result = handle_multiplexed_inbound(
            &command_envelope("r2"),
            Uuid::new_v4(),
            no_input(),
            None,
            None,
            Some(pending_commands.clone()),
        )
        .await;

        // @step Then no emitter callback should be invoked
        assert!(
            pending_commands.lock().unwrap().is_empty(),
            "no tool_call_id may be registered when nothing emits the command"
        );

        // @step And the handler should return Ok without crashing
        assert!(matches!(result, Ok(None)), "got {result:?}");

        // @step And a warning should be logged about no command emitter being configured
        let text = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
        assert!(
            text.lines()
                .any(|line| line.contains("WARN") && line.contains("no emitter configured")),
            "expected a no-emitter warning, got: {text}"
        );
    }

    #[test]
    fn test_fspec_command_result_sends_command_response() {
        // @step Given a bridge relay has a pending command with tool_call_id "abc-123" mapped to request_id "r1" and command "board"
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::from([(
            "abc-123".to_string(),
            ("r1".to_string(), "board".to_string()),
        )])));

        // @step When a FspecCommandResult chunk with toolCallId "abc-123", success true, and data containing results arrives on the broadcast channel
        let chunk_json = json!({
            "type": "fspecCommandResult",
            "fspecResult": {
                "success": true,
                "data": "{\"columns\":{\"backlog\":[\"AUTH-001\"]}}",
                "error": null,
                "systemReminder": null,
                "toolCallId": "abc-123"
            }
        });
        let result =
            process_outbound_envelope(&chunk_json, "my-project", "s1", Some(&pending_commands));

        // @step Then the pending_commands map entry for "abc-123" should be removed
        assert!(pending_commands.lock().unwrap().get("abc-123").is_none());

        // @step And the FspecCommandResult chunk should NOT be forwarded as a regular "chunk" message
        let env = match result {
            OutboundEnvelopeAction::CommandResponse(env) => env,
            other => panic!("Expected CommandResponse, got {other:?}"),
        };

        // @step And a commandResponse envelope with service "fspec" should be sent with request_id "r1"
        assert_eq!(env.service, Service::Fspec);
        assert_eq!(env.msg_type, "commandResponse");
        assert_eq!(env.request_id.as_deref(), Some("r1"));

        // @step And the commandResponse data should contain command "board", success true, and the result value
        let data = env.data.unwrap();
        assert_eq!(data["command"], "board");
        assert_eq!(data["success"], true);
        assert_eq!(
            data["result"],
            json!({"columns": {"backlog": ["AUTH-001"]}})
        );
    }

    #[test]
    fn test_fspec_command_request_chunk_is_skipped() {
        // @step Given a bridge relay is connected and processing outbound chunks
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When a FspecCommandRequest chunk with type "fspecCommandRequest" arrives on the broadcast channel
        let chunk_json = json!({
            "type": "fspecCommandRequest",
            "fspecRequest": {"command": "board", "argsJson": "{}", "projectRoot": "/p", "toolCallId": "t1"}
        });
        let result =
            process_outbound_envelope(&chunk_json, "my-project", "s1", Some(&pending_commands));

        // @step Then the chunk should be silently skipped
        // @step And no envelope should be sent to the WebSocket for this chunk
        assert!(
            matches!(result, OutboundEnvelopeAction::Skip),
            "got {result:?}"
        );
    }

    #[test]
    fn test_regular_text_chunk_forwarded_unchanged() {
        // @step Given a bridge relay is connected and processing outbound chunks
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When a regular text chunk with type "text" arrives on the broadcast channel
        let chunk_json = json!({"type": "text", "text": "Hello from the agent"});
        let result =
            process_outbound_envelope(&chunk_json, "my-project", "s1", Some(&pending_commands));

        // @step Then an Envelope with service "relay", type "chunk" and no request_id should be sent to the WebSocket
        let env = match result {
            OutboundEnvelopeAction::RelayChunk(env) => env,
            other => panic!("Expected RelayChunk, got {other:?}"),
        };
        assert_eq!(env.service, Service::Relay);
        assert_eq!(env.msg_type, "chunk");
        assert!(env.request_id.is_none());

        // @step And the chunk data should be forwarded without modification
        assert_eq!(env.data, Some(chunk_json));
    }

    #[test]
    fn test_fspec_command_result_with_unknown_tool_call_id_is_skipped() {
        // @step Given a bridge relay has an empty pending_commands map
        let pending_commands: PendingCommands = Arc::new(Mutex::new(HashMap::new()));

        // @step When a FspecCommandResult chunk with toolCallId "xyz-unknown" arrives on the broadcast channel
        let chunk_json = json!({
            "type": "fspecCommandResult",
            "fspecResult": {"success": true, "data": "{}", "toolCallId": "xyz-unknown"}
        });
        let result =
            process_outbound_envelope(&chunk_json, "my-project", "s1", Some(&pending_commands));

        // @step Then no commandResponse should be sent
        assert!(!matches!(
            result,
            OutboundEnvelopeAction::CommandResponse(_)
        ));

        // @step And the FspecCommandResult should NOT be forwarded as a regular chunk
        assert!(!matches!(result, OutboundEnvelopeAction::RelayChunk(_)));

        // @step And the chunk should be silently skipped without crashing
        assert!(
            matches!(result, OutboundEnvelopeAction::Skip),
            "got {result:?}"
        );
    }
}
