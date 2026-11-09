//! Feature: spec/features/session-replay-keeps-stored-envelopes.feature
//!
//! BUG-327: `get_session_message_envelopes` returns the envelopes the session
//! stored. Messages are written the way the agent loop's persistence commit
//! does (`persistence_retry_commit`): the typed envelope is blob-processed,
//! its fields plus `_blobRefs` become the stored metadata, and it is appended
//! under the envelope's own uuid with `append_message_idempotent`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use codelet_core::persistence::conversation_commit::append_message_idempotent;
use codelet_core::persistence::{
    append_message, create_session_with_provider, get_session_message_envelopes,
    get_session_messages, load_session, process_envelope_for_blob_storage, set_compaction_state,
    AssistantContent, AssistantMessage, MessageEnvelope, MessagePayload, TokenUsagePerMessage,
    UserContent, UserMessage,
};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

/// Persist `envelope` exactly as the agent loop's commit path stores it.
fn persist(session: Uuid, envelope: &MessageEnvelope, summary: &str) {
    let (processed, references) = process_envelope_for_blob_storage(envelope).unwrap();
    let mut metadata: HashMap<String, Value> =
        serde_json::from_value(serde_json::to_value(&processed).unwrap()).unwrap();
    if !references.is_empty() {
        metadata.insert(
            "_blobRefs".into(),
            references
                .into_iter()
                .map(|(key, hash)| json!({"key": key, "hash": hash}))
                .collect(),
        );
    }
    append_message_idempotent(
        session,
        envelope.uuid,
        &envelope.message_type,
        summary,
        metadata,
    )
    .unwrap();
}

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
#[serial]
fn scenario_replayed_envelopes_keep_the_stored_envelope_and_message_metadata() {
    let (_directory, session) = bug332_session("claude");
    let session = session.id;
    let large_result = "tool output line\n".repeat(1_000);
    let (user_id, assistant_id, root_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

    // @step Given a user message and an assistant reply persisted through the production write path with known uuid, parentUuid, timestamp and requestId
    let user = MessageEnvelope {
        uuid: user_id,
        parent_uuid: Some(root_id),
        timestamp: at("2026-01-02T03:04:05Z"),
        message_type: "user".into(),
        provider: "claude".into(),
        message: MessagePayload::User(UserMessage {
            role: "user".into(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_327".into(),
                call_id: None,
                content: large_result.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };
    // @step And the assistant reply carries message id "msg_327", model "claude-sonnet-4", stop reason "end_turn" and usage
    let usage = TokenUsagePerMessage {
        input_tokens: 1_200,
        output_tokens: 34,
        cache_read_input_tokens: Some(1_000),
        cache_creation_input_tokens: None,
    };
    let assistant = MessageEnvelope {
        uuid: assistant_id,
        parent_uuid: Some(user_id),
        timestamp: at("2026-01-02T03:04:09Z"),
        message_type: "assistant".into(),
        provider: "claude".into(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".into(),
            id: Some("msg_327".into()),
            model: Some("claude-sonnet-4".into()),
            content: vec![AssistantContent::Text {
                text: "done".into(),
            }],
            stop_reason: Some("end_turn".into()),
            usage: Some(usage.clone()),
        }),
        request_id: Some("req_327".into()),
    };
    persist(session, &user, "tool output");
    persist(session, &assistant, "done");

    // @step When the session's message envelopes are read for replay
    let replayed: Vec<Value> = get_session_message_envelopes(session)
        .unwrap()
        .iter()
        .map(|encoded| serde_json::from_str(encoded).unwrap())
        .filter(|envelope: &Value| envelope.get("uuid").is_some())
        .collect();
    assert_eq!(replayed.len(), 2);
    let (user_replay, assistant_replay) = (&replayed[0], &replayed[1]);

    // @step Then each envelope has its stored uuid, parentUuid, timestamp and requestId
    for (replay, stored) in [(user_replay, &user), (assistant_replay, &assistant)] {
        let expected = serde_json::to_value(stored).unwrap();
        for key in [
            "uuid",
            "parentUuid",
            "timestamp",
            "requestId",
            "type",
            "provider",
        ] {
            assert_eq!(replay[key], expected[key], "{key}");
        }
    }
    assert_eq!(assistant_replay["requestId"], "req_327");
    assert_eq!(user_replay["parentUuid"], json!(root_id));

    // @step And the assistant envelope's message has id "msg_327", model "claude-sonnet-4", stop reason "end_turn" and the stored usage
    let message = &assistant_replay["message"];
    assert_eq!(message["id"], "msg_327");
    assert_eq!(message["model"], "claude-sonnet-4");
    assert_eq!(message["stop_reason"], "end_turn");
    assert_eq!(message["usage"], serde_json::to_value(&usage).unwrap());

    // @step And each envelope's message content is the stored content with blobs rehydrated
    assert_eq!(
        user_replay["message"]["content"][0]["content"],
        json!(large_result)
    );
    assert_eq!(
        message["content"],
        json!([{"type": "text", "text": "done"}])
    );
    assert_eq!(user_replay["message"]["role"], "user");
    assert_eq!(message["role"], "assistant");
}

fn bug332_session(
    provider: &str,
) -> (
    tempfile::TempDir,
    codelet_core::persistence::SessionManifest,
) {
    let directory = setup_temp_directory().unwrap();
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(directory.path().to_owned()).unwrap();
    let session = create_session_with_provider("bug332", directory.path(), provider).unwrap();
    (directory, session)
}

#[test]
#[serial]
fn scenario_a_stored_row_without_a_message_envelope_is_refused_by_replay() {
    // @step Given a conversation row stored without a message envelope
    let (_directory, session) = bug332_session("claude");
    let row = Uuid::new_v4();
    append_message_idempotent(session.id, row, "user", "bare", HashMap::new()).unwrap();

    // @step When the session's message envelopes are read for replay
    let replay = get_session_message_envelopes(session.id);

    // @step Then replay fails naming the row and the missing envelope
    let error = replay.unwrap_err();
    assert!(error.contains(&row.to_string()), "{error}");
    assert!(error.contains("has no message envelope"), "{error}");
}

#[test]
#[serial]
fn scenario_rows_written_by_the_append_helpers_replay_with_a_full_envelope() {
    // @step Given a codex session with a message written by append_message
    let (_directory, mut session) = bug332_session("codex");
    append_message(&mut session, "user", "hello").unwrap();

    // @step When the session's message envelopes are read for replay
    let replayed: Vec<Value> = get_session_message_envelopes(session.id)
        .unwrap()
        .iter()
        .map(|encoded| serde_json::from_str(encoded).unwrap())
        .filter(|envelope: &Value| envelope.get("uuid").is_some())
        .collect();

    // @step Then its envelope names the session's provider with a null parent and request id
    assert_eq!(replayed.len(), 1);
    let envelope = &replayed[0];
    assert_eq!(envelope["provider"], "codex");
    assert_eq!(envelope["type"], "user");
    assert_eq!(envelope["parentUuid"], Value::Null);
    assert_eq!(envelope["requestId"], Value::Null);
    assert!(envelope["timestamp"].is_string());

    // @step And its message is the stored text
    assert_eq!(
        envelope["message"]["content"],
        json!([{"type": "text", "text": "hello"}])
    );
}

#[test]
#[serial]
fn scenario_the_compaction_summary_row_carries_a_real_envelope() {
    // @step Given a session compacted with a summary
    let (_directory, mut session) = bug332_session("claude");
    append_message(&mut session, "user", "before").unwrap();
    append_message(&mut session, "assistant", "reply").unwrap();
    set_compaction_state(&mut session, "earlier work".into(), 1).unwrap();

    // @step When its messages are loaded
    let messages = get_session_messages(&load_session(session.id).unwrap()).unwrap();

    // @step Then the summary row's metadata is a user message envelope from the compaction provider
    let summary = &messages[0];
    assert_eq!(summary.id, Uuid::nil());
    let metadata = &summary.metadata;
    assert_eq!(metadata["provider"], "compaction");
    assert_eq!(metadata["type"], "user");
    assert_eq!(metadata["parentUuid"], Value::Null);
    assert_eq!(metadata["requestId"], Value::Null);
    assert_eq!(
        metadata["timestamp"],
        json!(session_compacted_at(session.id))
    );
    assert_eq!(
        metadata["message"]["content"][0]["text"],
        json!(summary.content)
    );
}

fn session_compacted_at(id: Uuid) -> chrono::DateTime<Utc> {
    load_session(id)
        .unwrap()
        .compaction
        .expect("compacted")
        .compacted_at
}
