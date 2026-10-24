use rig::completion::{CompletionError, CompletionRequest};
use rig::message::{Message, UserContent};
use rig::streaming::RawStreamingToolCall;
use serde_json::json;
use std::sync::Mutex;

static LAST_REQUEST: Mutex<Vec<u8>> = Mutex::new(Vec::new());

pub(super) fn serialize(request: &CompletionRequest) -> Result<Vec<u8>, CompletionError> {
    Ok(serde_json::to_vec(&json!({
        "adapter":"stub", "preamble":request.preamble, "messages":request.chat_history,
        "documents":request.documents, "tools":request.tools,
        "temperature":request.temperature, "max_tokens":request.max_tokens,
        "tool_choice":request.tool_choice, "additional_params":request.additional_params
    }))?)
}

pub(super) fn record(request: &CompletionRequest) -> Result<(), CompletionError> {
    let bytes = serialize(request)?;
    *LAST_REQUEST.lock().map_err(|_| {
        CompletionError::RequestError("stub request capture lock unavailable".into())
    })? = bytes;
    super::record_request_history(request);
    Ok(())
}

pub fn last_request_bytes() -> Vec<u8> {
    LAST_REQUEST
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub(super) fn tool_call(
    request: &CompletionRequest,
) -> Result<Option<RawStreamingToolCall>, CompletionError> {
    let Some(value) = request
        .additional_params
        .as_ref()
        .and_then(|params| params.get("stub_tool_call"))
    else {
        return Ok(None);
    };
    let required = |key| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                CompletionError::RequestError(format!("stub_tool_call requires {key}").into())
            })
    };
    let identity = required("id")?;
    let name = required("name")?;
    let arguments = value
        .get("arguments")
        .filter(|arguments| arguments.is_object())
        .ok_or_else(|| {
            CompletionError::RequestError("stub_tool_call requires object arguments".into())
        })?;
    let completed = request.chat_history.iter().any(|message| match message {
        Message::User { content } => content.iter().any(|part| {
            matches!(part, UserContent::ToolResult(result) if result.id == identity || result.call_id.as_deref() == Some(identity))
        }),
        _ => false,
    });
    Ok((!completed)
        .then(|| RawStreamingToolCall::new(identity.into(), name.into(), arguments.clone())))
}
