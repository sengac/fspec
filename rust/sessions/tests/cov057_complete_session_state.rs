//! Feature: spec/features/complete-session-state-persistence-and-restoration.feature
//!
//! COV-057: per-message model metadata through the real message store
//! (save, process-style store reset, reload), and continuous-vs-restored
//! equivalence measured on the provider request each session actually sends
//! (real `run_agent_stream` turns against an offline wiremock Anthropic
//! endpoint, real persistence, real cold `resume_session`). The submodules
//! cover envelope metadata, content blocks, structure, blob storage and
//! restored session state through the same real save and cold restore.

#[path = "cov057_complete_session_state/content.rs"]
mod content;
#[path = "cov057_complete_session_state/envelope.rs"]
mod envelope;
#[path = "cov057_complete_session_state/media_blobs.rs"]
mod media_blobs;
#[path = "cov057_complete_session_state/persisted.rs"]
mod persisted;
#[path = "cov057_complete_session_state/restored_compaction.rs"]
mod restored_compaction;
#[path = "cov057_complete_session_state/session_state.rs"]
mod session_state;
#[path = "cov057_complete_session_state/structure.rs"]
mod structure;
#[path = "cmpct060/support.rs"]
pub mod support;
#[path = "cov057_support/wire.rs"]
pub mod wire;

use codelet_core::persistence::{
    append_message_with_metadata, get_session_messages, load_session, reset_stores_for_tests,
    AssistantContent, AssistantMessage, MessageEnvelope, MessagePayload,
};
use codelet_rpc_types::SessionId;
use std::collections::HashMap;
use support::{cold_resume, manager, TestResult};
use wire::turn;

fn assistant_envelope(model: &str, text: &str) -> MessageEnvelope {
    MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: chrono::Utc::now(),
        message_type: "assistant".into(),
        provider: "claude".into(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".into(),
            id: None,
            model: Some(model.into()),
            content: vec![AssistantContent::Text { text: text.into() }],
            stop_reason: Some("end_turn".into()),
            usage: None,
        }),
        request_id: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_mixed_model_session_preserved_correctly() -> TestResult {
    let (_directory, _manager, session_id, _) = manager().await?;
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    // @step Given a session with messages from different models:
    let rows = [
        ("claude-sonnet-4-20250514", "Response from Sonnet"),
        ("claude-opus-4-5-20251101", "Response from Opus"),
    ];
    let mut session = load_session(uuid)?;
    for (model, text) in rows {
        let envelope = serde_json::to_value(assistant_envelope(model, text))?;
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_value(envelope)?;
        append_message_with_metadata(&mut session, "assistant", text, metadata)?;
    }
    // @step When the session is saved and restored
    reset_stores_for_tests();
    let restored = get_session_messages(&load_session(uuid)?)?
        .into_iter()
        .map(|stored| {
            let value = serde_json::to_value(stored.metadata)?;
            Ok(serde_json::from_value::<MessageEnvelope>(value)?)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let models: Vec<Option<String>> = restored
        .iter()
        .filter_map(|envelope| match &envelope.message {
            MessagePayload::Assistant(message) => Some(message.model.clone()),
            MessagePayload::User(_) => None,
        })
        .collect();
    assert_eq!(models.len(), 2, "restored {restored:?}");
    // @step Then message 0 model equals "claude-sonnet-4-20250514"
    assert_eq!(models[0].as_deref(), Some("claude-sonnet-4-20250514"));
    // @step And message 1 model equals "claude-opus-4-5-20251101"
    assert_eq!(models[1].as_deref(), Some("claude-opus-4-5-20251101"));
    Ok(())
}

/// The two strings around the first byte where they differ.
fn first_difference(a: &str, b: &str) -> String {
    let at = a
        .bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    let window = |text: &str| {
        let start = at.saturating_sub(120);
        let end = (at + 240).min(text.len());
        text.get(start..end).unwrap_or(text).to_string()
    };
    format!(
        "continuous: ...{}...\nrestored:   ...{}...",
        window(a),
        window(b)
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_restored_session_indistinguishable_from_continuous_session() -> TestResult {
    // @step Given two sessions with identical initial prompts
    let (directory, continuous, session_a, _) = manager().await?;
    let session_b = SessionId::new(
        continuous
            .create_session(
                "anthropic/claude-sonnet-4",
                &directory.path().to_string_lossy(),
            )
            .await?,
    );
    let prompts = ["Plan the parser", "Write the tokenizer", "Add error spans"];
    // @step And session A continues without interruption for 3 turns
    // @step And session B is saved after 3 turns and restored
    for (index, prompt) in prompts.iter().enumerate() {
        let reply = format!("Step {index} done");
        turn(&continuous, &session_a, prompt, &reply, 40).await?;
        turn(&continuous, &session_b, prompt, &reply, 40).await?;
    }
    let restored = cold_resume(&session_b)?;
    // @step When the messages array is serialized to API format for both sessions
    let wire_a = turn(&continuous, &session_a, "Summarise progress", "ok", 90).await?;
    let wire_b = turn(&restored, &session_b, "Summarise progress", "ok", 90).await?;
    // @step Then the serialized messages are byte-identical
    let (messages_a, messages_b) = (
        serde_json::to_string(&wire_a["messages"])?,
        serde_json::to_string(&wire_b["messages"])?,
    );
    assert!(
        messages_a == messages_b,
        "requests diverge:\n{}",
        first_difference(&messages_a, &messages_b)
    );
    assert_eq!(
        serde_json::to_string(&wire_a["system"])?,
        serde_json::to_string(&wire_b["system"])?
    );
    // @step And both sessions have the same message count
    let a = continuous.get_session(&session_a.value)?;
    let b = restored.get_session(&session_b.value)?;
    let (inner_a, inner_b) = (a.inner.lock().await, b.inner.lock().await);
    assert_eq!(inner_a.messages.len(), inner_b.messages.len());
    // @step And both sessions have matching token counts
    assert_eq!(
        inner_a.token_tracker.input_tokens,
        inner_b.token_tracker.input_tokens
    );
    assert_eq!(
        inner_a.token_tracker.output_tokens,
        inner_b.token_tracker.output_tokens
    );
    Ok(())
}
