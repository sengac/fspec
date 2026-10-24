use super::{CheckpointError, CodexHistory, CodexItem};
use rig::completion::CompletionError;
use rig::message::{ToolCall, ToolFunction};

impl CodexHistory {
    pub fn tool_call(&self, item: &CodexItem) -> Result<Option<ToolCall>, CompletionError> {
        let (id, call_id) = match item {
            CodexItem::FunctionCall { id, call_id, .. } => {
                (id.as_ref().unwrap_or(call_id), call_id)
            }
            CodexItem::CustomToolCall { id, call_id, .. } => (id, call_id),
            _ => return Ok(None),
        };
        let tools = self
            .state
            .lock()
            .map_err(|_| CheckpointError::Inaccessible("state"))?
            .tools
            .clone();
        let normalized = tools
            .normalize_call(&serde_json::to_value(item)?)
            .map_err(|error| CompletionError::RequestError(Box::new(error)))?;
        let name = normalized["name"]
            .as_str()
            .ok_or(CheckpointError::Lineage("normalized tool name"))?;
        let arguments = normalized["arguments"]
            .as_str()
            .ok_or(CheckpointError::Lineage("normalized tool arguments"))?;
        Ok(Some(
            ToolCall::new(
                id.clone(),
                ToolFunction::new(name.to_owned(), serde_json::from_str(arguments)?),
            )
            .with_call_id(call_id.clone()),
        ))
    }
}
