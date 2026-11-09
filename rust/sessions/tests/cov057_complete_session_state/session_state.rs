//! COV-057: session-level state after a real cold `resume_session` — the
//! aggregate token tracker, turn boundaries rebuilt from the restored rig
//! messages, and a follow-up turn that answers from restored tool output.

use super::persisted::{
    assistant, fresh, reply, save, text, tool_result, user, user_text, Outcome,
};
use super::*;
use codelet_cli::interactive_helpers::{convert_messages_to_turns, extract_message_text};
use codelet_core::persistence::update_session_tokens;
use rig::message::Message;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_aggregate_token_tracker_state_restored() -> TestResult {
    let (_directory, manager, session_id, session) = fresh().await?;
    // @step Given a session with cumulative token usage:
    let mut manifest = load_session(session)?;
    update_session_tokens(&mut manifest, 150_000, 25_000, 50_000, 10_000, 0)?;
    // @step When the session is saved and restored
    drop(manager);
    let restored = cold_resume(&session_id)?;
    let background = restored.get_session(&session_id.value)?;
    let tracker = background.inner.lock().await.token_tracker.clone();
    // @step Then the restored token tracker input_tokens equals 150000
    assert_eq!(tracker.input_tokens, 150_000);
    // @step And the restored token tracker output_tokens equals 25000
    assert_eq!(tracker.output_tokens, 25_000);
    // @step And the restored token tracker cache_read_input_tokens equals 50000
    assert_eq!(tracker.cache_read_input_tokens, Some(50_000));
    // @step And the restored token tracker cache_creation_input_tokens equals 10000
    assert_eq!(tracker.cache_creation_input_tokens, Some(10_000));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_turn_boundaries_reconstructed_from_message_sequence() -> TestResult {
    let (_directory, manager, session_id, session) = fresh().await?;
    // @step Given a session with 10 user-assistant exchange pairs
    let pairs: Vec<MessageEnvelope> = (0..10)
        .flat_map(|index| {
            [
                user(vec![user_text(&format!("question {index}"))]),
                reply(assistant(vec![text(&format!("answer {index}"))])),
            ]
        })
        .collect();
    save(session, &pairs)?;
    // @step When the session is saved and restored
    drop(manager);
    let restored = cold_resume(&session_id)?;
    let background = restored.get_session(&session_id.value)?;
    let messages = background.inner.lock().await.messages.clone();
    let turns = convert_messages_to_turns(&messages);
    // @step Then the restored session has 10 turns
    assert_eq!(turns.len(), 10, "restored {messages:?}");
    // @step And each turn contains one user message and one assistant message
    for (index, turn) in turns.iter().enumerate() {
        assert_eq!(turn.user_message, format!("question {index}"));
        assert_eq!(turn.assistant_response, format!("answer {index}"));
    }
    // @step And turn boundaries align with user message indices
    let user_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| {
            matches!(message, Message::User { .. })
                && !codelet_cli::session::system_reminders::is_system_reminder(message)
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(user_indices.len(), turns.len());
    for (turn, start) in turns.iter().zip(&user_indices) {
        assert_eq!(extract_message_text(&messages[*start]), turn.user_message);
        let next = messages.get(start + 1).ok_or("turn has no response")?;
        assert!(matches!(next, Message::Assistant { .. }));
        assert_eq!(extract_message_text(next), turn.assistant_response);
    }
    Ok(())
}

/// Messages in the provider request whose JSON mentions `needle`.
fn mentions(wire: &serde_json::Value, needle: &str) -> Outcome<Vec<usize>> {
    let messages = wire["messages"].as_array().ok_or("no messages")?;
    let mut found = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        if serde_json::to_string(message)?.contains(needle) {
            found.push(index);
        }
    }
    Ok(found)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_restored_session_enables_context_aware_follow_up() -> TestResult {
    let (_directory, manager, session_id, session) = fresh().await?;
    // @step Given a session where read_file tool returned config with port=8080 and host=localhost
    let call = AssistantContent::ToolUse {
        id: "toolu_cfg".into(),
        call_id: None,
        name: "read_file".into(),
        input: json!({"path": "config.json"}),
    };
    let config = r#"{"port": 8080, "host": "localhost"}"#;
    // @step And the assistant only mentioned the port in the response
    save(
        session,
        &[
            user(vec![user_text("Read config.json")]),
            reply(assistant(vec![call])),
            user(vec![tool_result("toolu_cfg", config, false)]),
            reply(assistant(vec![text("The port is 8080.")])),
        ],
    )?;
    // @step When the session is saved and restored
    drop(manager);
    let restored = cold_resume(&session_id)?;
    let before = load_session(session)?.messages.len();
    // @step And the user asks "what was the host value in that config?"
    let question = "what was the host value in that config?";
    let wire = turn(
        &restored,
        &session_id,
        question,
        "The host is localhost.",
        50,
    )
    .await?;
    // @step Then the restored session contains the full tool result with host=localhost
    let messages = wire["messages"].as_array().ok_or("no messages")?;
    let hosts = mentions(&wire, "localhost")?;
    assert_eq!(
        hosts.len(),
        1,
        "only the tool result names the host: {wire}"
    );
    let result = &messages[hosts[0]];
    assert_eq!(result["role"], "user");
    let encoded = serde_json::to_string(result)?;
    assert!(encoded.contains("tool_result") && encoded.contains("toolu_cfg"));
    assert!(encoded.contains("8080"));
    // @step And the follow-up can be answered without re-reading the file
    assert_eq!(mentions(&wire, question)?, [messages.len() - 1]);
    assert!(
        hosts[0] < messages.len() - 1,
        "result precedes the question"
    );
    assert_eq!(
        mentions(&wire, "toolu_cfg")?.len(),
        2,
        "no new read_file call"
    );
    let after = load_session(session)?;
    assert_eq!(after.messages.len(), before + 2, "question + direct answer");
    let last = get_session_messages(&after)?.pop().ok_or("no answer")?;
    assert_eq!(
        (last.role.as_str(), last.content.as_str()),
        ("assistant", "The host is localhost.")
    );
    Ok(())
}
