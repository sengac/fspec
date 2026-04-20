//! Rhai `Dynamic` → `Vec<StreamChunk>` conversion for the streaming
//! bridge (PROV-064). Kept in its own file so `stream.rs` stays under
//! the 300-line cap.

use rhai::{Dynamic, Map};

use super::stream::{RhaiStreamProcessor, StreamChunk};
use crate::StopReason;

/// Convert the Rhai `Dynamic` returned by `parse_stream_chunk` into a
/// list of emitted [`StreamChunk`]s. Mutates the processor's tool-call
/// accumulator and pending-stop state as needed.
pub(super) fn dynamic_to_chunks(
    processor: &mut RhaiStreamProcessor,
    value: Dynamic,
) -> Vec<StreamChunk> {
    // Unit / () → ignore.
    if value.is_unit() {
        return Vec::new();
    }

    // Array of maps → dispatch each entry.
    if value.is_array() {
        let arr = value
            .into_typed_array::<Dynamic>()
            .unwrap_or_default();
        let mut out = Vec::new();
        for entry in arr {
            out.extend(handle_one(processor, entry));
        }
        return out;
    }

    handle_one(processor, value)
}

fn handle_one(
    processor: &mut RhaiStreamProcessor,
    value: Dynamic,
) -> Vec<StreamChunk> {
    let Some(map) = value.try_cast::<Map>() else {
        tracing::warn!(
            "parse_stream_chunk returned a non-map value; skipping event"
        );
        return Vec::new();
    };
    let kind = map
        .get("kind")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();

    match kind.as_str() {
        "text_delta" | "text" => handle_text(&map),
        "reasoning_delta" | "thinking_delta" => handle_reasoning(&map),
        "tool_call_delta" | "tool_call" => handle_tool_call(processor, &map),
        "stop" => handle_stop(processor, &map),
        "ignore" | "" => Vec::new(),
        other => {
            tracing::debug!(kind = %other, "ignoring unknown stream chunk kind");
            Vec::new()
        }
    }
}

fn handle_text(map: &Map) -> Vec<StreamChunk> {
    match non_empty_text(map, "text") {
        Some(text) => vec![StreamChunk::TextDelta(text)],
        None => Vec::new(),
    }
}

/// PROV-089: Bridge a Rhai `reasoning_delta` / `thinking_delta` map into a
/// [`StreamChunk::ReasoningDelta`]. Mirrors [`handle_text`] — empty or
/// missing text is ignored so keepalive-shaped reasoning events do not
/// produce empty chunks.
fn handle_reasoning(map: &Map) -> Vec<StreamChunk> {
    match non_empty_text(map, "text") {
        Some(text) => vec![StreamChunk::ReasoningDelta(text)],
        None => Vec::new(),
    }
}

/// Extract a non-empty string value for `key` from `map`. Returns
/// `None` when the key is absent, not-a-string, or an empty string —
/// the canonical "skip this event" signal for text-bearing chunk
/// kinds. Shared by [`handle_text`] and [`handle_reasoning`] so the
/// empty-text guard lives in exactly one place.
fn non_empty_text(map: &Map, key: &str) -> Option<String> {
    map.get(key)
        .cloned()
        .and_then(|v| v.into_string().ok())
        .filter(|s| !s.is_empty())
}

fn handle_stop(
    processor: &mut RhaiStreamProcessor,
    map: &Map,
) -> Vec<StreamChunk> {
    let reason_str = map
        .get("reason")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .unwrap_or_default();
    let reason = match reason_str.as_str() {
        "tool_use" | "tool_calls" => StopReason::ToolUse,
        "max_tokens" | "length" => StopReason::MaxTokens,
        _ => StopReason::EndTurn,
    };
    processor.record_stop(reason);
    processor.mark_done()
}

fn handle_tool_call(
    processor: &mut RhaiStreamProcessor,
    map: &Map,
) -> Vec<StreamChunk> {
    let explicit_id = map
        .get("id")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .filter(|s| !s.is_empty());
    let index = map.get("index").and_then(|v| v.as_int().ok());

    // Resolve the accumulator key. Prefer `index` when present — OpenAI
    // only sends `id` on the first chunk, so later chunks must still
    // map to the same accumulator. If no `index`, fall back to the
    // explicit id (Anthropic-style "one tool per block" flows).
    let key = match (index, explicit_id.as_ref()) {
        (Some(i), _) => format!("__idx_{i}"),
        (None, Some(id)) => id.clone(),
        (None, None) => "__idx_0".to_string(),
    };

    let mut out = Vec::new();

    let name_opt = map
        .get("name")
        .cloned()
        .and_then(|v| v.into_string().ok())
        .filter(|s| !s.is_empty());
    let args_opt = map
        .get("arguments")
        .cloned()
        .and_then(|v| v.into_string().ok());
    let complete = map
        .get("complete")
        .and_then(|v| v.as_bool().ok())
        .unwrap_or(false);

    {
        let entry = processor.tool_call_entry(&key);
        // Prefer an explicit id over the synthesised one.
        if let Some(id) = &explicit_id {
            if entry.id.starts_with("__idx_") || entry.id.is_empty() {
                entry.id = id.clone();
            }
        }
        if let Some(name) = &name_opt {
            entry.name = name.clone();
        }
    }

    // Emit ToolCallStart the first time both id+name are known.
    let (effective_id, start_chunk) = {
        let entry = processor.tool_call_entry(&key);
        let effective_id = entry.id.clone();
        let start =
            if !entry.started && !entry.id.is_empty() && !entry.name.is_empty() {
                entry.started = true;
                Some(StreamChunk::ToolCallStart {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                })
            } else {
                None
            };
        (effective_id, start)
    };
    if let Some(c) = start_chunk {
        out.push(c);
    }

    if let Some(args) = args_opt {
        if !args.is_empty() {
            let entry = processor.tool_call_entry(&key);
            entry.arguments_json.push_str(&args);
            out.push(StreamChunk::ToolCallArgsDelta {
                id: effective_id,
                chunk: args,
            });
        }
    }

    if complete {
        // Flush just this accumulator.
        if let Some(chunk) = flush_one(processor, &key) {
            out.push(chunk);
        }
    }

    out
}

fn flush_one(processor: &mut RhaiStreamProcessor, key: &str) -> Option<StreamChunk> {
    processor.flush_single(key)
}
