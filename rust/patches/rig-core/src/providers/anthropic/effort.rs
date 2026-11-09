//! fspec patch (MODEL-015): Claude per-message effort.
//!
//! A request whose additional params carry [`EFFORT_MESSAGES_PARAM`], a list
//! of `{"index": n, "effort": "low"}`, gets an effort-only message
//! `{"role": "system", "content": [], "output_config": {"effort": "low"}}`
//! right before the chat-history message at each index, and the request is
//! sent with [`PER_MESSAGE_EFFORT_BETA`] added to its `anthropic-beta`. The
//! param itself is never sent. A malformed list or an index past the
//! history refuses the request.

use crate::completion::CompletionError;
use serde_json::{Value, json};

/// The reserved additional param naming the effort messages of a request.
pub const EFFORT_MESSAGES_PARAM: &str = "effort_messages";

/// The beta that lets an effort message change effort mid-conversation.
pub const PER_MESSAGE_EFFORT_BETA: &str = "mid-conversation-output-config-2026-07-01";

/// One effort message: the chat-history index it precedes and its effort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortMessage {
    pub index: usize,
    pub effort: String,
}

fn malformed(reason: &str) -> CompletionError {
    CompletionError::RequestError(format!("invalid {EFFORT_MESSAGES_PARAM}: {reason}").into())
}

/// Removes the effort messages from `params`; none when it carries none.
pub(crate) fn take(params: &mut Option<Value>) -> Result<Vec<EffortMessage>, CompletionError> {
    let Some(Value::Object(map)) = params else {
        return Ok(Vec::new());
    };
    let Some(list) = map.remove(EFFORT_MESSAGES_PARAM) else {
        return Ok(Vec::new());
    };
    let Value::Array(entries) = list else {
        return Err(malformed("not a list"));
    };
    let mut messages = Vec::with_capacity(entries.len());
    for entry in entries {
        let index = entry
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| malformed("an entry has no index"))?;
        let effort = entry
            .get("effort")
            .and_then(Value::as_str)
            .filter(|effort| !effort.is_empty())
            .ok_or_else(|| malformed("an entry has no effort"))?;
        if messages
            .last()
            .is_some_and(|last: &EffortMessage| last.index >= index)
        {
            return Err(malformed("indices must increase"));
        }
        messages.push(EffortMessage {
            index,
            effort: effort.to_string(),
        });
    }
    Ok(messages)
}

/// Whether `params` names any effort message (the request then needs the
/// per-message effort beta).
pub(crate) fn requested(params: &Option<Value>) -> bool {
    params
        .as_ref()
        .and_then(|params| params.get(EFFORT_MESSAGES_PARAM))
        .and_then(Value::as_array)
        .is_some_and(|list| !list.is_empty())
}

/// Inserts the effort messages into the serialized `messages` array, whose
/// chat history starts at `offset` (after any documents message).
pub(crate) fn insert(
    messages: &mut Value,
    effort_messages: &[EffortMessage],
    offset: usize,
) -> Result<(), CompletionError> {
    if effort_messages.is_empty() {
        return Ok(());
    }
    let Value::Array(messages) = messages else {
        return Err(malformed("the request messages are not a list"));
    };
    for effort_message in effort_messages.iter().rev() {
        let at = effort_message.index.saturating_add(offset);
        if at >= messages.len() {
            return Err(malformed(&format!(
                "index {} is past the conversation",
                effort_message.index
            )));
        }
        messages.insert(
            at,
            json!({
                "role": "system",
                "content": [],
                "output_config": {"effort": effort_message.effort},
            }),
        );
    }
    Ok(())
}

/// Adds the per-message effort beta to the request's `anthropic-beta`.
pub(crate) fn with_beta(
    mut builder: http::request::Builder,
    requested: bool,
) -> Result<http::request::Builder, CompletionError> {
    if !requested {
        return Ok(builder);
    }
    let Some(headers) = builder.headers_mut() else {
        return Err(malformed("the request headers are unavailable"));
    };
    let betas = match headers
        .get("anthropic-beta")
        .map(|value| value.to_str())
        .transpose()
        .map_err(|error| malformed(&error.to_string()))?
    {
        Some(existing)
            if existing
                .split(',')
                .any(|beta| beta.trim() == PER_MESSAGE_EFFORT_BETA) =>
        {
            existing.to_string()
        }
        Some(existing) if !existing.is_empty() => format!("{existing},{PER_MESSAGE_EFFORT_BETA}"),
        _ => PER_MESSAGE_EFFORT_BETA.to_string(),
    };
    let value =
        http::HeaderValue::from_str(&betas).map_err(|error| malformed(&error.to_string()))?;
    headers.insert("anthropic-beta", value);
    Ok(builder)
}
