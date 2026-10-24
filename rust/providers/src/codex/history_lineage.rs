use super::{CheckpointError, CodexCheckpoint, CodexItem};
use serde_json::Value;
use std::collections::VecDeque;

fn is_call(value: &Value) -> bool {
    matches!(
        value["type"].as_str(),
        Some("function_call" | "custom_tool_call")
    )
}

fn call_position(
    current: &VecDeque<super::projection::Projection>,
    expected: &Value,
    batch: &[&Value],
) -> Result<Option<usize>, CheckpointError> {
    for (index, next) in current.iter().enumerate() {
        if is_call(&next.value) {
            return if next.value == *expected {
                Ok(Some(index))
            } else {
                Err(CheckpointError::Lineage("tool call ancestry"))
            };
        }
        if !matches!(
            next.value["type"].as_str(),
            Some("function_call_output" | "custom_tool_call_output")
        ) || !batch
            .iter()
            .any(|call| call["call_id"] == next.value["call_id"])
        {
            break;
        }
    }
    Ok(None)
}

pub(super) fn assemble(
    request: &Value,
    saved: CodexCheckpoint,
    incoming: Vec<CodexItem>,
    tools: &crate::codex::custom_tools::CustomToolRegistry,
) -> Result<Vec<CodexItem>, CheckpointError> {
    if saved.protocol != "full-current-dag" {
        return Err(CheckpointError::Lineage("protocol"));
    }
    if saved.model.is_none() && saved.items.is_empty() && saved.generation > 0 {
        return Ok(incoming);
    }
    if saved.model.as_deref() != request["model"].as_str() {
        return Err(CheckpointError::Lineage("model"));
    }
    let prior: Vec<_> = saved
        .items
        .iter()
        .filter(|item| !item.is_reminder())
        .collect();
    let current: Vec<_> = incoming.iter().filter(|item| !item.is_reminder()).collect();
    let prior_projection = super::projection::project(&prior, tools)?;
    let mut current_projection: VecDeque<_> = super::projection::project(&current, tools)?.into();
    let mut consumed = vec![false; current.len()];
    let mut batch = Vec::new();
    for item in &prior_projection {
        let tool_call = is_call(&item.value);
        if !tool_call {
            batch.clear();
        }
        let position = if current_projection
            .front()
            .is_some_and(|next| next.value == item.value)
        {
            Some(0)
        } else if tool_call {
            call_position(&current_projection, &item.value, &batch)?
        } else {
            None
        };
        if let Some(matched) = position.and_then(|index| current_projection.remove(index)) {
            consumed[matched.start..matched.end].fill(true);
            if tool_call {
                batch.push(&item.value);
            }
        } else if !tool_call
            && !matches!(
                item.value["type"].as_str(),
                Some("reasoning" | "web_search_call")
            )
        {
            return Err(CheckpointError::Lineage(
                "assistant, tool result or DAG ancestry",
            ));
        }
    }
    let mut replay: Vec<_> = incoming
        .iter()
        .filter(|item| item.is_reminder())
        .cloned()
        .collect();
    replay.extend(prior.into_iter().cloned());
    replay.extend(
        current
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !consumed[*index])
            .map(|(_, item)| item.clone()),
    );
    Ok(replay)
}
