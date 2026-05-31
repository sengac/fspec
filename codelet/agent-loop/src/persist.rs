//! Session persistence helpers (RPC-072 lift; originally REFAC-007).
//!
//! Verbatim lift of `codelet/napi/src/persist.rs` (292 LOC). The
//! canonical NAPI implementation persists messages, tool results, token
//! state, and structural annotations to the on-disk session manifest
//! through `codelet_core::persistence::*` (those types and free
//! functions were lifted into codelet-core by RPC-031..RPC-034).
//!
//! Originally `pub(crate)` because the only consumer was the napi-side
//! `agent_loop`; in the RPC-072 lift these functions become `pub` so
//! the NAPI-free `agent_loop` in this crate can call them.
//!
//! The NAPI-side persist.rs continues to exist as an internal copy
//! while `codelet-napi` is alive, but the canonical home for these
//! helpers — for any NAPI-free caller — is this module.

use codelet_core::persistence::{
    append_message_with_metadata, load_session, update_session_tokens, AssistantContent,
    AssistantMessage, MessageEnvelope, MessagePayload, UserContent, UserMessage,
};

/// Persist a user message to the Rust persistence layer
///
/// This function creates a proper MessageEnvelope and stores it via the persistence module.
/// Called from agent_loop when user input is received.
pub fn persist_user_message(
    session_id: &uuid::Uuid,
    text: &str,
) -> std::result::Result<(), String> {
    use chrono::Utc;
    use std::collections::HashMap;

    // Load the session manifest
    let mut session_manifest = load_session(*session_id)?;

    // Create the message envelope
    let envelope = MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "user".to_string(), // User input, not from a provider
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Text {
                text: text.to_string(),
            }],
        }),
        request_id: None,
    };

    // Convert envelope to metadata map for storage
    let envelope_json = serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize envelope: {e}"))?;
    let metadata_map: HashMap<String, serde_json::Value> = serde_json::from_str(&envelope_json)
        .map_err(|e| format!("Failed to parse envelope as map: {e}"))?;

    // Store the message
    append_message_with_metadata(&mut session_manifest, "user", text, metadata_map)?;

    tracing::debug!("REFAC-007: Persisted user message for session {}", session_id);
    Ok(())
}

/// REFAC-007: Persist an assistant message with accumulated content blocks
pub fn persist_assistant_message_internal(
    session_id: &uuid::Uuid,
    provider: &str,
    content: Vec<AssistantContent>,
    stop_reason: Option<String>,
) -> std::result::Result<(), String> {
    use chrono::Utc;
    use std::collections::HashMap;

    // Load the session manifest
    let mut session_manifest = load_session(*session_id)?;

    // Create a simple text representation for the message content
    let text_content: String = content
        .iter()
        .map(|c| match c {
            AssistantContent::Text { text } => text.clone(),
            AssistantContent::ToolUse { name, .. } => format!("[Tool: {name}]"),
            AssistantContent::Thinking { thinking, .. } => {
                // Truncate at character boundaries to avoid panicking on multi-byte UTF-8
                let truncated: String = thinking.chars().take(50).collect();
                format!("[Thinking: {truncated}...]")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Create the message envelope
    let envelope = MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: provider.to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: None,
            model: None,
            content,
            // PROV-039: Use the real stop_reason from the streaming pipeline.
            // Use "unknown" instead of "end_turn" when stop_reason is None — this
            // distinguishes "the API said it ended normally" from "we don't know
            // why it ended" (e.g., Gemini before stop_reason was implemented).
            stop_reason: stop_reason.or_else(|| Some("unknown".to_string())),
            usage: None,
        }),
        request_id: None,
    };

    // Convert envelope to metadata map for storage
    let envelope_json = serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize envelope: {e}"))?;
    let metadata_map: HashMap<String, serde_json::Value> = serde_json::from_str(&envelope_json)
        .map_err(|e| format!("Failed to parse envelope as map: {e}"))?;

    // Store the message
    append_message_with_metadata(&mut session_manifest, "assistant", &text_content, metadata_map)?;

    tracing::debug!(
        "REFAC-007: Persisted assistant message for session {}",
        session_id
    );
    Ok(())
}

/// REFAC-007: Persist a tool result message
pub fn persist_tool_result_internal(
    session_id: &uuid::Uuid,
    tool_call_id: &str,
    content: &str,
    is_error: bool,
) -> std::result::Result<(), String> {
    use chrono::Utc;
    use std::collections::HashMap;

    // Load the session manifest
    let mut session_manifest = load_session(*session_id)?;

    // Create the message envelope with tool result
    let envelope = MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(), // Tool results are user messages
        provider: "tool".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: tool_call_id.to_string(),
                content: content.to_string(),
                is_error,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Convert envelope to metadata map for storage
    let envelope_json = serde_json::to_string(&envelope)
        .map_err(|e| format!("Failed to serialize envelope: {e}"))?;
    let metadata_map: HashMap<String, serde_json::Value> = serde_json::from_str(&envelope_json)
        .map_err(|e| format!("Failed to parse envelope as map: {e}"))?;

    // Store the message - use a truncated summary for the content field
    // Use char boundary check to avoid panicking on multi-byte UTF-8 characters
    let summary = if content.len() > 200 {
        let mut end = 200;
        while !content.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &content[..end])
    } else {
        content.to_string()
    };
    append_message_with_metadata(&mut session_manifest, "user", &summary, metadata_map)?;

    tracing::debug!(
        "REFAC-007: Persisted tool result for session {}",
        session_id
    );
    Ok(())
}

/// REFAC-007 Rule [31]: Persist token state to session manifest
pub fn persist_token_state(
    session_id: &uuid::Uuid,
    input_tokens: u32,
    output_tokens: u32,
) -> std::result::Result<(), String> {
    // Load the session manifest
    let mut session_manifest = load_session(*session_id)?;

    // Update token state (using cumulative update)
    update_session_tokens(
        &mut session_manifest,
        input_tokens as u64,
        output_tokens as u64,
        0, // cache_read - not tracked per-turn
        0, // cache_create - not tracked per-turn
    )?;

    tracing::debug!(
        "REFAC-007: Persisted token state for session {} (input={}, output={})",
        session_id,
        input_tokens,
        output_tokens
    );
    Ok(())
}

/// Persist structural annotations from the stream loop to message metadata.
pub fn persist_pending_annotations(
    session_id: &uuid::Uuid,
    session: &mut codelet_cli::session::Session,
) {
    if session.annotations.is_empty() {
        return;
    }

    use codelet_cli::session::system_reminders::is_system_reminder;

    let system_reminder_count = session
        .messages
        .iter()
        .filter(|m| is_system_reminder(m))
        .count();

    let session_manifest = match codelet_core::persistence::load_session(*session_id) {
        Ok(manifest) => manifest,
        Err(e) => {
            tracing::warn!(
                "[persist_pending_annotations] Failed to load session manifest: {}",
                e
            );
            session.annotations.clear();
            return;
        }
    };
    let persisted_messages =
        match codelet_core::persistence::get_session_messages_full(&session_manifest) {
            Ok(msgs) => msgs,
            Err(e) => {
                tracing::warn!(
                    "[persist_pending_annotations] Failed to load persisted messages: {}",
                    e
                );
                session.annotations.clear();
                return;
            }
        };

    for (msg_idx, annotations) in session.annotations.drain() {
        let Some(persisted_idx) = msg_idx.checked_sub(system_reminder_count) else {
            tracing::debug!(
                "[persist_pending_annotations] msg_idx {} < system_reminder_count {}, skipping",
                msg_idx,
                system_reminder_count
            );
            continue;
        };

        let Some(stored_msg) = persisted_messages.get(persisted_idx) else {
            tracing::debug!(
                "[persist_pending_annotations] persisted_idx {} out of range (len={}), skipping",
                persisted_idx,
                persisted_messages.len()
            );
            continue;
        };

        let annotations_json = match serde_json::to_value(&annotations) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "[persist_pending_annotations] Failed to serialize annotations: {}",
                    e
                );
                continue;
            }
        };

        let mut entries = std::collections::HashMap::new();
        entries.insert("annotations".to_string(), annotations_json);

        if let Err(e) = codelet_core::persistence::update_message_metadata(stored_msg.id, entries) {
            tracing::warn!(
                "[persist_pending_annotations] Failed to update metadata for {}: {}",
                stored_msg.id,
                e
            );
        }
    }
}
