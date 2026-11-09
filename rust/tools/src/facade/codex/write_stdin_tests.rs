//! Feature: spec/features/codex-write-stdin-facade.feature
//!
//! BUG-115 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// BUG-115: write_stdin facade tests
// Feature: spec/features/codex-write-stdin-facade.feature
// =========================================================================

/// Scenario: CodexWriteStdinFacade maps non-empty chars to InternalExecParams::Write
#[test]
fn test_codex_write_stdin_maps_non_empty_chars_to_write() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id "a1b2c3d4" and chars "print(42)\n"
    let input = json!({
        "session_id": "a1b2c3d4",
        "chars": "print(42)\n"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Write with session_id "a1b2c3d4" and input "print(42)\n"
    // @step And yield_time_ms is None
    // @step And max_output_tokens is None
    assert_eq!(
        result,
        InternalExecParams::Write {
            session_id: "a1b2c3d4".to_string(),
            input: "print(42)\n".to_string(),
            yield_time_ms: None,
            max_output_tokens: None,
        }
    );
}

/// Scenario: CodexWriteStdinFacade passes all optional params through
#[test]
fn test_codex_write_stdin_passes_optional_params() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id "a1b2c3d4" chars "exit()\n" yield_time_ms 5000 and max_output_tokens 1024
    let input = json!({
        "session_id": "a1b2c3d4",
        "chars": "exit()\n",
        "yield_time_ms": 5000,
        "max_output_tokens": 1024
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Write with session_id "a1b2c3d4" and input "exit()\n"
    // @step And yield_time_ms is 5000
    // @step And max_output_tokens is 1024
    assert_eq!(
        result,
        InternalExecParams::Write {
            session_id: "a1b2c3d4".to_string(),
            input: "exit()\n".to_string(),
            yield_time_ms: Some(5000),
            max_output_tokens: Some(1024),
        }
    );
}

/// Scenario: CodexWriteStdinFacade maps empty chars to InternalExecParams::Poll
#[test]
fn test_codex_write_stdin_maps_empty_chars_to_poll() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id "a1b2c3d4" and chars ""
    let input = json!({
        "session_id": "a1b2c3d4",
        "chars": ""
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Poll with session_id "a1b2c3d4"
    // @step And yield_time_ms is None
    // @step And max_output_tokens is None
    assert_eq!(
        result,
        InternalExecParams::Poll {
            session_id: "a1b2c3d4".to_string(),
            yield_time_ms: None,
            max_output_tokens: None,
        }
    );
}

/// Scenario: CodexWriteStdinFacade maps absent chars to InternalExecParams::Poll
#[test]
fn test_codex_write_stdin_maps_absent_chars_to_poll() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id "a1b2c3d4" and no chars field
    let input = json!({
        "session_id": "a1b2c3d4"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Poll with session_id "a1b2c3d4"
    assert_eq!(
        result,
        InternalExecParams::Poll {
            session_id: "a1b2c3d4".to_string(),
            yield_time_ms: None,
            max_output_tokens: None,
        }
    );
}

/// Scenario: CodexWriteStdinFacade accepts string session_id directly
#[test]
fn test_codex_write_stdin_accepts_string_session_id() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id "05e8aa9c" and chars "hello"
    let input = json!({
        "session_id": "05e8aa9c",
        "chars": "hello"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Write with session_id "05e8aa9c" and input "hello"
    assert_eq!(
        result,
        InternalExecParams::Write {
            session_id: "05e8aa9c".to_string(),
            input: "hello".to_string(),
            yield_time_ms: None,
            max_output_tokens: None,
        }
    );
}

/// Scenario: CodexWriteStdinFacade rejects numeric session_id
#[test]
fn test_codex_write_stdin_rejects_numeric_session_id() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id 99 (number)
    let input = json!({
        "session_id": 99,
        "chars": "hello"
    });
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error mentioning "session_id must be a string"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "write_stdin");
        assert!(message.contains("session_id must be a string"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexWriteStdinFacade validates required session_id parameter
#[test]
fn test_codex_write_stdin_validates_required_session_id() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin without session_id
    let input = json!({
        "chars": "hello"
    });
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "write_stdin");
        assert!(message.contains("session_id"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexWriteStdinFacade rejects null session_id
#[test]
fn test_codex_write_stdin_rejects_null_session_id() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the Codex model calls write_stdin with session_id null
    let input = json!({
        "session_id": null
    });
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "write_stdin");
        assert!(message.contains("session_id"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexWriteStdinFacade schema has additionalProperties false
#[test]
fn test_codex_write_stdin_schema() {
    // @step Given a CodexWriteStdinFacade instance
    let facade = CodexWriteStdinFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has additionalProperties set to false
    assert_eq!(def.parameters["additionalProperties"], false);

    // @step And the schema has "session_id" in the required array
    assert_eq!(def.parameters["required"], json!(["session_id"]));

    // @step And the tool name is "write_stdin"
    assert_eq!(def.name, "write_stdin");
    assert_eq!(facade.tool_name(), "write_stdin");

    // @step And the provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: write_stdin facade is registered in Codex create_rig_agent
#[test]
fn test_codex_write_stdin_facade_tool_name_and_schema() {
    // @step Given a CodexWriteStdinFacade instance registered via ExecToolFacadeWrapper
    let facade = CodexWriteStdinFacade;

    // @step When the Codex agent tool list is inspected
    let def = facade.definition();

    // @step Then the tool list contains "write_stdin"
    assert_eq!(def.name, "write_stdin");
    assert!(def.parameters["properties"]["session_id"].is_object());
    assert!(def.parameters["properties"]["chars"].is_object());
    assert!(def.parameters["properties"]["yield_time_ms"].is_object());
    assert!(def.parameters["properties"]["max_output_tokens"].is_object());
}
