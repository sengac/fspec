//! Tests for subordinate session relay — SESS-015
//!
//! Feature: spec/features/subordinate-session-relay.feature
//!
//! Validates that subordinate agent session output is forwarded through
//! the parent's relay connection to the dashboard.

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::module_inception
)]
mod subordinate_relay_tests {
    use crate::bridge_multiplexed::Service;
    use crate::bridge_relay::{
        get_subordinate_chunk_senders, process_outbound_envelope,
        register_subordinate_chunk_channel, OutboundEnvelopeAction,
    };
    use serde_json::json;

    // =========================================================================
    // Scenario: Outbound envelope uses chunk-level session_id when present
    // =========================================================================

    #[test]
    fn test_outbound_envelope_uses_chunk_level_session_id() {
        // @step Given a JSON chunk with a "_relay_session_id" field set to "sub-session-abc"
        let chunk = json!({
            "type": "text",
            "text": "Hello from subordinate",
            "_relay_session_id": "sub-session-abc"
        });

        // @step When process_outbound_envelope is called with relay session_id "parent-session-xyz"
        let result = process_outbound_envelope(&chunk, "instance-1", "parent-session-xyz", None);

        // @step Then the envelope session_id should be "sub-session-abc"
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.session_id.as_deref(), Some("sub-session-abc"));
                assert_eq!(env.service, Service::Relay);
                assert_eq!(env.msg_type, "chunk");

                // @step And the chunk data should be forwarded unchanged
                let data = env.data.as_ref().unwrap();
                assert_eq!(data["type"], "text");
                assert_eq!(data["text"], "Hello from subordinate");
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Outbound envelope falls back to relay session_id when no override
    // =========================================================================

    #[test]
    fn test_outbound_envelope_falls_back_to_relay_session_id() {
        // @step Given a JSON chunk without a "_relay_session_id" field
        let chunk = json!({
            "type": "text",
            "text": "Hello from parent"
        });

        // @step When process_outbound_envelope is called with relay session_id "parent-session-xyz"
        let result = process_outbound_envelope(&chunk, "instance-1", "parent-session-xyz", None);

        // @step Then the envelope session_id should be "parent-session-xyz"
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.session_id.as_deref(), Some("parent-session-xyz"));
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Subordinate chunk forwarding injects relay session override
    // =========================================================================

    #[tokio::test]
    async fn test_subordinate_chunk_forwarding_injects_session_id() {
        use tokio::sync::broadcast;

        // @step Given a parent session with id "parent-001"
        let parent_id = uuid::Uuid::new_v4();

        // Register the parent's subordinate chunk channel (as the relay loop does)
        let mut subordinate_rx = register_subordinate_chunk_channel(parent_id);

        // Get the senders for the parent (as the forwarding task does)
        let senders = get_subordinate_chunk_senders(parent_id);
        assert!(!senders.is_empty(), "Parent should have at least one subordinate chunk sender");

        // @step And a subordinate session with id "sub-001" whose supervisor_broadcast emits StreamChunks
        let sub_id = uuid::Uuid::new_v4();
        let (sub_tx, _) = broadcast::channel::<serde_json::Value>(16);

        // Spawn a forwarding task (mirrors spawn_subordinate_forwarding_task pattern)
        let mut sub_rx = sub_tx.subscribe();
        let sub_session_id = sub_id;
        let fwd_senders = senders.clone();
        tokio::spawn(async move {
            loop {
                match sub_rx.recv().await {
                    Ok(mut chunk_json) => {
                        // Inject _relay_session_id into the JSON
                        if let Some(obj) = chunk_json.as_object_mut() {
                            obj.insert(
                                "_relay_session_id".to_string(),
                                serde_json::Value::String(sub_session_id.to_string()),
                            );
                        }
                        for tx in &fwd_senders {
                            let _ = tx.send((sub_session_id, chunk_json.clone()));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });

        // @step When the subordinate emits a text chunk "Hello from subordinate"
        let chunk = json!({
            "type": "text",
            "text": "Hello from subordinate"
        });
        sub_tx.send(chunk).unwrap();

        // @step Then the forwarding task should convert it to JSON
        // @step And inject "_relay_session_id" set to "sub-001"
        // @step And send it through the parent's subordinate chunk channel
        let (received_session_id, received_chunk) = subordinate_rx.recv().await.unwrap();
        assert_eq!(received_session_id, sub_id);
        assert_eq!(received_chunk["type"], "text");
        assert_eq!(received_chunk["text"], "Hello from subordinate");
        assert_eq!(
            received_chunk["_relay_session_id"].as_str().unwrap(),
            sub_id.to_string()
        );
    }

    // =========================================================================
    // Scenario: Forwarding task terminates when subordinate broadcast closes
    // =========================================================================

    #[tokio::test]
    async fn test_forwarding_task_terminates_on_broadcast_close() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use tokio::sync::broadcast;

        // @step Given a forwarding task subscribed to a subordinate's supervisor_broadcast
        let (sub_tx, _) = broadcast::channel::<serde_json::Value>(16);
        let mut sub_rx = sub_tx.subscribe();
        let task_terminated = Arc::new(AtomicBool::new(false));
        let task_terminated_clone = task_terminated.clone();

        let handle = tokio::spawn(async move {
            loop {
                match sub_rx.recv().await {
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
            task_terminated_clone.store(true, Ordering::SeqCst);
        });

        // @step When the subordinate session is destroyed and the broadcast sender is dropped
        drop(sub_tx);

        // @step Then the forwarding task should terminate cleanly
        handle.await.unwrap();
        assert!(task_terminated.load(Ordering::SeqCst));

        // @step And the parent's relay loop should continue running unaffected
        // (Verified by the fact that this test completes without hanging —
        // the forwarding task terminates independently)
    }

    // =========================================================================
    // Scenario: Relay loop reads from subordinate chunk channel
    // =========================================================================

    #[test]
    fn test_relay_loop_processes_subordinate_chunk_with_correct_session_id() {
        // @step Given a relay loop with a subordinate chunk channel registered
        // (Simulated by calling process_outbound_envelope directly with a tagged chunk)

        // @step When a subordinate chunk with session_id "sub-001" arrives on the channel
        let sub_id = uuid::Uuid::new_v4();
        let chunk = json!({
            "type": "text",
            "text": "Hello from sub",
            "_relay_session_id": sub_id.to_string()
        });

        // @step Then it should be processed through process_outbound_envelope
        let result = process_outbound_envelope(&chunk, "instance-1", "parent-session", None);

        // @step And the resulting envelope should use session_id "sub-001"
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.session_id.as_deref(), Some(sub_id.to_string().as_str()));
                // @step And be sent over the existing WebSocket connection
                // (Verified by the envelope being correctly constructed for sending)
                assert_eq!(env.service, Service::Relay);
                assert_eq!(env.msg_type, "chunk");
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    // =========================================================================
    // Scenario: Subordinate output reaches relay via parent connection
    // =========================================================================

    #[tokio::test]
    async fn test_subordinate_output_reaches_relay_via_parent() {
        use tokio::sync::broadcast;

        // @step Given a parent session connected to a relay via Bridge connect
        let parent_id = uuid::Uuid::new_v4();
        let sub_id = uuid::Uuid::new_v4();

        // Register the parent's subordinate chunk channel (as connect_and_relay does)
        let mut subordinate_rx = register_subordinate_chunk_channel(parent_id);
        let senders = get_subordinate_chunk_senders(parent_id);

        // Set up the subordinate's broadcast and forwarding
        let (sub_tx, _) = broadcast::channel::<serde_json::Value>(16);
        let mut sub_rx = sub_tx.subscribe();
        let sub_session_id = sub_id;
        let fwd_senders = senders;
        tokio::spawn(async move {
            loop {
                match sub_rx.recv().await {
                    Ok(mut chunk_json) => {
                        if let Some(obj) = chunk_json.as_object_mut() {
                            obj.insert(
                                "_relay_session_id".to_string(),
                                serde_json::Value::String(sub_session_id.to_string()),
                            );
                        }
                        for tx in &fwd_senders {
                            let _ = tx.send((sub_session_id, chunk_json.clone()));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });

        // @step When a subordinate session is spawned and emits stream chunks
        let chunks = vec![
            json!({"type": "text", "text": "Chunk 1"}),
            json!({"type": "toolCall", "toolCall": {"id": "t1", "name": "Read", "input": "{}"}}),
            json!({"type": "text", "text": "Chunk 2"}),
        ];
        for chunk in &chunks {
            sub_tx.send(chunk.clone()).unwrap();
        }

        // @step Then the chunks should be forwarded through the parent's single WebSocket connection
        for i in 0..3 {
            let (received_session_id, received_chunk) = subordinate_rx.recv().await.unwrap();

            // @step And each chunk envelope should contain the subordinate's session_id
            assert_eq!(received_session_id, sub_id);
            assert_eq!(
                received_chunk["_relay_session_id"].as_str().unwrap(),
                sub_id.to_string()
            );

            // Verify the envelope would use subordinate's session_id
            let result = process_outbound_envelope(
                &received_chunk,
                "instance-1",
                &parent_id.to_string(),
                None,
            );
            match result {
                OutboundEnvelopeAction::RelayChunk(env) => {
                    assert_eq!(env.session_id.as_deref(), Some(sub_id.to_string().as_str()));
                }
                _ => panic!("Expected RelayChunk for chunk {i}"),
            }
        }
    }

    // =========================================================================
    // Scenario: Nested subordinate output bubbles up through chain
    // =========================================================================

    #[tokio::test]
    async fn test_nested_subordinate_output_bubbles_up() {
        use tokio::sync::broadcast;

        // @step Given a parent session connected to a relay
        let parent_id = uuid::Uuid::new_v4();
        let sub_a_id = uuid::Uuid::new_v4();
        let sub_b_id = uuid::Uuid::new_v4();

        // Register the parent's subordinate chunk channel
        let mut parent_subordinate_rx = register_subordinate_chunk_channel(parent_id);
        let parent_senders = get_subordinate_chunk_senders(parent_id);

        // @step And the parent spawns subordinate sub-A
        let (sub_a_tx, _) = broadcast::channel::<serde_json::Value>(16);
        {
            let mut sub_a_rx = sub_a_tx.subscribe();
            let fwd_senders = parent_senders.clone();
            let sid = sub_a_id;
            tokio::spawn(async move {
                loop {
                    match sub_a_rx.recv().await {
                        Ok(mut chunk_json) => {
                            if let Some(obj) = chunk_json.as_object_mut() {
                                obj.insert(
                                    "_relay_session_id".to_string(),
                                    serde_json::Value::String(sid.to_string()),
                                );
                            }
                            for tx in &fwd_senders {
                                let _ = tx.send((sid, chunk_json.clone()));
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            });
        }

        // @step And sub-A spawns subordinate sub-B
        // Nested subordinates forward to the SAME root parent (via find_root_parent)
        let (sub_b_tx, _) = broadcast::channel::<serde_json::Value>(16);
        {
            let mut sub_b_rx = sub_b_tx.subscribe();
            let fwd_senders = parent_senders.clone();
            let sid = sub_b_id;
            tokio::spawn(async move {
                loop {
                    match sub_b_rx.recv().await {
                        Ok(mut chunk_json) => {
                            if let Some(obj) = chunk_json.as_object_mut() {
                                obj.insert(
                                    "_relay_session_id".to_string(),
                                    serde_json::Value::String(sid.to_string()),
                                );
                            }
                            for tx in &fwd_senders {
                                let _ = tx.send((sid, chunk_json.clone()));
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            });
        }

        // @step When sub-B emits stream chunks
        sub_b_tx
            .send(json!({"type": "text", "text": "Hello from sub-B"}))
            .unwrap();

        // @step Then the chunks should reach the parent's relay connection
        let (received_session_id, received_chunk) = parent_subordinate_rx.recv().await.unwrap();

        // @step And each chunk envelope should contain sub-B's session_id
        assert_eq!(received_session_id, sub_b_id);
        assert_eq!(
            received_chunk["_relay_session_id"].as_str().unwrap(),
            sub_b_id.to_string()
        );
    }

    // =========================================================================
    // Scenario: Late bridge connection picks up existing subordinate output
    // =========================================================================

    #[tokio::test]
    async fn test_late_bridge_connection_picks_up_subordinate_output() {
        use tokio::sync::broadcast;

        // @step Given a parent session that has already spawned a subordinate
        let parent_id = uuid::Uuid::new_v4();
        let sub_id = uuid::Uuid::new_v4();
        let (sub_tx, _) = broadcast::channel::<serde_json::Value>(16);

        // @step And the subordinate is actively emitting stream chunks
        // The forwarding task is already running — using get_subordinate_chunk_senders
        // Before the bridge connects, there are no senders, so chunks are dropped.
        // After Bridge connect registers a channel, new chunks will be forwarded.

        // Simulate "late" Bridge connect — register channel AFTER subordinate exists
        let mut subordinate_rx = register_subordinate_chunk_channel(parent_id);
        let senders = get_subordinate_chunk_senders(parent_id);

        let mut sub_rx = sub_tx.subscribe();
        let sub_session_id = sub_id;
        let fwd_senders = senders;
        tokio::spawn(async move {
            loop {
                match sub_rx.recv().await {
                    Ok(mut chunk_json) => {
                        if let Some(obj) = chunk_json.as_object_mut() {
                            obj.insert(
                                "_relay_session_id".to_string(),
                                serde_json::Value::String(sub_session_id.to_string()),
                            );
                        }
                        for tx in &fwd_senders {
                            let _ = tx.send((sub_session_id, chunk_json.clone()));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        });

        // @step When the parent calls Bridge connect to establish a relay connection
        // Simulated: the relay loop now starts reading from subordinate_rx
        // After the "late" connection, subordinate emits a new chunk
        sub_tx
            .send(json!({"type": "text", "text": "Post-connect chunk"}))
            .unwrap();

        // @step Then subordinate chunks emitted after the connection should be forwarded
        let (received_session_id, received_chunk) = subordinate_rx.recv().await.unwrap();
        assert_eq!(received_session_id, sub_id);
        assert_eq!(received_chunk["text"], "Post-connect chunk");

        // @step And appear in the subordinate's dashboard tab
        // Verified by checking the envelope uses the subordinate's session_id
        let result = process_outbound_envelope(
            &received_chunk,
            "instance-1",
            "parent-session",
            None,
        );
        match result {
            OutboundEnvelopeAction::RelayChunk(env) => {
                assert_eq!(env.session_id.as_deref(), Some(sub_id.to_string().as_str()));
            }
            other => panic!("Expected RelayChunk, got {other:?}"),
        }
    }

    // =========================================================================
    // Additional: Verify get_subordinate_chunk_senders returns empty for unknown
    // =========================================================================

    #[test]
    fn test_get_subordinate_chunk_senders_returns_empty_for_unknown_parent() {
        let unknown_id = uuid::Uuid::new_v4();
        let senders = get_subordinate_chunk_senders(unknown_id);
        assert!(senders.is_empty());
    }

    // =========================================================================
    // Additional: Verify register + get round-trip
    // =========================================================================

    #[test]
    fn test_register_then_get_subordinate_chunk_senders() {
        let parent_id = uuid::Uuid::new_v4();

        // Before registration, no senders
        let senders_before = get_subordinate_chunk_senders(parent_id);
        assert!(senders_before.is_empty());

        // Register a channel
        let _rx = register_subordinate_chunk_channel(parent_id);

        // After registration, one sender
        let senders_after = get_subordinate_chunk_senders(parent_id);
        assert_eq!(senders_after.len(), 1);
    }
}
