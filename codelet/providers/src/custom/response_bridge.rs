//! Response bridge: Rhai `Dynamic` → `CompletionResponse` (PROV-063).
//!
//! The custom-provider Rhai contract has `parse_response(raw)` return a
//! map with the following shape:
//!
//! ```rhai
//! #{
//!     content: <string | array of parts>,
//!     stop_reason: <string>,
//!     usage: #{
//!         input_tokens: 1234,
//!         output_tokens: 567,
//!         cache_read_input_tokens: 200,
//!         cache_creation_input_tokens: 100,
//!         reasoning_tokens: 0,
//!     },
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
//!
//! PROV-103: The optional `usage` sub-map carries the same token fields
//! as the streaming `kind: "usage"` chunk. All fields are optional; the
//! bridge returns it alongside the `CompletionResponse` so the rig
//! bridge can surface it to the TUI SessionHeader without reparsing
//! the response body.

use codelet_common::{ContentPart, MessageContent};
use rhai::{Dynamic, Map};

use super::conversion::dynamic_to_json_value;
use super::error::CustomProviderError;
use super::stream::StreamUsage;
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
/// `CompletionResponse` plus an optional token-usage snapshot. The
/// usage snapshot is `StreamUsage::default()` (all `None`) when the
/// script does not surface usage data.
pub fn rhai_to_completion_response(
    value: Dynamic,
) -> Result<(CompletionResponse, StreamUsage), CustomProviderError> {
    // Accept either a map (expected) or a raw string (shorthand for a
    // plain-text response). The latter makes scripts that just return
    // `"hello"` Just Work.
    if let Ok(text) = value.clone().into_string() {
        return Ok((
            CompletionResponse {
                content: MessageContent::Text(text),
                stop_reason: StopReason::EndTurn,
            },
            StreamUsage::default(),
        ));
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

    // PROV-103: Extract optional `usage` sub-map. Missing / non-map →
    // empty snapshot. Individual fields are parsed with the same
    // forgiving semantics as the streaming `handle_usage` helper.
    let usage = map
        .get("usage")
        .cloned()
        .and_then(rhai::Dynamic::try_cast::<Map>)
        .map(usage_from_map)
        .unwrap_or_default();

    Ok((
        CompletionResponse {
            content,
            stop_reason,
        },
        usage,
    ))
}

/// Pull token counts out of a Rhai `usage` sub-map. All fields are
/// optional — missing keys become `None`.
fn usage_from_map(map: Map) -> StreamUsage {
    StreamUsage {
        input_tokens: token_count_from_map(&map, "input_tokens"),
        output_tokens: token_count_from_map(&map, "output_tokens"),
        cache_read_input_tokens: token_count_from_map(&map, "cache_read_input_tokens"),
        cache_creation_input_tokens: token_count_from_map(&map, "cache_creation_input_tokens"),
        reasoning_tokens: token_count_from_map(&map, "reasoning_tokens"),
    }
}

/// Same `u64` normalisation used by the streaming `handle_usage` helper,
/// inlined here to avoid a cross-module visibility hop.
fn token_count_from_map(map: &Map, key: &str) -> Option<u64> {
    let value = map.get(key)?.clone();
    if value.is_unit() {
        return None;
    }
    match value.as_int() {
        Ok(i) if i >= 0 => Some(i as u64),
        _ => None,
    }
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
                CustomProviderError::RhaiRuntimeError("content part must be a Map".to_string())
            })?;
            parts.push(part_from_map(entry_map)?);
        }
        return Ok(MessageContent::Parts(parts));
    }

    Err(CustomProviderError::RhaiRuntimeError(
        "content must be a string or array of parts".to_string(),
    ))
}
