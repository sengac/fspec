//! EXT-016: Image content sanitization and recovery.
//!
//! Handles sanitization of image content from conversation history after
//! API rejection due to pixel dimension or size limits.

use rig::message::{Message, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;

/// EXT-016: Sanitize image content from conversation history.
///
/// Walks messages and replaces any Image content (UserContent::Image,
/// ToolResultContent::Image within UserContent::ToolResult) with text placeholders.
///
/// Returns `true` if any image content was replaced.
///
/// This function is public for testing.
pub fn sanitize_image_content(messages: &mut [Message]) -> bool {
    let mut replaced = false;

    for msg in messages.iter_mut().rev() {
        if let Message::User { content } = msg {
            let mut has_image = false;
            for item in content.iter() {
                match item {
                    UserContent::Image { .. } => {
                        has_image = true;
                        break;
                    }
                    UserContent::ToolResult(tool_result) => {
                        for tr_item in tool_result.content.iter() {
                            if matches!(tr_item, ToolResultContent::Image { .. }) {
                                has_image = true;
                                break;
                            }
                        }
                        if has_image {
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if has_image {
                let mut new_parts: Vec<UserContent> = Vec::new();
                for item in content.iter() {
                    match item {
                        UserContent::Image { .. } => {
                            new_parts.push(UserContent::text(
                                "[Image removed: exceeded provider pixel dimension limit]",
                            ));
                            replaced = true;
                        }
                        UserContent::ToolResult(tool_result) => {
                            // Check if tool result contains images
                            let has_tr_image = tool_result
                                .content
                                .iter()
                                .any(|i| matches!(i, ToolResultContent::Image { .. }));

                            if has_tr_image {
                                // Replace image content within tool result
                                let mut new_tr_parts: Vec<ToolResultContent> = Vec::new();
                                for tr_item in tool_result.content.iter() {
                                    match tr_item {
                                        ToolResultContent::Image { .. } => {
                                            new_tr_parts.push(ToolResultContent::text(
                                                "[Image removed: exceeded provider pixel dimension limit]",
                                            ));
                                            replaced = true;
                                        }
                                        other => {
                                            new_tr_parts.push(other.clone());
                                        }
                                    }
                                }
                                if let Ok(new_tr_content) =
                                    OneOrMany::many(new_tr_parts)
                                {
                                    // Preserve call_id if present (OpenAI provider path)
                                    if let Some(call_id) = &tool_result.call_id {
                                        new_parts.push(UserContent::tool_result_with_call_id(
                                            &tool_result.id,
                                            call_id.clone(),
                                            new_tr_content,
                                        ));
                                    } else {
                                        new_parts.push(UserContent::tool_result(
                                            &tool_result.id,
                                            new_tr_content,
                                        ));
                                    }
                                } else {
                                    new_parts.push(item.clone());
                                }
                            } else {
                                new_parts.push(item.clone());
                            }
                        }
                        other => {
                            new_parts.push(other.clone());
                        }
                    }
                }
                if let Ok(new_content) = OneOrMany::many(new_parts) {
                    *content = new_content;
                }
            }
        }
    }

    replaced
}
