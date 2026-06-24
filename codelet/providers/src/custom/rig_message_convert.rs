//! Bridge between rig's `completion::Message` shape and our internal
//! `codelet_common::Message` shape (PROV-092).
//!
//! `RhaiCustomProviderModel::completion()` receives a rig
//! [`CompletionRequest`] whose `chat_history` is a `OneOrMany<Message>`
//! using rig's User/Assistant content enums. The Rhai request bridge
//! ([`super::request_bridge`]) consumes our internal `Message`/`MessageContent`
//! shape (System/User/Assistant + `MessageContent::Text`/`Parts`).
//! This module converts between the two losslessly enough that the
//! Rhai script sees a stable wire format that matches what existing
//! tests already exercise.

use codelet_common::{
    ContentPart, ImageSource, Message, MessageContent, MessageRole, ToolResultPart,
};
use rig::completion::Message as RigMessage;
use rig::message::{
    AssistantContent, DocumentSourceKind, ImageMediaType, ToolResultContent, UserContent,
};

/// Convert a slice of rig messages into our internal `Vec<Message>`,
/// with an optional `preamble` prepended as a `MessageRole::System`
/// message (rig stores the system prompt separately on the request).
pub fn rig_messages_to_internal(preamble: Option<&str>, history: &[RigMessage]) -> Vec<Message> {
    let mut out: Vec<Message> = Vec::new();
    if let Some(text) = preamble {
        if !text.is_empty() {
            out.push(Message::system(text.to_string()));
        }
    }
    for msg in history {
        match msg {
            RigMessage::User { content } => {
                out.push(convert_user_message(content.iter()));
            }
            RigMessage::Assistant { content, .. } => {
                out.push(convert_assistant_message(content.iter()));
            }
        }
    }
    out
}

fn convert_user_message<'a, I>(content_iter: I) -> Message
where
    I: Iterator<Item = &'a UserContent>,
{
    let mut parts: Vec<ContentPart> = Vec::new();
    for piece in content_iter {
        match piece {
            UserContent::Text(text) => {
                parts.push(ContentPart::Text {
                    text: text.text.clone(),
                });
            }
            UserContent::ToolResult(result) => {
                // BUG-141: walk the rig tool_result entries IN ORDER and
                // build a structured `Vec<ToolResultPart>`. Text entries
                // become `ToolResultPart::Text`; image entries become
                // `ToolResultPart::Image` reusing the existing
                // [`image_to_source`] helper so the wire format matches
                // the request-side `ContentPart::Image`. Image variants
                // whose source cannot be derived (Raw / Unknown) are
                // skipped — `image_to_source` already logs at debug —
                // and never become an empty Text part.
                let mut tool_parts: Vec<ToolResultPart> = Vec::new();
                for tc in result.content.iter() {
                    match tc {
                        ToolResultContent::Text(text) => {
                            tool_parts.push(ToolResultPart::Text {
                                text: text.text.clone(),
                            });
                        }
                        ToolResultContent::Image(image) => {
                            if let Some(source) =
                                image_to_source(image.data.clone(), image.media_type.clone())
                            {
                                tool_parts.push(ToolResultPart::Image { source });
                            }
                        }
                    }
                }

                // Preserve the existing happy-path shape for text-only
                // tool_results (legacy callers still read `content` as
                // a string). For mixed or empty payloads, defer to
                // `tool_result_parts` which derives a best-effort
                // `content` summary from the parts vector.
                let single_text = match tool_parts.as_slice() {
                    [ToolResultPart::Text { text }] => Some(text.clone()),
                    _ => None,
                };
                let part = match single_text {
                    Some(text) => ContentPart::tool_result_text(result.id.clone(), text, false),
                    None if tool_parts.is_empty() => {
                        // Empty conversions (e.g. only an Unknown image
                        // input) collapse to an empty text part so the
                        // ToolResult invariant (parts non-empty) holds.
                        ContentPart::tool_result_text(result.id.clone(), String::new(), false)
                    }
                    None => ContentPart::tool_result_parts(result.id.clone(), tool_parts, false),
                };
                parts.push(part);
            }
            UserContent::Image(image) => {
                if let Some(source) = image_to_source(image.data.clone(), image.media_type.clone())
                {
                    parts.push(ContentPart::Image { source });
                }
            }
            UserContent::Audio(_) | UserContent::Video(_) | UserContent::Document(_) => {
                // Non-text/image multimedia is ignored — Rhai scripts
                // don't currently see these shapes; downstream codelet
                // bridges handle them only for native providers.
            }
        }
    }
    let content = collapse_parts_to_message_content(parts);
    Message {
        role: MessageRole::User,
        content,
    }
}

fn convert_assistant_message<'a, I>(content_iter: I) -> Message
where
    I: Iterator<Item = &'a AssistantContent>,
{
    let mut parts: Vec<ContentPart> = Vec::new();
    for piece in content_iter {
        match piece {
            AssistantContent::Text(text) => {
                parts.push(ContentPart::Text {
                    text: text.text.clone(),
                });
            }
            AssistantContent::ToolCall(call) => {
                parts.push(ContentPart::ToolUse {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    input: call.function.arguments.clone(),
                });
            }
            AssistantContent::Reasoning(reasoning) => {
                for r in &reasoning.reasoning {
                    parts.push(ContentPart::Text { text: r.clone() });
                }
            }
            AssistantContent::Image(_) => {
                // Assistant-side images are intentionally skipped here
                // — the rig history conversion path must remain lossy
                // rather than fail so a single stray image in a long
                // conversation does not abort the whole completion.
                // The stricter policy (hard error) lives in
                // `adapter::convert_assistant_content`, which is the
                // boundary where new assistant responses enter the
                // system; by that point an image is a protocol
                // violation we want to surface. Log at debug so the
                // drop is observable during integration testing.
                tracing::debug!(
                    "rig history → Rhai: dropping AssistantContent::Image \
                     (scripts cannot receive assistant-side images)"
                );
            }
        }
    }
    let content = collapse_parts_to_message_content(parts);
    Message {
        role: MessageRole::Assistant,
        content,
    }
}

fn collapse_parts_to_message_content(parts: Vec<ContentPart>) -> MessageContent {
    if parts.is_empty() {
        return MessageContent::Text(String::new());
    }
    if parts.len() == 1 {
        if let ContentPart::Text { text } = &parts[0] {
            return MessageContent::Text(text.clone());
        }
    }
    MessageContent::Parts(parts)
}

fn image_to_source(
    data: DocumentSourceKind,
    media_type: Option<ImageMediaType>,
) -> Option<ImageSource> {
    match data {
        DocumentSourceKind::Url(url) => Some(ImageSource::Url { url }),
        DocumentSourceKind::Base64(blob) => {
            let media = image_media_type_to_mime(media_type);
            Some(ImageSource::Base64 {
                media_type: media,
                data: blob,
            })
        }
        DocumentSourceKind::String(text) => Some(ImageSource::Url { url: text }),
        // Known unsupported variants — log once so observers know an
        // image was silently dropped.
        DocumentSourceKind::Raw(_) | DocumentSourceKind::Unknown => {
            tracing::debug!(
                "rig image source variant (Raw/Unknown) not supported by Rhai \
                 bridge — dropping image"
            );
            None
        }
        // `DocumentSourceKind` is `#[non_exhaustive]` upstream; fall
        // through gracefully when rig adds new variants, but warn
        // because the new variant needs explicit handling.
        _ => {
            tracing::warn!(
                "unknown rig DocumentSourceKind variant encountered in Rhai \
                 image bridge; update image_to_source to handle it"
            );
            None
        }
    }
}

/// Map a rig [`ImageMediaType`] to its canonical IANA MIME string.
///
/// Known variants (JPEG/PNG/GIF/WEBP) are spelled out exhaustively.
/// When the caller supplies `None` we return `application/octet-stream`
/// — this matches the behaviour of base64 attachments without an
/// explicit media type and avoids silently asserting "png" (the
/// previous default). Scripts that need a specific MIME can override
/// it via their own `build_request` logic.
///
/// Future rig enum expansions fall through the `other` arm into a
/// lowercase `image/<debug>` best-effort string plus a debug log,
/// making the fallback observable.
fn image_media_type_to_mime(media_type: Option<ImageMediaType>) -> String {
    let Some(m) = media_type else {
        return "application/octet-stream".to_string();
    };
    match m {
        ImageMediaType::JPEG => "image/jpeg".to_string(),
        ImageMediaType::PNG => "image/png".to_string(),
        ImageMediaType::GIF => "image/gif".to_string(),
        ImageMediaType::WEBP => "image/webp".to_string(),
        other => {
            let guessed = format!("image/{other:?}").to_lowercase();
            tracing::debug!(
                "rig ImageMediaType::{other:?} has no IANA mapping; \
                 using best-effort '{guessed}'"
            );
            guessed
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rig::OneOrMany;

    #[test]
    fn user_text_round_trips_to_internal_text() {
        let msg = RigMessage::User {
            content: OneOrMany::one(UserContent::Text("hello".into())),
        };
        let out = rig_messages_to_internal(None, &[msg]);
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0].role, MessageRole::User));
        match &out[0].content {
            MessageContent::Text(t) => assert_eq!(t, "hello"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn preamble_is_prepended_as_system_message() {
        let msg = RigMessage::User {
            content: OneOrMany::one(UserContent::Text("user".into())),
        };
        let out = rig_messages_to_internal(Some("system"), &[msg]);
        assert_eq!(out.len(), 2);
        assert!(matches!(out[0].role, MessageRole::System));
        assert!(matches!(out[1].role, MessageRole::User));
    }

    #[test]
    fn empty_preamble_is_dropped() {
        let msg = RigMessage::User {
            content: OneOrMany::one(UserContent::Text("user".into())),
        };
        let out = rig_messages_to_internal(Some(""), &[msg]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn assistant_tool_call_becomes_tool_use_part() {
        let msg = RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::tool_call(
                "call_1",
                "read_file",
                serde_json::json!({"path": "/tmp/x"}),
            )),
        };
        let out = rig_messages_to_internal(None, &[msg]);
        match &out[0].content {
            MessageContent::Parts(parts) => match &parts[0] {
                ContentPart::ToolUse { id, name, input } => {
                    assert_eq!(id, "call_1");
                    assert_eq!(name, "read_file");
                    assert_eq!(input["path"], "/tmp/x");
                }
                other => panic!("expected ToolUse, got {other:?}"),
            },
            other => panic!("expected Parts, got {other:?}"),
        }
    }
}
