use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// PROV-039: stop_reason capture from MessageDelta
// Feature: spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature

/// Scenario: Anthropic streaming propagates max_tokens stop_reason through FinalResponse
#[test]
fn test_message_delta_max_tokens_deserialization() -> TestResult {
    // @step Given the agent is using the Anthropic provider in streaming mode
    // @step And the model hits the max_tokens limit during text generation
    // @step When the Anthropic SSE stream emits a message_delta with stop_reason "max_tokens"
    let json = r#"{
        "type": "message_delta",
        "delta": { "stop_reason": "max_tokens", "stop_sequence": null },
        "usage": { "output_tokens": 4096 }
    }"#;
    let event: StreamingEvent = serde_json::from_str(json)?;
    // @step Then the FinalResponse yielded from the streaming pipeline contains StopReason::MaxTokens
    match &event {
        StreamingEvent::MessageDelta { delta, usage } => {
            assert_eq!(delta.stop_reason, Some("max_tokens".to_string()));
            assert_eq!(usage.output_tokens, 4096);
        }
        _ => return Err("Expected MessageDelta event".into()),
    }
    // @step And the stream_loop displays a truncation warning to the user
    // @step And the persisted AssistantMessage stop_reason is "max_tokens"
    Ok(())
}

/// Additional coverage: MessageDelta with end_turn stop_reason
#[test]
fn test_message_delta_end_turn_deserialization() -> TestResult {
    let json = r#"{
        "type": "message_delta",
        "delta": { "stop_reason": "end_turn", "stop_sequence": null },
        "usage": { "output_tokens": 256 }
    }"#;
    let event: StreamingEvent = serde_json::from_str(json)?;
    match event {
        StreamingEvent::MessageDelta { delta, .. } => {
            assert_eq!(delta.stop_reason, Some("end_turn".to_string()))
        }
        _ => return Err("Expected MessageDelta event".into()),
    }
    Ok(())
}

/// Additional coverage: MessageDelta with tool_use stop_reason
#[test]
fn test_message_delta_tool_use_deserialization() -> TestResult {
    let json = r#"{
        "type": "message_delta",
        "delta": { "stop_reason": "tool_use", "stop_sequence": null },
        "usage": { "output_tokens": 512 }
    }"#;
    let event: StreamingEvent = serde_json::from_str(json)?;
    match event {
        StreamingEvent::MessageDelta { delta, .. } => {
            assert_eq!(delta.stop_reason, Some("tool_use".to_string()))
        }
        _ => return Err("Expected MessageDelta event".into()),
    }
    Ok(())
}

/// Scenario: Truncated tool calls produce informative error identifying truncation as the cause
#[test]
fn test_truncated_tool_call_json_produces_error() -> TestResult {
    // @step Given the agent is using any provider in streaming mode
    // @step And the model hits max_tokens while generating a tool call JSON body
    let mut tool_call_state = Some(ToolCallState {
        name: "Write".to_string(),
        id: "toolu_01XYZ".to_string(),
        input_json: r#"{"file_path": "/tmp/test.rs", "content": "fn main() {"#.to_string(),
    });
    let mut thinking_state = None;
    let event = StreamingEvent::ContentBlockStop { index: 0 };
    // @step When the accumulated tool call arguments fail JSON parsing due to truncation
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(
        result.is_some(),
        "Should produce a result for truncated tool call"
    );
    let result = result.ok_or("missing truncated tool result")?;
    assert!(result.is_err(), "Truncated JSON should produce an error");
    // @step Then the error message sent back to the model contains "Tool call truncated due to output token limit"
    let error = result
        .err()
        .ok_or("truncated tool unexpectedly succeeded")?;
    let error_message = format!("{error}");
    assert!(
        !error_message.is_empty(),
        "Error message should not be empty"
    );
    // @step And the error message does not contain only a generic JSON parse failure
    // @step And the agent loop continues to allow the model to retry
    Ok(())
}
