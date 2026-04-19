//! Response bridge: Rhai `Dynamic` → `CompletionResponse` (PROV-063).
//!
//! The custom-provider Rhai contract has `parse_response(raw)` return a
//! map with the following shape:
//!
//! ```rhai
//! #{
//!     content: <string | array of parts>,
//!     stop_reason: <string>,
//! }
//! ```
//!
//! Where each `content` part is either `#{ type: "text", text }` or
//! `#{ type: "tool_use", id, name, input }`.
//!
//! The `stop_reason` string maps to `StopReason` as follows:
//! - `"end_turn"` | `"stop"` → `EndTurn`
//! - `"tool_use"` | `"tool_calls"` → `ToolUse`
//! - `"max_tokens"` | `"length"` → `MaxTokens`
//! - anything else → `EndTurn` (safe fallback)

use codelet_common::{ContentPart, MessageContent};
use rhai::{Dynamic, Map};

use super::conversion::dynamic_to_json_value;
use super::error::CustomProviderError;
use crate::{CompletionResponse, StopReason};

/// Map a Rhai stop-reason string to our `StopReason` enum.
fn map_stop_reason(raw: &str) -> StopReason {
    match raw {
        "tool_use" | "tool_calls" => StopReason::ToolUse,
        "max_tokens" | "length" => StopReason::MaxTokens,
        // "end_turn", "stop", and anything unrecognised fall through to EndTurn
        _ => StopReason::EndTurn,
    }
}

/// Extract a single `ContentPart` from a Rhai map entry.
fn part_from_map(part_map: Map) -> Result<ContentPart, CustomProviderError> {
    let part_type = part_map
        .get("type")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();

    match part_type.as_str() {
        "text" => {
            let text = part_map
                .get("text")
                .cloned()
                .and_then(|v| v.into_string().ok())
                .unwrap_or_default();
            Ok(ContentPart::Text { text })
        }
        "tool_use" => {
            let id = part_map
                .get("id")
                .cloned()
                .and_then(|v| v.into_string().ok())
                .unwrap_or_default();
            let name = part_map
                .get("name")
                .cloned()
                .and_then(|v| v.into_string().ok())
                .unwrap_or_default();
            let input_dyn = part_map.get("input").cloned().unwrap_or(Dynamic::UNIT);
            let input = dynamic_to_json_value(&input_dyn);
            Ok(ContentPart::ToolUse { id, name, input })
        }
        other => Err(CustomProviderError::RhaiRuntimeError(format!(
            "unknown content part type '{other}'"
        ))),
    }
}

/// Convert the raw Rhai `Dynamic` returned by `parse_response` into a
/// `CompletionResponse`.
pub fn rhai_to_completion_response(
    value: Dynamic,
) -> Result<CompletionResponse, CustomProviderError> {
    // Accept either a map (expected) or a raw string (shorthand for a
    // plain-text response). The latter makes scripts that just return
    // `"hello"` Just Work.
    if let Ok(text) = value.clone().into_string() {
        return Ok(CompletionResponse {
            content: MessageContent::Text(text),
            stop_reason: StopReason::EndTurn,
        });
    }

    let map = value.try_cast::<Map>().ok_or_else(|| {
        CustomProviderError::RhaiRuntimeError(
            "parse_response must return a Map (or string)".to_string(),
        )
    })?;

    let stop_reason = map
        .get("stop_reason")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .map(|s| map_stop_reason(&s))
        .unwrap_or(StopReason::EndTurn);

    let content_dyn = map.get("content").cloned().unwrap_or(Dynamic::UNIT);
    let content = extract_content(content_dyn)?;

    Ok(CompletionResponse {
        content,
        stop_reason,
    })
}

/// Turn the Rhai `content` field into a `MessageContent` — either plain
/// text or a list of parts.
fn extract_content(value: Dynamic) -> Result<MessageContent, CustomProviderError> {
    // Unit means empty — represent as an empty text block to keep the
    // invariant "Completion always has content" alive.
    if value.is_unit() {
        return Ok(MessageContent::Text(String::new()));
    }

    if let Ok(text) = value.clone().into_string() {
        return Ok(MessageContent::Text(text));
    }

    if value.is_array() {
        let arr = value.into_typed_array::<Dynamic>().map_err(|typ| {
            CustomProviderError::RhaiRuntimeError(format!(
                "content array conversion failed ({typ})"
            ))
        })?;
        let mut parts: Vec<ContentPart> = Vec::with_capacity(arr.len());
        for entry in arr {
            let entry_map = entry.try_cast::<Map>().ok_or_else(|| {
                CustomProviderError::RhaiRuntimeError(
                    "content part must be a Map".to_string(),
                )
            })?;
            parts.push(part_from_map(entry_map)?);
        }
        return Ok(MessageContent::Parts(parts));
    }

    Err(CustomProviderError::RhaiRuntimeError(
        "content must be a string or array of parts".to_string(),
    ))
}
