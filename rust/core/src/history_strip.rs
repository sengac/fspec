//! PROV-143 — strip thinking/reasoning blocks from outgoing chat history.
//!
//! Feature: spec/features/profile-preserve-thinking.feature
//!
//! "Preserve thinking" means keeping `AssistantContent::Reasoning` blocks in
//! the conversation history that is sent BACK to the LLM. When a session's
//! preserve-thinking flag is disabled (the default), old thinking blocks are
//! stripped from the history clone before it is handed to the model so the
//! LLM is not confused by stale reasoning from previous turns. The persisted
//! session history always keeps the blocks; only the outgoing clone drops
//! them.
//!
//! This is a pure function over `&mut [Message]` (cloned by the caller) so the
//! live session history is never mutated.

use rig::message::{AssistantContent, Message};

/// Return a copy of `history` with every `AssistantContent::Reasoning` block
/// removed when `preserve_thinking` is `false`. When `preserve_thinking` is
/// `true` the copy is returned unchanged.
///
/// The source slice is never mutated — the caller passes in the session's live
/// history and receives a fresh outgoing clone.
///
/// Assistant messages that become EMPTY after the strip (reasoning-only
/// messages) are dropped entirely — an assistant message with no content is
/// invalid on the OpenAI-compat wire format.
pub fn strip_reasoning_from_history(
    history: &[Message],
    preserve_thinking: bool,
) -> Vec<Message> {
    if preserve_thinking {
        return history.to_vec();
    }
    let mut out: Vec<Message> = Vec::with_capacity(history.len());
    for message in history {
        match message {
            Message::Assistant { id, content } => {
                let kept: Vec<AssistantContent> = content
                    .iter()
                    .filter(|c| !matches!(c, AssistantContent::Reasoning(_)))
                    .cloned()
                    .collect();
                if kept.is_empty() {
                    // Reasoning-only assistant message: drop it so the outgoing
                    // history never carries an empty assistant message.
                    continue;
                }
                let content = match rig::OneOrMany::many(kept) {
                    Ok(c) => c,
                    Err(e) => {
                        // Unreachable (kept is non-empty); keep the original
                        // rather than drop the message.
                        tracing::warn!(
                            error = %e,
                            "PROV-143: OneOrMany::many failed; keeping message unchanged"
                        );
                        out.push(message.clone());
                        continue;
                    }
                };
                out.push(Message::Assistant { id: id.clone(), content });
            }
            other => out.push(other.clone()),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use rig::message::{Reasoning, Text};
    use rig::OneOrMany;

    fn reasoning() -> AssistantContent {
        AssistantContent::Reasoning(Reasoning::new("old thinking"))
    }

    fn text(t: &str) -> AssistantContent {
        AssistantContent::Text(Text {
            text: t.to_string(),
        })
    }

    #[test]
    fn disabled_strip_removes_reasoning_keeps_text() {
        let history = vec![Message::Assistant {
            id: None,
            content: OneOrMany::many(vec![reasoning(), text("answer")])
                .expect("two items"),
        }];
        let out = strip_reasoning_from_history(&history, false);
        let Message::Assistant { content, .. } = &out[0] else {
            panic!("assistant lost");
        };
        let items: Vec<_> = content.iter().collect();
        assert_eq!(items.len(), 1, "only the text block should survive");
        assert!(matches!(&items[0], AssistantContent::Text(t) if t.text == "answer"));
        // source is untouched
        let src: Vec<_> = match &history[0] {
            Message::Assistant { content, .. } => content.iter().collect(),
            _ => panic!("user?"),
        };
        assert_eq!(src.len(), 2, "source history must be unmodified");
    }

    #[test]
    fn enabled_flag_returns_history_unchanged() {
        let history = vec![Message::Assistant {
            id: None,
            content: OneOrMany::many(vec![reasoning(), text("answer")])
                .expect("two items"),
        }];
        let out = strip_reasoning_from_history(&history, true);
        let src: Vec<_> = match &history[0] {
            Message::Assistant { content, .. } => content.iter().collect(),
            _ => panic!(),
        };
        let dst: Vec<_> = match &out[0] {
            Message::Assistant { content, .. } => content.iter().collect(),
            _ => panic!(),
        };
        assert_eq!(src.len(), dst.len());
    }

    #[test]
    fn reasoning_only_assistant_message_is_dropped() {
        let history = vec![
            Message::Assistant {
                id: None,
                content: OneOrMany::one(reasoning()),
            },
            Message::Assistant {
                id: None,
                content: OneOrMany::one(text("keep me")),
            },
        ];
        let out = strip_reasoning_from_history(&history, false);
        assert_eq!(out.len(), 1, "reasoning-only message must be dropped");
        assert!(matches!(&out[0], Message::Assistant { content, .. } if content.iter().any(|c| matches!(c, AssistantContent::Text(_)))));
    }

    #[test]
    fn user_messages_pass_through_verbatim() {
        let history = vec![rig::message::Message::User {
            content: OneOrMany::one(rig::message::UserContent::text("hi")),
        }];
        let out = strip_reasoning_from_history(&history, false);
        assert!(matches!(&out[0], Message::User { .. }));
    }
}
