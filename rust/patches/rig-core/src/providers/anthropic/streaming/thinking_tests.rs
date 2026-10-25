use super::*;
use crate::streaming::RawStreamingChoice;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_thinking_delta_deserialization() -> TestResult {
    let json = r#"{"type": "thinking_delta", "thinking": "Let me think about this..."}"#;
    let delta: ContentDelta = serde_json::from_str(json)?;
    match delta {
        ContentDelta::ThinkingDelta { thinking } => {
            assert_eq!(thinking, "Let me think about this...")
        }
        _ => return Err("Expected ThinkingDelta variant".into()),
    }
    Ok(())
}

#[test]
fn test_signature_delta_deserialization() -> TestResult {
    let json = r#"{"type": "signature_delta", "signature": "abc123def456"}"#;
    let delta: ContentDelta = serde_json::from_str(json)?;
    match delta {
        ContentDelta::SignatureDelta { signature } => assert_eq!(signature, "abc123def456"),
        _ => return Err("Expected SignatureDelta variant".into()),
    }
    Ok(())
}

#[test]
fn test_thinking_delta_streaming_event_deserialization() -> TestResult {
    let json = r#"{
        "type": "content_block_delta",
        "index": 0,
        "delta": {
            "type": "thinking_delta",
            "thinking": "First, I need to understand the problem."
        }
    }"#;
    let event: StreamingEvent = serde_json::from_str(json)?;
    match event {
        StreamingEvent::ContentBlockDelta { index, delta } => {
            assert_eq!(index, 0);
            match delta {
                ContentDelta::ThinkingDelta { thinking } => {
                    assert_eq!(thinking, "First, I need to understand the problem.")
                }
                _ => return Err("Expected ThinkingDelta".into()),
            }
        }
        _ => return Err("Expected ContentBlockDelta event".into()),
    }
    Ok(())
}

#[test]
fn test_signature_delta_streaming_event_deserialization() -> TestResult {
    let json = r#"{
        "type": "content_block_delta",
        "index": 0,
        "delta": {
            "type": "signature_delta",
            "signature": "ErUBCkYICBgCIkCaGbqC85F4"
        }
    }"#;
    let event: StreamingEvent = serde_json::from_str(json)?;
    match event {
        StreamingEvent::ContentBlockDelta { index, delta } => {
            assert_eq!(index, 0);
            match delta {
                ContentDelta::SignatureDelta { signature } => {
                    assert_eq!(signature, "ErUBCkYICBgCIkCaGbqC85F4")
                }
                _ => return Err("Expected SignatureDelta".into()),
            }
        }
        _ => return Err("Expected ContentBlockDelta event".into()),
    }
    Ok(())
}

#[test]
fn test_handle_thinking_delta_event() -> TestResult {
    let event = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::ThinkingDelta {
            thinking: "Analyzing the request...".to_string(),
        },
    };
    let mut tool_call_state = None;
    let mut thinking_state = None;
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(result.is_some());
    let choice = result.ok_or("missing thinking result")??;
    match choice {
        RawStreamingChoice::ReasoningDelta { id, reasoning, .. } => {
            assert_eq!(id, None);
            assert_eq!(reasoning, "Analyzing the request...");
        }
        _ => return Err("Expected ReasoningDelta choice".into()),
    }
    assert!(thinking_state.is_some());
    assert_eq!(
        thinking_state.ok_or("missing thinking state")?.thinking,
        "Analyzing the request..."
    );
    Ok(())
}

#[test]
fn test_handle_signature_delta_event() -> TestResult {
    let event = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::SignatureDelta {
            signature: "test_signature".to_string(),
        },
    };
    let mut tool_call_state = None;
    let mut thinking_state = None;
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(result.is_none());
    assert!(thinking_state.is_some());
    assert_eq!(
        thinking_state.ok_or("missing signature state")?.signature,
        "test_signature"
    );
    Ok(())
}

#[test]
fn test_handle_text_delta_event() -> TestResult {
    let event = StreamingEvent::ContentBlockDelta {
        index: 0,
        delta: ContentDelta::TextDelta {
            text: "Hello, world!".to_string(),
        },
    };
    let mut tool_call_state = None;
    let mut thinking_state = None;
    let result = handle_event(&event, &mut tool_call_state, &mut thinking_state);
    assert!(result.is_some());
    let choice = result.ok_or("missing text result")??;
    match choice {
        RawStreamingChoice::Message(text) => assert_eq!(text, "Hello, world!"),
        _ => return Err("Expected Message choice".into()),
    }
    Ok(())
}
