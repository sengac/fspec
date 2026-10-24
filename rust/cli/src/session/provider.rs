use super::Session;
use codelet_core::persistence::{migrate_provider_state, persist_provider_state};
use codelet_providers::session::ExtensionError;
use rig::message::{AssistantContent, Message, UserContent};
use serde_json::Value;
use std::sync::Arc;

pub(super) fn conversational_message(envelope: &Value) -> Result<Option<Message>, String> {
    let Some(value) = envelope.get("message") else {
        return Ok(None);
    };
    if envelope.get("provider").and_then(Value::as_str) == Some("codex")
        && envelope.get("codexContinuation").is_some()
        && value.get("role").and_then(Value::as_str) == Some("assistant")
        && value
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        return Ok(None);
    }
    let message: rig::providers::anthropic::completion::Message =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    let mut message: Message = message
        .try_into()
        .map_err(|error: rig::message::MessageError| error.to_string())?;
    let blocks = value.get("content").and_then(Value::as_array);
    match &mut message {
        Message::Assistant { content, .. } => {
            for content in content.iter_mut() {
                if let AssistantContent::ToolCall(call) = content {
                    let block = blocks
                        .and_then(|blocks| blocks.iter().find(|block| block["id"] == call.id));
                    call.call_id = Some(invocation_identity(block, &call.id)?);
                }
            }
        }
        Message::User { content } => {
            for content in content.iter_mut() {
                if let UserContent::ToolResult(result) = content {
                    let block = blocks.and_then(|blocks| {
                        blocks
                            .iter()
                            .find(|block| block["tool_use_id"] == result.id)
                    });
                    result.call_id = Some(invocation_identity(block, &result.id)?);
                }
            }
        }
    }
    Ok(Some(message))
}

fn invocation_identity(block: Option<&Value>, item_id: &str) -> Result<String, String> {
    match block.and_then(|block| block.get("call_id")) {
        Some(Value::String(identity)) if !identity.is_empty() => Ok(identity.clone()),
        Some(_) => Err("invalid persisted tool invocation identity".into()),
        None if !item_id.is_empty() => Ok(item_id.to_owned()),
        None => Err("missing persisted tool invocation identity".into()),
    }
}

impl Session {
    pub fn begin_provider_turn(&self) {
        self.provider_session.begin_turn();
    }

    pub fn reset_provider_state(&mut self) -> Result<(), ExtensionError> {
        self.provider_session.reset()?;
        self.token_tracker.invalidate_input();
        Ok(())
    }

    pub fn restore_provider_state(&mut self, envelopes: &[String]) -> Result<(), ExtensionError> {
        self.provider_session
            .ensure_provider(self.provider_manager.current_provider_name());
        self.provider_session
            .restore_for_provider(self.provider_manager.current_provider_name(), envelopes)
    }

    pub fn bind_provider_persistence(
        &mut self,
        session_id: uuid::Uuid,
    ) -> Result<(), ExtensionError> {
        self.provider_session
            .ensure_provider(self.provider_manager.current_provider_name());
        migrate_provider_state(session_id)
            .map_err(|error| ExtensionError::Persistence(error.to_string()))?;
        self.provider_session.bind_writers(|provider| {
            let provider = provider.to_owned();
            Arc::new(move |metadata| {
                persist_provider_state(session_id, &provider, metadata)
                    .map_err(|error| ExtensionError::Persistence(error.to_string()))?;
                Ok(())
            })
        })
    }
}
