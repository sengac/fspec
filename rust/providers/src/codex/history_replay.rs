use super::{CheckpointError, CodexItem};
use rig::completion::CompletionError;
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn validate(
    request: &mut Value,
    mut items: Vec<CodexItem>,
) -> Result<(), CompletionError> {
    let calls: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            CodexItem::FunctionCall { call_id, .. } => Some((call_id.clone(), false)),
            CodexItem::CustomToolCall { call_id, .. } => Some((call_id.clone(), true)),
            _ => None,
        })
        .collect();
    for (call_id, _) in &calls {
        if !items.iter().any(|item| output_id(item) == Some(call_id)) {
            return Err(CheckpointError::MissingOutcome(call_id.clone()).into());
        }
    }
    for item in &mut items {
        if let CodexItem::FunctionCallOutput { call_id, output } = item {
            if calls.contains(&(call_id.clone(), true)) {
                *item = CodexItem::CustomToolCallOutput {
                    call_id: call_id.clone(),
                    output: output.clone(),
                };
            }
        }
    }
    let mut emitted = HashSet::new();
    items.retain(|item| output_id(item).is_none_or(|call_id| emitted.insert(call_id.to_owned())));
    request["input"] = serde_json::to_value(items)?;
    Ok(())
}

fn output_id(item: &CodexItem) -> Option<&str> {
    match item {
        CodexItem::FunctionCallOutput { call_id, .. }
        | CodexItem::CustomToolCallOutput { call_id, .. } => Some(call_id),
        _ => None,
    }
}
