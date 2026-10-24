use rig::message::{ToolResult, ToolResultContent};

#[derive(Debug, thiserror::Error)]
#[error("summary submission blocked ordinary inference: {reason}")]
pub struct CompactionHandoffError {
    pub reason: String,
}

impl crate::session::Session {
    pub fn block_compaction_inference(&mut self, reason: String) {
        if let Some(operation) = &mut self.compaction_operation {
            operation.prevent_fresh_retry();
        }
        self.compaction_inference_block = Some(reason);
    }

    pub(crate) fn restart_failed_compaction_builder(&mut self) -> bool {
        if self.pending_compaction_commit.is_some() || self.compaction_inference_block.is_none() {
            return false;
        }
        self.compaction_operation
            .as_mut()
            .is_some_and(crate::compaction_operation::CompactionOperation::restart_failed_builder)
    }

    pub fn ensure_compaction_inference_ready(&self) -> Result<(), CompactionHandoffError> {
        match &self.compaction_inference_block {
            Some(reason) => Err(CompactionHandoffError {
                reason: reason.clone(),
            }),
            None => Ok(()),
        }
    }
}

pub(super) fn requires_handoff(
    tool_name: Option<&str>,
    tool_arguments: Option<&serde_json::Value>,
    caller_id: uuid::Uuid,
    result: &ToolResult,
) -> Result<bool, CompactionHandoffError> {
    let self_generation = tool_name == Some("GenerateCompaction")
        && tool_arguments
            .and_then(|arguments| arguments["session_id"].as_str())
            .is_none_or(|target| target.trim().parse::<uuid::Uuid>() == Ok(caller_id));
    if tool_name != Some("inject_summary") && !self_generation {
        return Ok(false);
    }
    let text = result
        .content
        .iter()
        .filter_map(|content| match content {
            ToolResultContent::Text(text) => Some(text.text.as_str()),
            ToolResultContent::Image(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let value =
        serde_json::from_str::<serde_json::Value>(&text).and_then(|value| match value.as_str() {
            Some(encoded) => serde_json::from_str(encoded),
            None => Ok(value),
        });
    match value {
        Ok(value) if value["status"] == "queued" => Ok(true),
        _ => Err(CompactionHandoffError { reason: text }),
    }
}
