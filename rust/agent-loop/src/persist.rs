use crate::persistence_retry::{submit, PersistenceSpool, RecoveryRecord};
use chrono::Utc;
use codelet_core::persistence::{
    load_session, AssistantContent, AssistantMessage, MessageEnvelope, MessagePayload, UserContent,
    UserMessage,
};

#[path = "persist_state.rs"]
mod state;
pub use state::{persist_pending_annotations, persist_token_state};

pub fn retry_pending_persistence(session_id: &uuid::Uuid) -> Result<(), String> {
    PersistenceSpool::open(*session_id)?.retry()
}

pub fn persist_user_message(session_id: &uuid::Uuid, text: &str) -> Result<(), String> {
    persist_user_content(session_id, vec![UserContent::Text { text: text.into() }])
}

pub fn persist_user_content(
    session_id: &uuid::Uuid,
    content: Vec<UserContent>,
) -> Result<(), String> {
    retry_pending_persistence(session_id)?;
    let summary = content
        .iter()
        .map(|block| match block {
            UserContent::Text { text } => text.clone(),
            UserContent::Image { .. } => "[Image]".into(),
            UserContent::Document { .. } => "[Document]".into(),
            UserContent::ToolResult { content, .. } => content.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let envelope = envelope(
        "user",
        "user",
        MessagePayload::User(UserMessage {
            role: "user".into(),
            content,
        }),
    );
    submit_envelope(session_id, envelope, summary, None)
}

pub fn persist_assistant_message_internal(
    session_id: &uuid::Uuid,
    provider: &str,
    content: Vec<AssistantContent>,
    stop_reason: Option<String>,
) -> Result<(), String> {
    persist_assistant_message_with_interruption(session_id, provider, content, stop_reason, None)
}

pub(crate) fn persist_assistant_message_with_interruption(
    session_id: &uuid::Uuid,
    provider: &str,
    content: Vec<AssistantContent>,
    stop_reason: Option<String>,
    reason: Option<codelet_rpc_types::InterruptionReason>,
) -> Result<(), String> {
    persist_assistant_message_identified(
        session_id,
        provider,
        uuid::Uuid::new_v4(),
        content,
        stop_reason,
        reason,
    )
}

pub fn persist_assistant_message_identified(
    session_id: &uuid::Uuid,
    provider: &str,
    id: uuid::Uuid,
    content: Vec<AssistantContent>,
    stop_reason: Option<String>,
    reason: Option<codelet_rpc_types::InterruptionReason>,
) -> Result<(), String> {
    let summary = content
        .iter()
        .map(|block| match block {
            AssistantContent::Text { text } => text.clone(),
            AssistantContent::ToolUse { name, .. } => format!("[Tool: {name}]"),
            AssistantContent::Thinking { thinking, .. } => format!(
                "[Thinking: {}...]",
                thinking.chars().take(50).collect::<String>()
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut envelope = envelope(
        "assistant",
        provider,
        MessagePayload::Assistant(AssistantMessage {
            role: "assistant".into(),
            id: None,
            model: None,
            content,
            stop_reason: stop_reason.or_else(|| Some("unknown".into())),
            usage: None,
        }),
    );
    envelope.uuid = id;
    submit_envelope(session_id, envelope, summary, reason)
}

pub fn persist_tool_result_internal(
    session_id: &uuid::Uuid,
    tool_call_id: &str,
    content: &str,
    is_error: bool,
) -> Result<(), String> {
    persist_tool_result_with_call_id_internal(session_id, tool_call_id, None, content, is_error)
}

pub fn persist_tool_result_with_call_id_internal(
    session_id: &uuid::Uuid,
    tool_call_id: &str,
    call_id: Option<&str>,
    content: &str,
    is_error: bool,
) -> Result<(), String> {
    persist_tool_result_identified(
        session_id,
        uuid::Uuid::new_v4(),
        tool_call_id,
        call_id,
        content,
        is_error,
    )
}

pub fn persist_tool_result_identified(
    session_id: &uuid::Uuid,
    id: uuid::Uuid,
    tool_call_id: &str,
    call_id: Option<&str>,
    content: &str,
    is_error: bool,
) -> Result<(), String> {
    let summary = if content.len() > 200 {
        format!("{}...", content.chars().take(200).collect::<String>())
    } else {
        content.into()
    };
    let mut envelope = envelope(
        "user",
        "tool",
        MessagePayload::User(UserMessage {
            role: "user".into(),
            content: vec![UserContent::ToolResult {
                tool_use_id: tool_call_id.into(),
                call_id: call_id.map(str::to_owned),
                content: content.into(),
                is_error,
                tool_use_result: None,
            }],
        }),
    );
    envelope.uuid = id;
    submit_envelope(session_id, envelope, summary, None)
}

fn envelope(role: &str, provider: &str, message: MessagePayload) -> MessageEnvelope {
    MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: role.into(),
        provider: provider.into(),
        message,
        request_id: None,
    }
}

fn submit_envelope(
    session_id: &uuid::Uuid,
    envelope: MessageEnvelope,
    content: String,
    reason: Option<codelet_rpc_types::InterruptionReason>,
) -> Result<(), String> {
    load_session(*session_id)
        .map_err(|_| "persistence admission failed; original remains with caller")?;
    let mut metadata: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_value(
            serde_json::to_value(&envelope).map_err(|_| "envelope encoding failed")?,
        )
        .map_err(|_| "envelope metadata encoding failed")?;
    if let Some(reason) = reason {
        metadata.insert(
            "interruptionReason".into(),
            serde_json::to_value(reason).map_err(|_| "interruption encoding failed")?,
        );
    }
    submit(
        *session_id,
        RecoveryRecord {
            version: 1,
            id: envelope.uuid,
            role: envelope.message_type,
            content,
            metadata,
        },
    )
}
