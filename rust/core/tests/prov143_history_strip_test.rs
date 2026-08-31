//! PROV-143 — strip thinking/reasoning blocks from outgoing chat history.
//!
//! Feature: spec/features/profile-preserve-thinking-strip.feature
//!
//! `strip_reasoning_from_history` produces a copy of a rig message history
//! with every `AssistantContent::Reasoning` block removed from assistant
//! messages, leaving text/tool-call content and the message vector's shape
//! untouched. It is the single helper the agent loop calls when the
//! session's preserve-thinking flag is disabled, so the LLM never sees old
//! thinking blocks while the persisted history keeps them.
//!
//! Each Gherkin step maps to a `// @step` comment immediately above the code
//! that exercises it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::strip_reasoning_from_history;
use rig::completion::AssistantContent;
use rig::message::{Message, Reasoning, UserContent};
use rig::OneOrMany;

fn assistant_with_reasoning_and_text() -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::many(vec![
            AssistantContent::Reasoning(Reasoning::new(
                "old thinking that must not go back to the LLM",
            )),
            AssistantContent::Text(rig::message::Text {
                text: "the visible answer".into(),
            }),
        ])
        .expect("two content items must build a OneOrMany"),
    }
}

/// Scenario: History handed to the LLM is stripped of thinking when disabled
#[test]
fn history_handed_to_the_llm_is_stripped_of_thinking_when_disabled() {
    // @step Given a history containing an assistant message with Reasoning and Text content
    let history = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text("do the thing")),
        },
        assistant_with_reasoning_and_text(),
    ];

    // @step When preserve-thinking is disabled for the session
    let outgoing = strip_reasoning_from_history(&history, false);

    // @step Then the history clone passed to the LLM keeps the Text content
    match &outgoing[1] {
        Message::Assistant { content, .. } => {
            let texts: Vec<&str> = content
                .iter()
                .filter_map(|c| match c {
                    AssistantContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                texts,
                vec!["the visible answer"],
                "text content must survive the strip: {content:?}"
            );
        }
        other => panic!("assistant message lost in the strip: {other:?}"),
    }

    // @step And the clone contains no Reasoning content in that message
    assert!(
        outgoing
            .iter()
            .all(|msg| !matches!(msg, Message::Assistant { content, .. } if content.iter().any(|c| matches!(c, AssistantContent::Reasoning(_))))),
        "no assistant message may carry Reasoning when preserve-thinking is off"
    );

    // @step And the original session history is not mutated
    assert!(
        matches!(&history[1], Message::Assistant { content, .. } if content.iter().any(|c| matches!(c, AssistantContent::Reasoning(_)))),
        "the source history must keep its Reasoning block"
    );
}

#[test]
fn reasoning_only_assistant_message_is_dropped_from_the_outgoing_history() {
    // @step Given a history containing an assistant message with Reasoning and Text content
    let history = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text("hello")),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Reasoning(
                Reasoning::new("only thinking"),
            )),
        },
        Message::User {
            content: OneOrMany::one(UserContent::text("follow-up")),
        },
    ];

    // @step When preserve-thinking is disabled for the session
    let outgoing = strip_reasoning_from_history(&history, false);

    // @step Then the clone passed to the LLM contains no empty assistant message
    assert_eq!(
        outgoing.len(),
        2,
        "a reasoning-only assistant message must be dropped, leaving the two user messages: {outgoing:?}"
    );
    assert!(
        outgoing.iter().all(|m| !matches!(m, Message::Assistant { content, .. } if content.is_empty())),
        "no empty assistant message may reach the LLM"
    );
}

#[test]
fn preserve_thinking_enabled_returns_history_unchanged() {
    // @step Given a history containing an assistant message with Reasoning and Text content
    let history = vec![assistant_with_reasoning_and_text()];

    // @step When preserve-thinking is enabled for the session
    let outgoing = strip_reasoning_from_history(&history, true);

    // @step Then the clone passed to the LLM still contains the Reasoning content
    assert!(
        matches!(&outgoing[0], Message::Assistant { content, .. } if content.iter().any(|c| matches!(c, AssistantContent::Reasoning(_)))),
        "preserve-thinking enabled must keep the Reasoning block"
    );
}

#[test]
fn stripping_keeps_message_count_and_user_messages_intact() {
    // @step Given a history containing an assistant message with Reasoning and Text content
    let history = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text("hello")),
        },
        assistant_with_reasoning_and_text(),
        Message::User {
            content: OneOrMany::one(UserContent::text("follow-up")),
        },
    ];

    // @step When preserve-thinking is disabled for the session
    let outgoing = strip_reasoning_from_history(&history, false);

    // @step Then the clone passed to the LLM keeps the same number of messages
    assert_eq!(
        outgoing.len(),
        3,
        "stripping must not drop messages, only Reasoning blocks"
    );
    assert!(
        matches!(&outgoing[0], Message::User { .. }) && matches!(&outgoing[2], Message::User { .. }),
        "user messages must survive verbatim"
    );
}
