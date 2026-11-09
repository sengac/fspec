//! COV-057: envelope metadata (identity, threading, timing, model,
//! provider, request id, stop reason, message id, usage) survives a real
//! save and cold restore of the session's message store.

use super::persisted::{assistant, assistant_of, fresh, reply, restore, save, text, Outcome};
use super::*;
use codelet_core::persistence::TokenUsagePerMessage;

/// Save one assistant message and return it as restored.
fn round_trip(session: uuid::Uuid, envelope: MessageEnvelope) -> Outcome<MessageEnvelope> {
    save(session, &[envelope])?;
    let mut restored = restore(session)?;
    assert_eq!(restored.len(), 1, "restored {restored:?}");
    Ok(restored.remove(0))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_message_uuid_preserved_for_identity() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message with uuid "a6bdbefb-902d-4f98-b539-8cbee91ec831"
    let mut envelope = reply(assistant(vec![text("identified")]));
    envelope.uuid = uuid::Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831")?;
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    // @step Then the restored message uuid equals "a6bdbefb-902d-4f98-b539-8cbee91ec831"
    assert_eq!(
        restored.uuid.to_string(),
        "a6bdbefb-902d-4f98-b539-8cbee91ec831"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_parent_uuid_preserved_for_message_threading() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with messages:
    let mut original: Vec<MessageEnvelope> = Vec::new();
    for (index, role) in ["user", "assistant", "user", "assistant"]
        .iter()
        .enumerate()
    {
        let mut envelope = if *role == "user" {
            super::persisted::user(vec![super::persisted::user_text(&format!("msg-{index}"))])
        } else {
            reply(assistant(vec![text(&format!("msg-{index}"))]))
        };
        envelope.parent_uuid = original.last().map(|previous| previous.uuid);
        original.push(envelope);
    }
    // @step When the session is saved and restored
    save(session, &original)?;
    let restored = restore(session)?;
    assert_eq!(restored.len(), 4);
    // @step Then each message parentUuid matches its predecessor's uuid
    assert_eq!(restored[0].parent_uuid, None);
    for pair in restored.windows(2) {
        assert_eq!(pair[1].parent_uuid, Some(pair[0].uuid));
    }
    // @step And message threading can be reconstructed
    let mut thread = Vec::new();
    let mut cursor = restored
        .iter()
        .find(|message| message.parent_uuid.is_none());
    while let Some(message) = cursor {
        thread.push((message.uuid, message.message_type.clone()));
        cursor = restored
            .iter()
            .find(|child| child.parent_uuid == Some(message.uuid));
    }
    let expected: Vec<_> = original
        .iter()
        .map(|message| (message.uuid, message.message_type.clone()))
        .collect();
    assert_eq!(thread, expected);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_message_timestamp_preserved_for_ordering() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message with timestamp "2025-12-23T08:51:44.813Z"
    let mut envelope = reply(assistant(vec![text("timed")]));
    envelope.timestamp = "2025-12-23T08:51:44.813Z".parse()?;
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    // @step Then the restored message timestamp equals "2025-12-23T08:51:44.813Z"
    let stamp = restored
        .timestamp
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    assert_eq!(stamp, "2025-12-23T08:51:44.813Z");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_model_identifier_preserved_per_message() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message:
    let envelope = reply(AssistantMessage {
        model: Some("claude-opus-4-5-20251101".into()),
        ..assistant(vec![text("Response from Opus")])
    });
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    let message = assistant_of(&restored)?;
    // @step Then the restored message model equals "claude-opus-4-5-20251101"
    assert_eq!(message.model.as_deref(), Some("claude-opus-4-5-20251101"));
    assert_eq!(message.content, vec![text("Response from Opus")]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_provider_type_preserved_per_message() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message:
    let mut envelope = reply(assistant(vec![text("Response from Claude")]));
    envelope.provider = "claude".into();
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    // @step Then the restored message provider equals "claude"
    assert_eq!(restored.provider, "claude");
    assert_eq!(
        assistant_of(&restored)?.model.as_deref(),
        Some("claude-opus-4-5-20251101")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_mixed_provider_session_preserved_correctly() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with messages from different providers:
    let rows = [
        ("claude", "claude-opus-4-5-20251101", "Response from Claude"),
        ("openai", "gpt-4-turbo", "Response from OpenAI"),
        ("gemini", "gemini-pro", "Response from Gemini"),
    ];
    let envelopes: Vec<MessageEnvelope> = rows
        .iter()
        .map(|(provider, model, body)| {
            let mut envelope = reply(AssistantMessage {
                model: Some((*model).into()),
                ..assistant(vec![text(body)])
            });
            envelope.provider = (*provider).into();
            envelope
        })
        .collect();
    // @step When the session is saved and restored
    save(session, &envelopes)?;
    let restored = restore(session)?;
    assert_eq!(restored.len(), 3);
    // @step Then message 0 provider equals "claude"
    assert_eq!(restored[0].provider, "claude");
    // @step And message 1 provider equals "openai"
    assert_eq!(restored[1].provider, "openai");
    // @step And message 2 provider equals "gemini"
    assert_eq!(restored[2].provider, "gemini");
    for (envelope, (_, model, _)) in restored.iter().zip(rows) {
        assert_eq!(assistant_of(envelope)?.model.as_deref(), Some(model));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_api_request_id_preserved_for_debugging() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message with requestId "req_011CWPLKJZcigRWVriKduWSr"
    let mut envelope = reply(assistant(vec![text("traced")]));
    envelope.request_id = Some("req_011CWPLKJZcigRWVriKduWSr".into());
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    // @step Then the restored message requestId equals "req_011CWPLKJZcigRWVriKduWSr"
    assert_eq!(
        restored.request_id.as_deref(),
        Some("req_011CWPLKJZcigRWVriKduWSr")
    );
    Ok(())
}

/// Save an assistant message with `stop_reason` and return the restored one.
async fn stop_reason_round_trip(stop_reason: &str, body: &str) -> Outcome<AssistantMessage> {
    let (_directory, _manager, _, session) = fresh().await?;
    let envelope = reply(AssistantMessage {
        stop_reason: Some(stop_reason.into()),
        ..assistant(vec![text(body)])
    });
    let restored = round_trip(session, envelope)?;
    let message = assistant_of(&restored)?.clone();
    assert_eq!(message.content, vec![text(body)]);
    Ok(message)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_stop_reason_preserved_for_response_analysis() -> TestResult {
    // @step Given a session with an assistant message:
    // @step When the session is saved and restored
    let message = stop_reason_round_trip("end_turn", "Complete response").await?;
    // @step Then the restored message stop_reason equals "end_turn"
    assert_eq!(message.stop_reason.as_deref(), Some("end_turn"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_max_tokens_stop_reason_indicates_truncation() -> TestResult {
    // @step Given a session with an assistant message:
    // @step When the session is saved and restored
    let body = "Truncated response that was cut off mid-";
    let message = stop_reason_round_trip("max_tokens", body).await?;
    // @step Then the restored message stop_reason equals "max_tokens"
    assert_eq!(message.stop_reason.as_deref(), Some("max_tokens"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_assistant_message_id_preserved() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message:
    let envelope = reply(AssistantMessage {
        id: Some("msg_01Abc123XYZ".into()),
        ..assistant(vec![text("Here is my response")])
    });
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    let message = assistant_of(&restored)?;
    // @step Then the restored assistant message id equals "msg_01Abc123XYZ"
    assert_eq!(message.id.as_deref(), Some("msg_01Abc123XYZ"));
    assert_eq!(message.content, vec![text("Here is my response")]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_per_message_token_usage_preserved() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message with usage:
    let usage = TokenUsagePerMessage {
        input_tokens: 500,
        output_tokens: 150,
        cache_read_input_tokens: Some(200),
        cache_creation_input_tokens: Some(100),
    };
    let envelope = reply(AssistantMessage {
        usage: Some(usage.clone()),
        ..assistant(vec![text("metered")])
    });
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    // @step Then the restored message has usage metadata
    let restored_usage = assistant_of(&restored)?
        .usage
        .clone()
        .ok_or("usage metadata lost")?;
    assert_eq!(restored_usage, usage);
    // @step And the restored message usage input_tokens equals 500
    assert_eq!(restored_usage.input_tokens, 500);
    // @step And the restored message usage output_tokens equals 150
    assert_eq!(restored_usage.output_tokens, 150);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_assistant_message_without_id_handled_gracefully() -> TestResult {
    let (_directory, _manager, _, session) = fresh().await?;
    // @step Given a session with an assistant message:
    let envelope = reply(assistant(vec![text("Response without ID")]));
    // @step When the session is saved and restored
    let restored = round_trip(session, envelope)?;
    let message = assistant_of(&restored)?;
    // @step Then the restored assistant message id is null
    assert_eq!(message.id, None);
    // @step And the restored message text equals "Response without ID"
    assert_eq!(message.content, vec![text("Response without ID")]);
    Ok(())
}
