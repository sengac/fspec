use super::CodexResponsesModel;
use crate::codex::history::{CheckpointError, CodexItem};
use rig::completion::CompletionError;
use rig::message::{Message, ToolResult, UserContent};
use rig::providers::openai::responses_api::InputItem;
use rig::OneOrMany;
use serde_json::json;

fn outcome_id(item: &CodexItem) -> Option<&str> {
    match item {
        CodexItem::FunctionCallOutput { call_id, .. }
        | CodexItem::CustomToolCallOutput { call_id, .. } => Some(call_id),
        _ => None,
    }
}

fn outcome(result: &ToolResult, custom: bool) -> Result<CodexItem, CompletionError> {
    let items: Vec<InputItem> = Message::User {
        content: OneOrMany::one(UserContent::ToolResult(result.clone())),
    }
    .try_into()?;
    let mut items = items.into_iter();
    let item = items
        .next()
        .ok_or(CheckpointError::Lineage("settlement result encoding"))?;
    if items.next().is_some() {
        return Err(CheckpointError::Lineage("settlement result cardinality").into());
    }
    let native = CodexItem::parse(serde_json::to_value(item)?)?;
    match (native, custom) {
        (CodexItem::FunctionCallOutput { call_id, output }, true) => {
            Ok(CodexItem::CustomToolCallOutput { call_id, output })
        }
        (native @ CodexItem::FunctionCallOutput { .. }, false) => Ok(native),
        _ => Err(CheckpointError::Lineage("settlement result encoding").into()),
    }
}

pub(super) fn settle(
    model: &CodexResponsesModel,
    history: &[Message],
) -> Result<Vec<Message>, CompletionError> {
    let Some(mut checkpoint) = model.history.snapshot()? else {
        return Ok(Vec::new());
    };
    if checkpoint.model.as_deref() != Some(model.model.as_str()) {
        return Err(CheckpointError::Lineage("settlement model").into());
    }
    let mut changed = false;
    for message in history {
        let Message::User { content } = message else {
            continue;
        };
        for content in content.iter() {
            let UserContent::ToolResult(result) = content else {
                continue;
            };
            let mut matched = None;
            for item in &checkpoint.items {
                if let Some(call) = model.history.tool_call(item)? {
                    if call.id == result.id && call.call_id == result.call_id {
                        matched = Some(matches!(item, CodexItem::CustomToolCall { .. }));
                        break;
                    }
                }
            }
            let custom =
                matched.ok_or(CheckpointError::Lineage("settlement result call identity"))?;
            let native = outcome(result, custom)?;
            if let Some(existing) = checkpoint
                .items
                .iter()
                .find(|item| outcome_id(item) == outcome_id(&native))
            {
                if existing != &native {
                    return Err(CheckpointError::Lineage("conflicting settlement result").into());
                }
            } else {
                checkpoint.items.push(native);
                changed = true;
            }
        }
    }
    let mut pending = Vec::new();
    for item in &checkpoint.items {
        let Some(call) = model.history.tool_call(item)? else {
            continue;
        };
        if checkpoint
            .items
            .iter()
            .any(|item| outcome_id(item) == call.call_id.as_deref())
        {
            continue;
        }
        let record = json!({
            "status": "outcome_unknown",
            "source": "compaction_handoff",
            "tool_result_available": false,
            "execution_may_continue": true,
            "retry_permitted": false,
            "message": "No genuine tool result was captured at exploration cutoff. This runtime handoff record is not a tool-produced result. Execution may still be running or may never have started. Do not infer success, cancellation or non-execution. Do not retry; use only other gathered evidence."
        });
        pending.push(Message::tool_result_with_call_id(
            call.id,
            call.call_id,
            record.to_string(),
        ));
    }
    let mut request = json!({"model":checkpoint.model, "input":checkpoint.items});
    match model
        .history
        .prepare_with_tools(&mut request, &model.custom_tools)
    {
        Ok(()) => {}
        Err(CompletionError::RequestError(source))
            if matches!(
                source.downcast_ref::<CheckpointError>(),
                Some(CheckpointError::MissingOutcome(_))
            ) => {}
        Err(error) => return Err(error),
    }
    if changed {
        model
            .history
            .commit_for_generation(&request, &[], checkpoint.generation)?;
    }
    Ok(pending)
}
