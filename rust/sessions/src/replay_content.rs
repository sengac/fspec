use codelet_rpc_types::{StreamChunk, ToolCallInfo, ToolResultInfo};
use rig::message::{AssistantContent, Message, Reasoning, ToolCall, ToolFunction, UserContent};
use rig::OneOrMany;
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error(transparent)]
    Media(#[from] crate::replay_media::MediaError),
    #[error("invalid replay JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("incompatible replay content: {0}")]
    Content(String),
    #[error("incompatible reminder identity")]
    Identity,
    #[error("replay revision conflict: {0}")]
    Conflict(String),
    #[error("session restore failed: {0}")]
    Store(String),
}

fn invocation_identity(block: &Value, item_id: &str) -> Result<String, RestoreError> {
    match block.get("call_id") {
        Some(Value::String(identity)) if !identity.is_empty() => Ok(identity.clone()),
        Some(_) => Err(RestoreError::Content(
            "invalid tool invocation identity".into(),
        )),
        None if !item_id.is_empty() => Ok(item_id.to_owned()),
        None => Err(RestoreError::Content(
            "missing tool invocation identity".into(),
        )),
    }
}

pub struct Replay {
    pub envelopes: Vec<String>,
    pub messages: Vec<Message>,
    pub chunks: Vec<StreamChunk>,
    pub identity: Option<Value>,
}

impl Replay {
    pub fn parse(envelopes: &[String]) -> Result<Self, RestoreError> {
        let mut replay = Self {
            envelopes: Vec::new(),
            messages: Vec::new(),
            chunks: Vec::new(),
            identity: None,
        };
        let mut identities = Vec::new();
        for encoded in envelopes {
            let envelope: Value = serde_json::from_str(encoded)?;
            if let Some(record) = envelope.get("_transcript") {
                if envelope.as_object().is_none_or(|fields| fields.len() != 1) {
                    return Err(RestoreError::Identity);
                }
                replay
                    .chunks
                    .extend(crate::replay_transcript::decode(record)?);
                continue;
            }
            identities.push(envelope.get("_sessionReplay").cloned());
            let summary = envelope.get("_compactionSummary");
            let reminder = envelope.get("_runtimeReminder");
            if summary.is_some_and(|value| value != &json!(true))
                || (summary.is_some() && reminder.is_some())
            {
                return Err(RestoreError::Identity);
            }
            if let Some(reminder) = reminder {
                if reminder != &json!({"version":1,"replaceable":true}) {
                    return Err(RestoreError::Identity);
                }
                continue;
            }
            if let Some(message) = envelope.get("message") {
                replay.convert(message)?;
            } else if envelope.get("_sessionReplay").is_none() {
                return Err(RestoreError::Content("missing message".into()));
            }
            replay.envelopes.push(encoded.clone());
        }
        replay.identity = identities.first().cloned().flatten();
        if identities
            .iter()
            .any(|identity| identity != &replay.identity)
        {
            return Err(RestoreError::Conflict("mixed replay snapshots".into()));
        }
        Ok(replay)
    }

    fn convert(&mut self, message: &Value) -> Result<(), RestoreError> {
        let role = required(message, "role")?;
        if !matches!(role, "user" | "assistant") {
            return Err(RestoreError::Content(format!("unsupported role {role}")));
        }
        let content = message
            .get("content")
            .ok_or_else(|| RestoreError::Content("missing content".into()))?;
        let blocks = match content {
            Value::String(text) => vec![json!({"type":"text","text":text})],
            Value::Array(blocks) => blocks.clone(),
            _ => {
                return Err(RestoreError::Content(
                    "content must be text or blocks".into(),
                ))
            }
        };
        let mut assistant = Vec::new();
        let mut user = Vec::new();
        for block in blocks {
            match (role, required(&block, "type")?) {
                ("assistant", "text") => {
                    let text = required(&block, "text")?;
                    assistant.push(AssistantContent::text(text));
                    self.chunks.push(StreamChunk::text(text.into()));
                }
                ("assistant", "thinking") => {
                    let thinking = required(&block, "thinking")?;
                    let signature = block
                        .get("signature")
                        .map(|_| required(&block, "signature"))
                        .transpose()?
                        .map(str::to_owned);
                    assistant.push(AssistantContent::Reasoning(
                        Reasoning::new(thinking).with_signature(signature),
                    ));
                    self.chunks.push(StreamChunk::thinking(thinking.into()));
                }
                ("assistant", "tool_use") => {
                    let id = required(&block, "id")?;
                    let name = required(&block, "name")?;
                    let input = block
                        .get("input")
                        .ok_or_else(|| RestoreError::Content("missing tool input".into()))?;
                    assistant.push(AssistantContent::ToolCall(
                        ToolCall::new(id.into(), ToolFunction::new(name.into(), input.clone()))
                            .with_call_id(invocation_identity(&block, id)?),
                    ));
                    self.chunks.push(StreamChunk::tool_call(ToolCallInfo {
                        id: id.into(),
                        name: name.into(),
                        input: serde_json::to_string(input)?,
                    }));
                }
                ("user", "text") => {
                    let text = required(&block, "text")?;
                    user.push(UserContent::text(text));
                    self.chunks.push(StreamChunk::user_input(text.into()));
                }
                ("user", "image" | "document") => {
                    let (content, chunk) = crate::replay_media::convert(&block)?;
                    user.push(content);
                    self.chunks.push(chunk);
                }
                ("user", "tool_result") => {
                    let id = required(&block, "tool_use_id")?;
                    let content = required(&block, "content")?;
                    user.push(UserContent::tool_result_with_call_id(
                        id,
                        invocation_identity(&block, id)?,
                        OneOrMany::one(rig::message::ToolResultContent::text(content)),
                    ));
                    self.chunks.push(StreamChunk::tool_result(ToolResultInfo {
                        tool_call_id: id.into(),
                        content: content.into(),
                        is_error: block
                            .get("is_error")
                            .map(|value| {
                                value.as_bool().ok_or_else(|| {
                                    RestoreError::Content("invalid result status".into())
                                })
                            })
                            .transpose()?
                            .unwrap_or(false),
                    }));
                }
                (_, kind) => {
                    return Err(RestoreError::Content(format!(
                        "unsupported {role} block {kind}"
                    )))
                }
            }
        }
        if !assistant.is_empty() {
            self.messages.push(Message::Assistant {
                id: message.get("id").and_then(Value::as_str).map(str::to_owned),
                content: OneOrMany::many(assistant)
                    .map_err(|error| RestoreError::Content(error.to_string()))?,
            });
        }
        if !user.is_empty() {
            self.messages.push(Message::User {
                content: OneOrMany::many(user)
                    .map_err(|error| RestoreError::Content(error.to_string()))?,
            });
        }
        self.chunks.push(StreamChunk::done());
        Ok(())
    }
}

fn required<'value>(value: &'value Value, field: &str) -> Result<&'value str, RestoreError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| RestoreError::Content(format!("missing or invalid {field}")))
}
