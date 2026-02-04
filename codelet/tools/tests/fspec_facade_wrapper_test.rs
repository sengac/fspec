#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
//! CODE-009: FspecToolFacadeWrapper tests for structured command request emission
//!
//! These tests verify that FspecToolFacadeWrapper emits the correct JSON structure
//! with __fspec_request__ marker instead of the old FSPEC_INTERCEPT string pattern.

use codelet_tools::facade::{
    claude_fspec_tool, gemini_fspec_tool,
    wrapper::FacadeArgs,
};
use rig::tool::Tool;
use serde_json::{json, Value};

// ============================================================================
// Scenario: FspecTool emits structured command request and receives typed result
// ============================================================================

#[tokio::test]
async fn test_fspec_tool_wrapper_emits_structured_json_marker() {
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FspecTool emits structured command request and receives typed result
    
    // @step Given a codelet session with FspecTool available
    let wrapper = claude_fspec_tool();
    
    // @step And the StreamChunk type includes FspecCommandRequest and FspecCommandResult variants
    // (verified by type system - FspecRequest/FspecResult exist in types.rs)
    
    // @step When the LLM invokes Fspec tool with command "show-work-unit" and args '{"id":"CODE-001"}'
    let args = FacadeArgs(json!({
        "command": "show-work-unit",
        "args": "{\"id\":\"CODE-001\"}",
        "project_root": "/test/project"
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    
    // @step Then Rust should emit a FspecCommandRequest chunk with typed fields
    let parsed: Value = serde_json::from_str(&result).expect("result should be valid JSON");
    
    // Verify the __fspec_request__ marker is present (critical for session layer detection)
    assert_eq!(
        parsed.get("__fspec_request__").and_then(|v| v.as_bool()),
        Some(true),
        "Result must have __fspec_request__: true marker"
    );
    
    // @step And TypeScript should receive the chunk with direct field access via chunk.fspecRequest.command
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("show-work-unit"),
        "command field should be directly accessible"
    );
    
    // Verify argsJson field is present and correct
    assert_eq!(
        parsed.get("argsJson").and_then(|v| v.as_str()),
        Some("{\"id\":\"CODE-001\"}"),
        "argsJson field should be directly accessible"
    );
    
    // Verify projectRoot field is present and correct
    assert_eq!(
        parsed.get("projectRoot").and_then(|v| v.as_str()),
        Some("/test/project"),
        "projectRoot field should be directly accessible"
    );
    
    // Verify provider field is present
    assert_eq!(
        parsed.get("provider").and_then(|v| v.as_str()),
        Some("claude"),
        "provider field should identify the facade provider"
    );
}

#[tokio::test]
async fn test_fspec_tool_wrapper_with_gemini_facade() {
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FspecTool emits structured command request and receives typed result
    
    // @step Given a codelet session with FspecTool available
    let wrapper = gemini_fspec_tool();
    
    // @step When the LLM invokes Fspec tool with command "list-work-units" and args '{}'
    // Gemini uses different parameter names (args instead of argsJson)
    let args = FacadeArgs(json!({
        "command": "list-work-units",
        "args": "{}",
        "project_root": "."
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    
    // @step Then Rust should emit a FspecCommandRequest chunk with typed fields
    let parsed: Value = serde_json::from_str(&result).expect("result should be valid JSON");
    
    assert_eq!(
        parsed.get("__fspec_request__").and_then(|v| v.as_bool()),
        Some(true),
        "Result must have __fspec_request__: true marker"
    );
    
    assert_eq!(
        parsed.get("command").and_then(|v| v.as_str()),
        Some("list-work-units")
    );
    
    assert_eq!(
        parsed.get("provider").and_then(|v| v.as_str()),
        Some("gemini")
    );
}

// ============================================================================
// Scenario: TypeScript handles FspecCommandRequest with type-safe field access
// ============================================================================

#[tokio::test]
async fn test_fspec_request_json_has_all_required_fields_for_typescript() {
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: TypeScript handles FspecCommandRequest with type-safe field access
    
    // @step Given a codelet session processing StreamChunk events
    let wrapper = claude_fspec_tool();
    
    // @step When a FspecCommandRequest chunk is received
    let args = FacadeArgs(json!({
        "command": "create-story",
        "args": "{\"prefix\":\"TEST\",\"title\":\"Test Story\"}",
        "project_root": "/projects/test"
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    let parsed: Value = serde_json::from_str(&result).expect("result should be valid JSON");
    
    // @step Then TypeScript should access chunk.fspecRequest.command directly without string parsing
    assert!(
        parsed.get("command").is_some(),
        "command field must exist for direct TypeScript access"
    );
    
    // @step And TypeScript should access chunk.fspecRequest.argsJson directly without regex extraction
    assert!(
        parsed.get("argsJson").is_some(),
        "argsJson field must exist for direct TypeScript access"
    );
    
    // @step And TypeScript should access chunk.fspecRequest.projectRoot directly without field parsing
    assert!(
        parsed.get("projectRoot").is_some(),
        "projectRoot field must exist for direct TypeScript access"
    );
    
    // Verify NO string parsing is needed by confirming all values are already extracted
    let command = parsed.get("command").and_then(|v| v.as_str()).unwrap();
    let args_json = parsed.get("argsJson").and_then(|v| v.as_str()).unwrap();
    let project_root = parsed.get("projectRoot").and_then(|v| v.as_str()).unwrap();
    
    assert_eq!(command, "create-story");
    assert_eq!(args_json, "{\"prefix\":\"TEST\",\"title\":\"Test Story\"}");
    assert_eq!(project_root, "/projects/test");
}

// ============================================================================
// Scenario: FSPEC_INTERCEPT string pattern is removed after migration
// ============================================================================

#[tokio::test]
async fn test_fspec_wrapper_does_not_emit_fspec_intercept_string() {
    // Feature: spec/features/structured-fspectool-results-via-streamchunk-discriminated-union.feature
    // Scenario: FSPEC_INTERCEPT string pattern is removed after migration
    
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
    
    // Verify result is valid JSON with the new marker pattern
    let parsed: Value = serde_json::from_str(&result)
        .expect("Result must be valid JSON, not FSPEC_INTERCEPT string");
    
    assert!(
        parsed.get("__fspec_request__").is_some(),
        "Result must use new __fspec_request__ JSON marker"
    );
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[tokio::test]
async fn test_fspec_wrapper_handles_empty_args() {
    // Test edge case: empty args should still produce valid JSON
    let wrapper = claude_fspec_tool();
    
    let args = FacadeArgs(json!({
        "command": "list-work-units",
        "args": "",
        "project_root": "."
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    let parsed: Value = serde_json::from_str(&result).expect("result should be valid JSON");
    
    assert_eq!(
        parsed.get("__fspec_request__").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        parsed.get("argsJson").and_then(|v| v.as_str()),
        Some("")
    );
}

#[tokio::test]
async fn test_fspec_wrapper_handles_special_characters_in_args() {
    // Test edge case: special characters in args should be preserved
    let wrapper = claude_fspec_tool();
    
    let special_args = r#"{"title":"Test with 'quotes' and \"escaped\""}"#;
    let args = FacadeArgs(json!({
        "command": "create-story",
        "args": special_args,
        "project_root": "."
    }));
    
    let result = wrapper.call(args).await.expect("call should succeed");
    let parsed: Value = serde_json::from_str(&result).expect("result should be valid JSON");
    
    assert_eq!(
        parsed.get("argsJson").and_then(|v| v.as_str()),
        Some(special_args),
        "Special characters in args should be preserved"
    );
}
