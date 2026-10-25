use super::{CheckpointError, CodexItem};
use crate::codex::custom_tools::CustomToolRegistry;
use serde_json::{json, Value};

pub(super) struct Projection {
    pub value: Value,
    pub start: usize,
    pub end: usize,
}

fn text_part(part: &Value) -> Option<&str> {
    if part["type"] != "output_text" || part.as_object()?.len() != 2 {
        return None;
    }
    part["text"].as_str()
}

fn assistant_text(item: &Value) -> Option<&str> {
    if item["type"] != "message" || item["role"] != "assistant" {
        return None;
    }
    let parts = item["content"].as_array()?;
    if parts.len() != 1 {
        return None;
    }
    text_part(&parts[0])
}

fn comparable(item: &CodexItem, tools: &CustomToolRegistry) -> Result<Value, CheckpointError> {
    let mut value = serde_json::to_value(item)
        .map_err(|_| CheckpointError::Lineage("unserializable history item"))?;
    if matches!(
        item,
        CodexItem::CustomToolCall { .. } | CodexItem::FunctionCall { .. }
    ) {
        if let Ok(normalized) = tools.normalize_call(&value) {
            value = normalized;
        }
    }
    if matches!(item, CodexItem::CustomToolCallOutput { .. }) {
        value["type"] = json!("function_call_output");
    }
    let assistant = value["role"] == "assistant";
    if let Some(object) = value.as_object_mut() {
        object.remove("id");
        object.remove("phase");
        if object.get("type").and_then(Value::as_str) == Some("function_call") {
            if let Some(arguments) = object.get("arguments").and_then(Value::as_str) {
                if let Ok(arguments) = serde_json::from_str::<Value>(arguments) {
                    object.insert("arguments".into(), arguments);
                }
            }
        }
        if let Some(parts) = object.get_mut("content").and_then(Value::as_array_mut) {
            let mut normalized = Vec::new();
            for mut part in std::mem::take(parts) {
                if let Some(object) = part.as_object_mut() {
                    object.remove("annotations");
                    object.remove("logprobs");
                }
                if assistant
                    && part["type"] == "refusal"
                    && part.as_object().is_some_and(|object| object.len() == 2)
                {
                    if let Some(text) = part["refusal"].as_str() {
                        part = json!({"type":"output_text", "text":text});
                    }
                }
                if let Some(previous) = normalized.last_mut() {
                    if let (Some(prefix), Some(suffix)) = (text_part(previous), text_part(&part)) {
                        previous["text"] = json!(format!("{prefix}{suffix}"));
                        continue;
                    }
                }
                normalized.push(part);
            }
            *parts = normalized;
        }
    }
    Ok(value)
}

pub(super) fn project(
    items: &[&CodexItem],
    tools: &CustomToolRegistry,
) -> Result<Vec<Projection>, CheckpointError> {
    let mut projected: Vec<Projection> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let value = comparable(item, tools)?;
        if let Some(previous) = projected.iter_mut().rev().find(|previous| {
            !matches!(
                previous.value["type"].as_str(),
                Some("reasoning" | "web_search_call")
            )
        }) {
            if let (Some(prefix), Some(suffix)) =
                (assistant_text(&previous.value), assistant_text(&value))
            {
                previous.value["content"][0]["text"] = json!(format!("{prefix}{suffix}"));
                previous.end = index + 1;
                continue;
            }
        }
        projected.push(Projection {
            value,
            start: index,
            end: index + 1,
        });
    }
    Ok(projected)
}
