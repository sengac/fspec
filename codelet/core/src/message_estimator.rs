//! Message token estimation for rig Message types
//!
//! Provides token estimation for rig's Message types before API calls.
//! This allows the CompactionHook to check estimated payload size, not just
//! the stale token count from the previous API response.

use codelet_common::token_estimator::count_tokens;
use rig::message::{AssistantContent, Message, ToolResultContent, UserContent};

/// Estimate tokens for a single rig Message
pub fn estimate_message_tokens(message: &Message) -> usize {
    match message {
        Message::User { content } => content.iter().map(estimate_user_content_tokens).sum(),
        Message::Assistant { content, .. } => {
            content.iter().map(estimate_assistant_content_tokens).sum()
        }
    }
}

/// Estimate tokens for a slice of messages (prompt + history)
pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

fn estimate_user_content_tokens(content: &UserContent) -> usize {
    match content {
        UserContent::Text(text) => count_tokens(&text.text),
        UserContent::ToolResult(result) => result
            .content
            .iter()
            .map(|trc| match trc {
                ToolResultContent::Text(text) => count_tokens(&text.text),
                ToolResultContent::Image(_) => IMAGE_TOKEN_ESTIMATE,
            })
            .sum(),
        UserContent::Image(_) => IMAGE_TOKEN_ESTIMATE,
        UserContent::Audio(_) => AUDIO_TOKEN_ESTIMATE,
        UserContent::Video(_) => VIDEO_TOKEN_ESTIMATE,
        UserContent::Document(_) => DOCUMENT_TOKEN_ESTIMATE,
    }
}

fn estimate_assistant_content_tokens(content: &AssistantContent) -> usize {
    match content {
        AssistantContent::Text(text) => count_tokens(&text.text),
        AssistantContent::ToolCall(tool_call) => {
            let name_tokens = count_tokens(&tool_call.function.name);
            let args_json = serde_json::to_string(&tool_call.function.arguments).unwrap_or_default();
            let args_tokens = count_tokens(&args_json);
            name_tokens + args_tokens + TOOL_CALL_OVERHEAD
        }
        AssistantContent::Reasoning(reasoning) => {
            reasoning.reasoning.iter().map(|s| count_tokens(s)).sum()
        }
        AssistantContent::Image(_) => IMAGE_TOKEN_ESTIMATE,
    }
}

const IMAGE_TOKEN_ESTIMATE: usize = 85;
const AUDIO_TOKEN_ESTIMATE: usize = 100;
const VIDEO_TOKEN_ESTIMATE: usize = 200;
const DOCUMENT_TOKEN_ESTIMATE: usize = 100;
const TOOL_CALL_OVERHEAD: usize = 20;

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::{Text, ToolCall, ToolFunction, ToolResult};
    use rig::OneOrMany;

    #[test]
    fn test_estimate_text_message() {
        let message = Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Hello, world!".to_string(),
            })),
        };
        let tokens = estimate_message_tokens(&message);
        assert!(tokens > 0);
        assert!(tokens < 10);
    }

    #[test]
    fn test_estimate_tool_result_message() {
        let large_content = "x".repeat(10_000);
        let message = Message::User {
            content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                id: "test".to_string(),
                call_id: None,
                content: OneOrMany::one(ToolResultContent::Text(Text { text: large_content })),
            })),
        };
        let tokens = estimate_message_tokens(&message);
        // 10,000 chars of 'x' should be roughly 1,250-2,500 tokens depending on encoding
        assert!(tokens > 1000, "Expected >1000 tokens, got {tokens}");
    }

    #[test]
    fn test_estimate_tool_call_message() {
        let message = Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                id: "call_123".to_string(),
                call_id: None,
                function: ToolFunction {
                    name: "read".to_string(),
                    arguments: serde_json::json!({"file_path": "/path/to/file.rs"}),
                },
                signature: None,
                additional_params: None,
            })),
        };
        let tokens = estimate_message_tokens(&message);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_empty_messages() {
        let messages: Vec<Message> = vec![];
        assert_eq!(estimate_messages_tokens(&messages), 0);
    }

    #[test]
    fn test_estimate_multiple_messages() {
        let messages = vec![
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "Hello".to_string(),
                })),
            },
            Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: "Hi there!".to_string(),
                })),
            },
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "How are you?".to_string(),
                })),
            },
        ];
        let tokens = estimate_messages_tokens(&messages);
        assert!(tokens > 5, "Expected >5 tokens for 3 messages, got {tokens}");
    }

    #[test]
    fn test_estimate_message_with_large_tool_result() {
        // Simulates reading a large file - this is the main scenario we're fixing
        let file_content = "fn main() { println!(\"Hello\"); }\n".repeat(1000);
        let messages = vec![
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "Read this file".to_string(),
                })),
            },
            Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::ToolCall(ToolCall {
                    id: "call_1".to_string(),
                    call_id: None,
                    function: ToolFunction {
                        name: "read".to_string(),
                        arguments: serde_json::json!({"file_path": "/src/main.rs"}),
                    },
                    signature: None,
                    additional_params: None,
                })),
            },
            Message::User {
                content: OneOrMany::one(UserContent::ToolResult(ToolResult {
                    id: "call_1".to_string(),
                    call_id: None,
                    content: OneOrMany::one(ToolResultContent::Text(Text { text: file_content })),
                })),
            },
        ];
        let tokens = estimate_messages_tokens(&messages);
        // Large file content should dominate the token count
        assert!(tokens > 5000, "Expected >5000 tokens for large file, got {tokens}");
    }

    #[test]
    fn test_image_content_has_fixed_estimate() {
        use rig::message::{DocumentSourceKind, Image};
        
        let message = Message::User {
            content: OneOrMany::one(UserContent::Image(Image {
                data: DocumentSourceKind::Base64("fake_base64_data".to_string()),
                media_type: None,
                detail: None,
                additional_params: None,
            })),
        };
        let tokens = estimate_message_tokens(&message);
        assert_eq!(tokens, IMAGE_TOKEN_ESTIMATE);
    }

    #[test]
    fn test_assistant_text_message() {
        let message = Message::Assistant {
            id: Some("msg_123".to_string()),
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "This is a response from the assistant.".to_string(),
            })),
        };
        let tokens = estimate_message_tokens(&message);
        assert!(tokens > 0);
        assert!(tokens < 20);
    }
}
