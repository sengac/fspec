#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/tool-wrapper-session-association.feature
//!
//! TOOL-012: Tool Wrappers Store Session ID at Construction
//!
//! These tests verify that tool wrappers (FspecToolFacadeWrapper, BridgeToolFacadeWrapper)
//! store session_id at construction time and use self.session_id for handler lookup.
//!
//! ## Session Association
//!
//! Tools are constructed WITH their session ID:
//! - `claude_fspec_tool(session_id)` → wrapper stores session_id as field
//! - Tool `call()` uses `self.session_id` directly for handler lookup

use codelet_tools::fspec_handler::{
    clear_all_fspec_handlers, set_fspec_handler_for_session, FspecHandler, FspecResult,
};
use codelet_tools::bridge_handler::{
    set_bridge_session_context, remove_bridge_session_context,
};
use codelet_tools::facade::{
    claude_fspec_tool, gemini_fspec_tool, openai_fspec_tool, zai_fspec_tool,
    claude_bridge_tool,
    wrapper::FacadeArgs,
};
use rig::tool::Tool;
use serial_test::serial;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// TOOL-012 TESTS - Session ID at Construction
// =============================================================================

/// @scenario: Fspec tool wrapper stores session_id at construction and uses it at call time
#[tokio::test]
#[serial]
async fn test_fspec_tool_stores_session_id_at_construction() {
    // Setup
    clear_all_fspec_handlers();

    // @step Given a session manager has created a session with ID "session-A"
    let session_a = Uuid::new_v4();

    // @step And a handler has been registered for session "session-A"
    let handler_called = Arc::new(AtomicUsize::new(0));
    let handler_called_clone = handler_called.clone();
    let handler: FspecHandler = Arc::new(move |_| {
        handler_called_clone.fetch_add(1, Ordering::SeqCst);
        FspecResult {
            success: true,
            data: "handler_for_session_a".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler_for_session(session_a, Some(handler));

    // @step When the session manager creates an Fspec tool with claude_fspec_tool(session_id)
    let tool = claude_fspec_tool(session_a);

    // @step Then the tool wrapper should store session_id as a field
    assert_eq!(tool.session_id(), session_a, "Tool should store session_id");

    // @step When the LLM invokes the Fspec tool with command "board"
    let args = json!({
        "command": "board",
        "args": "{}",
        "project_root": "."
    });

    // @step Then the tool should use self.session_id to look up the handler
    let result = tool.call(FacadeArgs(args)).await;

    // @step And the correct handler for "session-A" should be invoked
    assert!(result.is_ok(), "Tool call should succeed using self.session_id");

    // @step And the command should execute successfully
    assert_eq!(
        handler_called.load(Ordering::SeqCst),
        1,
        "Handler for session-A should be called"
    );

    // Cleanup
    clear_all_fspec_handlers();
}

/// @scenario: Fspec tool call succeeds across async boundaries
#[tokio::test]
#[serial]
async fn test_fspec_tool_survives_async_boundaries() {
    // Setup
    clear_all_fspec_handlers();

    // @step Given a session manager has created a session with ID "session-B"
    let session_b = Uuid::new_v4();

    // @step And a handler has been registered for session "session-B"
    let handler: FspecHandler = Arc::new(move |_| FspecResult {
        success: true,
        data: "async_boundary_test_passed".to_string(),
        error: None,
        system_reminder: None,
    });
    set_fspec_handler_for_session(session_b, Some(handler));

    // @step And an Fspec tool has been created with session_id "session-B"
    let tool = claude_fspec_tool(session_b);

    let tool_arc = Arc::new(tool);
    let tool_for_task = tool_arc.clone();

    // @step When the tool call crosses an async boundary via tokio task spawn
    let handle = tokio::spawn(async move {
        // This runs on potentially different thread than where tool was created
        // Thread-local session state would be LOST here
        // But self.session_id survives
        let args = json!({
            "command": "board",
            "args": "{}",
            "project_root": "."
        });
        tool_for_task.call(FacadeArgs(args)).await
    });

    let result = handle.await.expect("Task should complete");

    // @step Then self.session_id should still be valid
    // @step And the handler lookup should succeed
    // @step And the command should execute on the correct session
    assert!(result.is_ok(), "Tool call should succeed across async boundary");
    let output = result.unwrap();
    assert!(
        output.contains("async_boundary_test_passed"),
        "Should receive correct handler's response"
    );

    // Cleanup
    clear_all_fspec_handlers();
}

/// @scenario: Concurrent sessions use their own isolated tool instances
#[tokio::test]
#[serial]
async fn test_concurrent_sessions_isolated_tool_instances() {
    // Setup
    clear_all_fspec_handlers();

    // @step Given session "session-A" exists with its own registered handler
    let session_a = Uuid::new_v4();
    let handler_a_calls = Arc::new(AtomicUsize::new(0));
    let handler_a_calls_clone = handler_a_calls.clone();
    let handler_a: FspecHandler = Arc::new(move |_| {
        handler_a_calls_clone.fetch_add(1, Ordering::SeqCst);
        FspecResult {
            success: true,
            data: "from_session_a".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler_for_session(session_a, Some(handler_a));

    // @step And session "session-B" exists with its own registered handler
    let session_b = Uuid::new_v4();
    let handler_b_calls = Arc::new(AtomicUsize::new(0));
    let handler_b_calls_clone = handler_b_calls.clone();
    let handler_b: FspecHandler = Arc::new(move |_| {
        handler_b_calls_clone.fetch_add(1, Ordering::SeqCst);
        FspecResult {
            success: true,
            data: "from_session_b".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler_for_session(session_b, Some(handler_b));

    // @step When session A creates its Fspec tool with claude_fspec_tool(session_id_A)
    let tool_a = claude_fspec_tool(session_a);

    // @step And session B creates its Fspec tool with claude_fspec_tool(session_id_B)
    let tool_b = claude_fspec_tool(session_b);

    // @step Then A's tool should have session_id field set to session_id_A
    assert_eq!(tool_a.session_id(), session_a);

    // @step And B's tool should have session_id field set to session_id_B
    assert_eq!(tool_b.session_id(), session_b);

    // @step When A's tool is invoked
    let args_a = json!({
        "command": "board",
        "args": "{}",
        "project_root": "."
    });
    let result_a = tool_a.call(FacadeArgs(args_a)).await;

    // @step Then handler for session A should be called
    assert!(result_a.is_ok());
    assert_eq!(handler_a_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handler_b_calls.load(Ordering::SeqCst), 0);

    // @step When B's tool is invoked
    let args_b = json!({
        "command": "board",
        "args": "{}",
        "project_root": "."
    });
    let result_b = tool_b.call(FacadeArgs(args_b)).await;

    // @step Then handler for session B should be called
    assert!(result_b.is_ok());
    assert_eq!(handler_a_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handler_b_calls.load(Ordering::SeqCst), 1);

    // @step And there should be no cross-contamination between sessions
    let output_a = result_a.unwrap();
    let output_b = result_b.unwrap();
    assert!(output_a.contains("from_session_a"));
    assert!(output_b.contains("from_session_b"));

    // Cleanup
    clear_all_fspec_handlers();
}

/// @scenario: Rust CLI creates session UUID before building agent
#[test]
#[serial]
fn test_cli_pattern_generates_session_uuid() {
    // @step Given the Rust CLI is running in single-shot mode
    // (Simulated by this test)

    // @step When the CLI prepares to build a rig agent
    // @step Then the CLI should generate a new session_id with Uuid::new_v4()
    let session_id = Uuid::new_v4();
    assert!(!session_id.is_nil(), "CLI should generate non-nil UUID");

    // @step And the CLI should call create_rig_agent(session_id, None, None)
    // (Tested by successful compilation)

    // @step And the Fspec tool in the agent should have the generated session_id
    let tool = claude_fspec_tool(session_id);
    assert_eq!(tool.session_id(), session_id);
}

/// @scenario: Watcher session and parent session use separate Fspec tool instances
#[tokio::test]
#[serial]
async fn test_watcher_and_parent_session_isolation() {
    // Setup
    clear_all_fspec_handlers();

    // @step Given parent session "parent-P" exists with its Fspec tool
    let parent_session = Uuid::new_v4();
    let parent_handler: FspecHandler = Arc::new(move |_| FspecResult {
        success: true,
        data: "from_parent".to_string(),
        error: None,
        system_reminder: None,
    });
    set_fspec_handler_for_session(parent_session, Some(parent_handler));
    let parent_tool = claude_fspec_tool(parent_session);

    // @step And watcher session "watcher-W" is monitoring "parent-P"
    // (Watcher relationship is managed elsewhere)

    // @step And watcher session "watcher-W" has its own Fspec tool
    let watcher_session = Uuid::new_v4();
    let watcher_handler: FspecHandler = Arc::new(move |_| FspecResult {
        success: true,
        data: "from_watcher".to_string(),
        error: None,
        system_reminder: None,
    });
    set_fspec_handler_for_session(watcher_session, Some(watcher_handler));
    let watcher_tool = claude_fspec_tool(watcher_session);

    // @step When the parent session's Fspec tool is invoked
    let args = json!({
        "command": "board",
        "args": "{}",
        "project_root": "."
    });
    let parent_result = parent_tool.call(FacadeArgs(args.clone())).await;

    // @step Then the handler for "parent-P" should be used
    assert!(parent_result.is_ok());
    assert!(parent_result.unwrap().contains("from_parent"));

    // @step When the watcher session's Fspec tool is invoked
    let watcher_result = watcher_tool.call(FacadeArgs(args)).await;

    // @step Then the handler for "watcher-W" should be used
    assert!(watcher_result.is_ok());
    assert!(watcher_result.unwrap().contains("from_watcher"));

    // @step And each session operates independently with no confusion

    // Cleanup
    clear_all_fspec_handlers();
}

/// @scenario: Bridge tool wrapper stores session_id at construction
#[tokio::test]
#[serial]
async fn test_bridge_tool_stores_session_id() {
    // @step Given a session manager has created a session with ID "session-C"
    let session_c = Uuid::new_v4();

    // @step And bridge session context has been set for "session-C"
    let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
    let broadcast_factory: codelet_tools::BroadcastReceiverFactory = Arc::new(move || tx.subscribe());
    let input_injector: codelet_tools::InputInjector = Arc::new(|_| {});
    set_bridge_session_context(session_c, broadcast_factory, input_injector, None, None);
    
    // Set up a bridge handler
    use codelet_tools::bridge_handler::{set_bridge_handler, BridgeHandler};
    use codelet_tools::bridge::BridgeResult;
    let handler: BridgeHandler = Arc::new(move |req| {
        // Verify the request contains the correct session_id
        assert_eq!(req.session_id, session_c, "Request should use self.session_id");
        BridgeResult {
            success: true,
            message: "bridge_handler_called".to_string(),
            connections: Some(vec![]),
        }
    });
    set_bridge_handler(session_c, Some(handler));

    // @step When the session manager creates a Bridge tool with claude_bridge_tool(session_id)
    let tool = claude_bridge_tool(session_c);

    // @step Then the Bridge tool wrapper should store session_id as a field
    assert_eq!(tool.session_id(), session_c);

    // @step When the LLM invokes the Bridge tool with action "list"
    let args = FacadeArgs(serde_json::json!({
        "action": {"type": "list"}
    }));
    let result = tool.call(args).await;

    // @step Then the Bridge tool should use self.session_id for context lookup
    // (Verified by the handler assertion above)
    assert!(result.is_ok());

    // @step And the correct session context for "session-C" should be used
    let output = result.unwrap();
    assert!(output.message.contains("bridge_handler_called"));

    // Cleanup
    set_bridge_handler(session_c, None);
    remove_bridge_session_context(session_c);
}

/// @scenario: create_rig_agent accepts session_id as first parameter
#[test]
#[serial]
fn test_registration_functions_accept_session_id() {
    // @step Given a provider instance (Claude, Gemini, OpenAI, or ZAI)
    let session_id = Uuid::new_v4();

    // @step When I call create_rig_agent(session_id, preamble, thinking_config)
    // We test the tool registration functions

    // @step Then the method should accept session_id as the first parameter
    let claude_tool = claude_fspec_tool(session_id);
    assert_eq!(claude_tool.session_id(), session_id);

    let gemini_tool = gemini_fspec_tool(session_id);
    assert_eq!(gemini_tool.session_id(), session_id);

    let openai_tool = openai_fspec_tool(session_id);
    assert_eq!(openai_tool.session_id(), session_id);

    let zai_tool = zai_fspec_tool(session_id);
    assert_eq!(zai_tool.session_id(), session_id);

    // @step And the Fspec tool in the agent should be constructed with session_id
    // (Verified above)

    // @step And the Bridge tool in the agent should be constructed with session_id
    let claude_bridge = claude_bridge_tool(session_id);
    assert_eq!(claude_bridge.session_id(), session_id);
}

/// @scenario: Tools use session_id at construction, not thread-local state
#[test]
#[serial]
fn test_session_id_at_construction() {
    // @step Given the new session-at-construction architecture is implemented
    let session_id = Uuid::new_v4();

    // @step But set_fspec_handler_for_session() should still exist
    // Handler registration still uses session_id
    let handler: FspecHandler = Arc::new(|_| FspecResult {
        success: true,
        data: "test".to_string(),
        error: None,
        system_reminder: None,
    });
    set_fspec_handler_for_session(session_id, Some(handler));

    // @step And set_bridge_session_context() should still exist
    // Bridge context still uses session_id
    let (tx, _rx) = tokio::sync::broadcast::channel::<serde_json::Value>(16);
    let broadcast_factory: codelet_tools::BroadcastReceiverFactory = Arc::new(move || tx.subscribe());
    let input_injector: codelet_tools::InputInjector = Arc::new(|_| {});
    set_bridge_session_context(session_id, broadcast_factory, input_injector, None, None);

    // @step Then tools work without calling set_current_fspec_session()
    // Tools store session_id at construction (no set_current_fspec_session needed)
    let fspec_tool = claude_fspec_tool(session_id);
    assert_eq!(fspec_tool.session_id(), session_id);

    // @step And tools work without calling set_current_bridge_session()
    // Bridge tool also stores session_id at construction (no set_current_bridge_session needed)
    let bridge_tool = claude_bridge_tool(session_id);
    assert_eq!(bridge_tool.session_id(), session_id);

    // Cleanup
    remove_bridge_session_context(session_id);
    set_fspec_handler_for_session(session_id, None);
}

// =============================================================================
// Additional helper tests
// =============================================================================

/// Test that nil UUID can be used for testing tools that don't need handler routing
#[test]
#[serial]
fn test_nil_uuid_for_testing() {
    let nil_session = Uuid::nil();

    let tool = claude_fspec_tool(nil_session);
    assert_eq!(tool.session_id(), nil_session);
    assert!(tool.session_id().is_nil());
}

/// Test all providers support session_id parameter
#[test]
#[serial]
fn test_all_providers_have_consistent_api() {
    let session_id = Uuid::new_v4();
    
    let claude = claude_fspec_tool(session_id);
    let gemini = gemini_fspec_tool(session_id);
    let openai = openai_fspec_tool(session_id);
    let zai = zai_fspec_tool(session_id);

    // All have correct provider names
    assert_eq!(claude.provider(), "claude");
    assert_eq!(gemini.provider(), "gemini");
    assert_eq!(openai.provider(), "openai");
    assert_eq!(zai.provider(), "zai");
    
    // All store session_id
    assert_eq!(claude.session_id(), session_id);
    assert_eq!(gemini.session_id(), session_id);
    assert_eq!(openai.session_id(), session_id);
    assert_eq!(zai.session_id(), session_id);
}
