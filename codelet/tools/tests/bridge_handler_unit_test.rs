#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Unit tests extracted from `src/bridge_handler.rs`.
//!
//! These tests verify bridge handler registration, session context management,
//! and bridge action execution.

use codelet_tools::bridge::{BridgeAction, BridgeResult};
use codelet_tools::bridge_handler::{
    execute_bridge_command, handle_bridge_action, has_bridge_handler_for_session,
    remove_bridge_session_context, set_bridge_handler, set_bridge_session_context, BridgeHandler,
    BridgeRequest, BroadcastReceiverFactory,
};
use codelet_tools::bridge_relay::InputInjector;
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

fn with_clean_handler<T>(f: impl FnOnce(Uuid) -> T) -> T {
    let sid = Uuid::new_v4();
    set_bridge_handler(sid, None);
    let result = f(sid);
    set_bridge_handler(sid, None);
    result
}

#[test]
#[serial]
fn test_no_handler_returns_error() {
    with_clean_handler(|_sid| {
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
    with_clean_handler(|sid| {
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

        set_bridge_handler(sid, Some(handler));

        let result = execute_bridge_command(BridgeRequest {
            session_id: sid,
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
    with_clean_handler(|sid| {
        // No handler or context
        assert!(!has_bridge_handler_for_session(sid));

        // Set handler only
        let handler: BridgeHandler = Arc::new(|_| BridgeResult {
            success: true,
            message: String::new(),
            connections: None,
        });
        set_bridge_handler(sid, Some(handler));
        assert!(!has_bridge_handler_for_session(sid)); // Still false - no context

        // Set context
        let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
        let broadcast_factory: BroadcastReceiverFactory = Arc::new(move || tx.subscribe());
        let input_injector: InputInjector = Arc::new(|_| {});
        set_bridge_session_context(sid, broadcast_factory, input_injector, None, None);

        assert!(has_bridge_handler_for_session(sid)); // Now true

        // Remove context
        remove_bridge_session_context(sid);
        assert!(!has_bridge_handler_for_session(sid));
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
    codelet_tools::bridge::remove_bridge_manager(session_id).await;
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
    codelet_tools::bridge::remove_bridge_manager(session_id).await;
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

    let broadcast_rx_factory: BroadcastReceiverFactory =
        Arc::new(move || broadcast_tx_clone.subscribe());

    let input_injector: InputInjector =
        Arc::new(|_input: codelet_tools::bridge_relay::InjectedInput| {
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
    codelet_tools::bridge::remove_bridge_manager(session_id).await;
}
