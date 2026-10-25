use super::*;
use crate::streaming::{RawStreamingChoice, RawStreamingToolCall, ToolCallDeltaContent};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_thinking_delta_does_not_interfere_with_tool_calls() -> TestResult {
    let event = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::ThinkingDelta {
            thinking: "Thinking while tool is active...".to_string(),
        },
    };
    let mut tool_call_state = Some(ToolCallState {
        name: "test_tool".to_string(),
        id: "tool_123".to_string(),
        input_json: String::new(),
    });
    let mut thinking_state = None;
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(result.is_some());
    let choice = result.ok_or("missing tool reasoning")??;
    match choice {
        RawStreamingChoice::ReasoningDelta { reasoning, .. } => {
            assert_eq!(reasoning, "Thinking while tool is active...")
        }
        _ => return Err("Expected ReasoningDelta choice".into()),
    }
    assert!(tool_call_state.is_some());
    Ok(())
}

#[test]
fn test_handle_input_json_delta_event() -> TestResult {
    let event = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::InputJsonDelta {
            partial_json: "{\"arg\":\"value".to_string(),
        },
    };
    let mut tool_call_state = Some(ToolCallState {
        name: "test_tool".to_string(),
        id: "tool_123".to_string(),
        input_json: String::new(),
    });
    let mut thinking_state = None;
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(result.is_some());
    let choice = result.ok_or("missing JSON delta")??;
    match choice {
        RawStreamingChoice::ToolCallDelta { id, content } => {
            assert_eq!(id, "tool_123");
            match content {
                ToolCallDeltaContent::Delta(delta) => assert_eq!(delta, "{\"arg\":\"value"),
                _ => return Err("Expected Delta content".into()),
            }
        }
        _ => return Err(format!("Expected ToolCallDelta choice, got {choice:?}").into()),
    }
    assert!(tool_call_state.is_some());
    let state = tool_call_state.ok_or("missing tool state")?;
    assert_eq!(state.input_json, "{\"arg\":\"value");
    Ok(())
}

#[test]
fn test_tool_call_accumulation_with_multiple_deltas() -> TestResult {
    let mut tool_call_state = Some(ToolCallState {
        name: "test_tool".to_string(),
        id: "tool_123".to_string(),
        input_json: String::new(),
    });
    let mut thinking_state = None;
    let event1 = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::InputJsonDelta {
            partial_json: "{\"location\":".to_string(),
        },
    };
    let result1 = handle_event(&event1, &mut tool_call_state, &mut thinking_state);
    assert!(result1.is_some());
    let event2 = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::InputJsonDelta {
            partial_json: "\"Paris\",".to_string(),
        },
    };
    let result2 = handle_event(&event2, &mut tool_call_state, &mut thinking_state);
    assert!(result2.is_some());
    let event3 = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::InputJsonDelta {
            partial_json: "\"temp\":\"20C\"}".to_string(),
        },
    };
    let result3 = handle_event(&event3, &mut tool_call_state, &mut thinking_state);
    assert!(result3.is_some());
    assert!(tool_call_state.is_some());
    let state = tool_call_state
        .as_ref()
        .ok_or("missing accumulated tool state")?;
    assert_eq!(
        state.input_json,
        "{\"location\":\"Paris\",\"temp\":\"20C\"}"
    );
    let stop_event = StreamingEvent::ContentBlockStop { index: 0 };
    let final_result = handle_event(&stop_event, &mut tool_call_state, &mut thinking_state);
    assert!(final_result.is_some());
    match final_result.ok_or("missing completed tool")?? {
        RawStreamingChoice::ToolCall(RawStreamingToolCall {
            id,
            name,
            arguments,
            ..
        }) => {
            assert_eq!(id, "tool_123");
            assert_eq!(name, "test_tool");
            assert_eq!(
                arguments
                    .get("location")
                    .ok_or("missing location")?
                    .as_str()
                    .ok_or("location not text")?,
                "Paris"
            );
            assert_eq!(
                arguments
                    .get("temp")
                    .ok_or("missing temp")?
                    .as_str()
                    .ok_or("temp not text")?,
                "20C"
            );
        }
        other => return Err(format!("Expected ToolCall, got {other:?}").into()),
    }
    assert!(tool_call_state.is_none());
    Ok(())
}
