#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
//! CODE-009: FspecToolFacadeWrapper tests for structured command request emission
//!
//! These tests verify that FspecToolFacadeWrapper emits the correct JSON structure
//! with __fspec_request__ marker instead of the old FSPEC_INTERCEPT string pattern.
//!
//! NOTE: These tests require a mock fspec handler to be configured since the wrapper
//! now calls execute_fspec_command which needs TypeScript integration.
//!
//! Tests are serialized using a static mutex because they modify global state (the handler).

use codelet_tools::facade::{
    claude_fspec_tool, gemini_fspec_tool,
    wrapper::FacadeArgs,
};
use codelet_tools::fspec_handler::FspecResult;
use codelet_tools::{set_fspec_handler, FspecHandler};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Mutex to ensure tests run serially (they modify global fspec handler state)
static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Helper to set up a mock fspec handler that returns a mock result
fn setup_mock_handler() {
    let handler: FspecHandler = Arc::new(|req| {
        // Return mock result - actual tests verify the request structure via handler
        FspecResult {
            success: true,
            data: format!("Mock result for command: {}", req.command),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
}

/// Helper to clean up handler after test
fn cleanup_handler() {
    set_fspec_handler(None);
}

// ============================================================================
// Scenario: FspecTool calls handler with correct request structure
// ============================================================================

#[tokio::test]
async fn test_fspec_tool_wrapper_emits_structured_json_marker() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FspecTool emits structured command request and receives typed result
    
    // Setup mock handler that captures the request
    let captured_command = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_args = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_project_root = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    
    let cmd_clone = captured_command.clone();
    let args_clone = captured_args.clone();
    let root_clone = captured_project_root.clone();
    let prov_clone = captured_provider.clone();
    
    let handler: FspecHandler = Arc::new(move |req| {
        *cmd_clone.lock().unwrap() = req.command.clone();
        *args_clone.lock().unwrap() = req.args_json.clone();
        *root_clone.lock().unwrap() = req.project_root.clone();
        *prov_clone.lock().unwrap() = req.provider;
        FspecResult {
            success: true,
            data: "Test result".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
    
    // @step Given a codelet session with FspecTool available
    let wrapper = claude_fspec_tool();
    
    // @step When the LLM invokes Fspec tool with command "show-work-unit" and args '{"id":"CODE-001"}'
    let args = FacadeArgs(json!({
        "command": "show-work-unit",
        "args": "{\"id\":\"CODE-001\"}",
        "project_root": "/test/project"
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    
    // @step Then the handler should receive the request with correct fields
    assert_eq!(*captured_command.lock().unwrap(), "show-work-unit");
    assert_eq!(*captured_args.lock().unwrap(), "{\"id\":\"CODE-001\"}");
    assert_eq!(*captured_project_root.lock().unwrap(), "/test/project");
    assert_eq!(*captured_provider.lock().unwrap(), "claude");
    
    // Result is the handler's response
    assert_eq!(result, "Test result");
    
    cleanup_handler();
}

#[tokio::test]
async fn test_fspec_tool_wrapper_with_gemini_facade() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FspecTool emits structured command request and receives typed result
    
    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_command = Arc::new(std::sync::Mutex::new(String::new()));
    let prov_clone = captured_provider.clone();
    let cmd_clone = captured_command.clone();
    
    let handler: FspecHandler = Arc::new(move |req| {
        *prov_clone.lock().unwrap() = req.provider;
        *cmd_clone.lock().unwrap() = req.command;
        FspecResult {
            success: true,
            data: "Gemini result".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
    
    // @step Given a codelet session with FspecTool available
    let wrapper = gemini_fspec_tool();
    
    // @step When the LLM invokes Fspec tool with command "list-work-units" and args '{}'
    let args = FacadeArgs(json!({
        "command": "list-work-units",
        "args": "{}",
        "project_root": "."
    }));
    
    let _result = wrapper.call(args).await.expect("call should succeed");
    
    // Verify Gemini facade identifies itself correctly
    assert_eq!(*captured_provider.lock().unwrap(), "gemini");
    assert_eq!(*captured_command.lock().unwrap(), "list-work-units");
    
    cleanup_handler();
}

// ============================================================================
// Scenario: TypeScript handles FspecCommandRequest with type-safe field access
// ============================================================================

#[tokio::test]
async fn test_fspec_request_json_has_all_required_fields_for_typescript() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: TypeScript handles FspecCommandRequest with type-safe field access
    
    let captured_command = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_args = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_project_root = Arc::new(std::sync::Mutex::new(String::new()));
    
    let cmd_clone = captured_command.clone();
    let args_clone = captured_args.clone();
    let root_clone = captured_project_root.clone();
    
    let handler: FspecHandler = Arc::new(move |req| {
        *cmd_clone.lock().unwrap() = req.command;
        *args_clone.lock().unwrap() = req.args_json;
        *root_clone.lock().unwrap() = req.project_root;
        FspecResult {
            success: true,
            data: "Result".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
    
    // @step Given a codelet session processing StreamChunk events
    let wrapper = claude_fspec_tool();
    
    // @step When a FspecCommandRequest chunk is received
    let args = FacadeArgs(json!({
        "command": "create-story",
        "args": "{\"prefix\":\"TEST\",\"title\":\"Test Story\"}",
        "project_root": "/projects/test"
    }));
    
    let _result = wrapper.call(args).await.expect("call should succeed");
    
    // @step Then handler should receive all fields for direct TypeScript access
    assert_eq!(*captured_command.lock().unwrap(), "create-story");
    assert_eq!(*captured_args.lock().unwrap(), "{\"prefix\":\"TEST\",\"title\":\"Test Story\"}");
    assert_eq!(*captured_project_root.lock().unwrap(), "/projects/test");
    
    cleanup_handler();
}

// ============================================================================
// Scenario: FSPEC_INTERCEPT string pattern is removed after migration
// ============================================================================

#[tokio::test]
async fn test_fspec_wrapper_does_not_emit_fspec_intercept_string() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FSPEC_INTERCEPT string pattern is removed after migration
    
    setup_mock_handler();
    
    // @step Given the structured StreamChunk flow is implemented for fspec commands
    let wrapper = claude_fspec_tool();
    
    // @step When all fspec tool calls use FspecCommandRequest and FspecCommandResult
    let args = FacadeArgs(json!({
        "command": "show-work-unit",
        "args": "{\"id\":\"CODE-001\"}",
        "project_root": "."
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    
    // @step Then the FSPEC_INTERCEPT string pattern should be removed from wrapper.rs
    assert!(
        !result.contains("FSPEC_INTERCEPT"),
        "Result must NOT contain FSPEC_INTERCEPT string pattern"
    );
    
    cleanup_handler();
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[tokio::test]
async fn test_fspec_wrapper_handles_empty_args() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Test edge case: empty args should still work
    let captured_args = Arc::new(std::sync::Mutex::new(String::new()));
    let args_clone = captured_args.clone();
    
    let handler: FspecHandler = Arc::new(move |req| {
        *args_clone.lock().unwrap() = req.args_json;
        FspecResult {
            success: true,
            data: "Result".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
    
    let wrapper = claude_fspec_tool();
    
    let args = FacadeArgs(json!({
        "command": "list-work-units",
        "args": "",
        "project_root": "."
    }));
    
    let _result = wrapper.call(args).await.expect("call should succeed");
    
    assert_eq!(*captured_args.lock().unwrap(), "");
    
    cleanup_handler();
}

#[tokio::test]
async fn test_fspec_wrapper_handles_special_characters_in_args() {
    let _lock = TEST_MUTEX.lock().unwrap();
    
    // Test edge case: special characters in args should be preserved
    let captured_args = Arc::new(std::sync::Mutex::new(String::new()));
    let args_clone = captured_args.clone();
    
    let handler: FspecHandler = Arc::new(move |req| {
        *args_clone.lock().unwrap() = req.args_json;
        FspecResult {
            success: true,
            data: "Result".to_string(),
            error: None,
            system_reminder: None,
        }
    });
    set_fspec_handler(Some(handler));
    
    let wrapper = claude_fspec_tool();
    
    let special_args = r#"{"title":"Test with 'quotes' and \"escaped\""}"#;
    let args = FacadeArgs(json!({
        "command": "create-story",
        "args": special_args,
        "project_root": "."
    }));
    
    let _result = wrapper.call(args).await.expect("call should succeed");
    
    assert_eq!(
        *captured_args.lock().unwrap(),
        special_args,
        "Special characters in args should be preserved"
    );
    
    cleanup_handler();
}
