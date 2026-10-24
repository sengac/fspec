use super::fixture::{TestResult, SUFFIX};
use serde_json::{json, Value};

pub fn assert_chat(wire: &Value) -> TestResult {
    use rig::providers::openai::completion::{Message, ToolResultContentValue, ToolType};
    let messages: Vec<Message> = serde_json::from_value(wire["messages"].clone())?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, message) in messages.iter().enumerate() {
        match message {
            Message::Assistant { tool_calls, .. } => {
                for call in tool_calls {
                    if call.id == "call_after_snapshot" {
                        assert_eq!(call.r#type, ToolType::Function);
                        assert_eq!(call.function.name, "lookup");
                        assert_eq!(call.function.arguments, json!({"path":"src/deep.rs"}));
                        calls.push(position);
                    }
                }
            }
            Message::ToolResult {
                tool_call_id,
                content,
            } if tool_call_id == "call_after_snapshot" => {
                match content {
                    ToolResultContentValue::String(text) => assert_eq!(text, SUFFIX),
                    ToolResultContentValue::Array(parts) => {
                        assert_eq!(parts.len(), 1);
                        assert_eq!(parts[0].text, SUFFIX);
                    }
                }
                results.push(position);
            }
            _ => {}
        }
    }
    assert_pair(&calls, &results)
}

pub fn assert_anthropic(wire: &Value) -> TestResult {
    use rig::providers::anthropic::completion::{Content, Message, Role, ToolResultContent};
    let messages: Vec<Message> = serde_json::from_value(wire["messages"].clone())?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, message) in messages.iter().enumerate() {
        for part in message.content.iter() {
            match part {
                Content::ToolUse { id, name, input } if id == "call_after_snapshot" => {
                    assert_eq!(message.role, Role::Assistant);
                    assert_eq!(name, "lookup");
                    assert_eq!(input, &json!({"path":"src/deep.rs"}));
                    calls.push(position);
                }
                Content::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                    ..
                } if tool_use_id == "call_after_snapshot" => {
                    assert_eq!(message.role, Role::User);
                    assert_ne!(*is_error, Some(true));
                    assert_eq!(content.len(), 1);
                    assert_eq!(
                        content.first(),
                        ToolResultContent::Text {
                            text: SUFFIX.into()
                        }
                    );
                    results.push(position);
                }
                _ => {}
            }
        }
    }
    assert_pair(&calls, &results)
}

pub fn assert_custom(wire: &Value) -> TestResult {
    use codelet_common::{ContentPart, Message, MessageContent, MessageRole, ToolResultPart};
    let messages: Vec<Message> = serde_json::from_value(wire["messages"].clone())?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, message) in messages.iter().enumerate() {
        let MessageContent::Parts(parts) = &message.content else {
            continue;
        };
        for part in parts {
            match part {
                ContentPart::ToolUse { id, name, input } if id == "call_after_snapshot" => {
                    assert_eq!(message.role, MessageRole::Assistant);
                    assert_eq!(name, "lookup");
                    assert_eq!(input, &json!({"path":"src/deep.rs"}));
                    calls.push(position);
                }
                ContentPart::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                    parts,
                } if tool_use_id == "call_after_snapshot" => {
                    assert_eq!(message.role, MessageRole::User);
                    assert!(!is_error);
                    assert_eq!(content, SUFFIX);
                    assert_eq!(
                        parts,
                        &[ToolResultPart::Text {
                            text: SUFFIX.into()
                        }]
                    );
                    results.push(position);
                }
                _ => {}
            }
        }
    }
    assert_pair(&calls, &results)
}

fn assert_pair(calls: &[usize], results: &[usize]) -> TestResult {
    assert_eq!(calls.len(), 1, "duplicated or omitted suffix call");
    assert_eq!(results.len(), 1, "duplicated or omitted suffix result");
    assert!(calls[0] < results[0], "suffix call must precede its result");
    Ok(())
}
