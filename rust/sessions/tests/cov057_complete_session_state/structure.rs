//! COV-057: multi-part structure, parallel tool pairing, turn sequencing and
//! session-size edge cases survive a real save and cold restore of the
//! session's message store.

use super::persisted::{
    assistant, assistant_of, blob_files, fresh, reply, restore, save, stored, text, tool_result,
    user, user_of, user_text, Outcome,
};
use super::*;
use codelet_core::persistence::{compute_hash, UserContent};
use serde_json::json;

fn tool_use(id: &str, name: &str, input: serde_json::Value) -> AssistantContent {
    AssistantContent::ToolUse {
        id: id.into(),
        call_id: None,
        name: name.into(),
        input,
    }
}

/// `count` text-only user/assistant exchanges with unique content.
fn exchanges(count: usize) -> Vec<MessageEnvelope> {
    (0..count)
        .flat_map(|index| {
            [
                user(vec![user_text(&format!("question {index}"))]),
                reply(assistant(vec![text(&format!("answer {index}"))])),
            ]
        })
        .collect()
}

fn roles(envelopes: &[MessageEnvelope]) -> Vec<&str> {
    envelopes
        .iter()
        .map(|envelope| envelope.message_type.as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_multi_part_assistant_message_preserves_structure_and_order() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message containing:
    let parts = vec![
        text("Let me check that file..."),
        tool_use("toolu_a", "read_file", json!({"path": "/foo.ts"})),
        text("Found it. Now searching..."),
        tool_use("toolu_b", "grep", json!({"pattern": "error"})),
        text("Here's what I found."),
    ];
    // @step When the session is saved and restored
    save(session, &[reply(assistant(parts.clone()))])?;
    let restored = restore(session)?;
    let content = &assistant_of(restored.first().ok_or("no message")?)?.content;
    // @step Then the restored message has 5 content parts
    assert_eq!(content.len(), 5);
    let kind = |index: usize| -> Outcome<String> {
        Ok(serde_json::to_value(&content[index])?["type"]
            .as_str()
            .ok_or("untyped")?
            .to_owned())
    };
    // @step And content part 0 has type "text"
    assert_eq!(kind(0)?, "text");
    // @step And content part 1 has type "tool_use"
    assert_eq!(kind(1)?, "tool_use");
    // @step And content part 2 has type "text"
    assert_eq!(kind(2)?, "text");
    // @step And content part 3 has type "tool_use"
    assert_eq!(kind(3)?, "tool_use");
    // @step And content part 4 has type "text"
    assert_eq!(kind(4)?, "text");
    assert_eq!(content, &parts, "parts restored in their original order");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_parallel_tool_uses_in_single_message_preserved() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    let calls = [
        ("toolu_001", "read_file", "file contents"),
        ("toolu_002", "grep", "grep output"),
        ("toolu_003", "bash", "command output"),
    ];
    // @step Given a session with an assistant message containing 3 tool uses:
    let uses = calls
        .iter()
        .map(|(id, name, _)| tool_use(id, name, json!({})))
        .collect();
    // @step And a user message containing 3 tool results:
    let results = calls
        .iter()
        .rev()
        .map(|(id, _, output)| tool_result(id, output, false))
        .collect();
    // @step When the session is saved and restored
    save(session, &[reply(assistant(uses)), user(results)])?;
    let restored = restore(session)?;
    assert_eq!(restored.len(), 2);
    // @step Then the restored assistant message has 3 tool_use content parts
    let use_ids: Vec<&str> = assistant_of(&restored[0])?
        .content
        .iter()
        .filter_map(|part| match part {
            AssistantContent::ToolUse { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(use_ids, ["toolu_001", "toolu_002", "toolu_003"]);
    // @step And the restored user message has 3 tool_result content parts
    let result_ids: Vec<(&str, &str)> = user_of(&restored[1])?
        .iter()
        .filter_map(|part| match part {
            UserContent::ToolResult {
                tool_use_id,
                content,
                ..
            } => Some((tool_use_id.as_str(), content.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(result_ids.len(), 3);
    // @step And each tool_result tool_use_id matches its corresponding tool_use id
    for (id, _, output) in calls {
        assert!(use_ids.contains(&id));
        assert!(result_ids.contains(&(id, output)), "{id} lost its result");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_message_sequence_preserved_across_multiple_turns() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with 5 turns of user-assistant exchanges
    // @step And each turn has unique content
    let original = exchanges(5);
    // @step When the session is saved and restored
    save(session, &original)?;
    let restored = restore(session)?;
    // @step Then the restored session has 10 messages
    assert_eq!(restored.len(), 10);
    // @step And message 0 has role "user"
    assert_eq!(restored[0].message_type, "user");
    // @step And message 1 has role "assistant"
    assert_eq!(restored[1].message_type, "assistant");
    // @step And the message sequence alternates user-assistant
    assert_eq!(roles(&restored), ["user", "assistant"].repeat(5));
    assert_eq!(restored, original, "each turn's unique content in order");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_with_only_text_messages() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with 3 text-only exchanges
    // @step And no tool uses or results
    let original = exchanges(3);
    // @step When the session is saved and restored
    save(session, &original)?;
    let restored = restore(session)?;
    // @step Then all 6 messages are restored with correct roles
    assert_eq!(roles(&restored), roles(&original));
    assert_eq!(restored.len(), 6);
    // @step And all message text content matches original
    for (restored, original) in restored.iter().zip(&original) {
        assert_eq!(restored.message, original.message);
    }
    // @step And no blob storage is used
    assert!(blob_files()?.is_empty());
    assert!(stored(session)?
        .iter()
        .all(|raw| raw.get("_blobRefs").is_none()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_with_single_user_message() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with only one user message "Hello"
    // @step And no assistant response yet
    save(session, &[user(vec![user_text("Hello")])])?;
    // @step When the session is saved and restored
    let restored = restore(session)?;
    // @step Then the restored session has exactly 1 message
    assert_eq!(restored.len(), 1);
    // @step And the restored message role equals "user"
    assert_eq!(restored[0].message_type, "user");
    // @step And the restored message text equals "Hello"
    assert_eq!(user_of(&restored[0])?, [user_text("Hello")]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_session_with_many_turns_restores_completely() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with 100 turns
    // @step And mixed content types across messages
    let original: Vec<MessageEnvelope> = (0..100)
        .flat_map(|index| {
            let id = format!("toolu_{index:03}");
            match index % 3 {
                0 => [
                    user(vec![user_text(&format!("question {index}"))]),
                    reply(assistant(vec![text(&format!("answer {index}"))])),
                ],
                1 => [
                    reply(assistant(vec![tool_use(&id, "grep", json!({"n": index}))])),
                    user(vec![tool_result(&id, &format!("match {index}"), false)]),
                ],
                _ => [
                    user(vec![user_text(&format!("think {index}"))]),
                    reply(assistant(vec![AssistantContent::Thinking {
                        thinking: format!("reasoning {index}"),
                        signature: Some(format!("sig-{index}")),
                    }])),
                ],
            }
        })
        .collect();
    // @step When the session is saved and restored
    save(session, &original)?;
    let restored = restore(session)?;
    // @step Then the restored session has exactly 200 messages
    assert_eq!(restored.len(), 200);
    // @step And each message content hash matches original
    let hash = |envelope: &MessageEnvelope| -> Outcome<String> {
        Ok(compute_hash(&serde_json::to_vec(&envelope.message)?))
    };
    for (restored, original) in restored.iter().zip(&original) {
        assert_eq!(hash(restored)?, hash(original)?);
    }
    // @step And message order is preserved
    let order = |envelopes: &[MessageEnvelope]| -> Vec<uuid::Uuid> {
        envelopes.iter().map(|envelope| envelope.uuid).collect()
    };
    assert_eq!(order(&restored), order(&original));
    Ok(())
}
