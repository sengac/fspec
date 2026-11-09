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

    /// BUG-641: release a threshold compaction whose builder failed before
    /// anything was installed, so the turn carries on uncompacted with a fresh
    /// budget for the next attempt. False when the failure must keep holding
    /// (a pending commit, or a failure that is not the builder's).
    pub(crate) fn release_failed_threshold_compaction(&mut self) -> bool {
        if !self.restart_failed_compaction_builder() {
            return false;
        }
        self.compaction_inference_block = None;
        true
    }

    /// BUG-516: make the next agent-stream run a manual compaction.
    pub fn request_manual_compaction(&mut self) {
        self.manual_compaction_requested = true;
    }

    pub(crate) fn take_manual_compaction_request(&mut self) -> bool {
        std::mem::take(&mut self.manual_compaction_requested)
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

/// BUG-614: what a summary submission's tool result means for the turn.
/// No result can block inference: nothing reaches durable state before the
/// turn's reconciliation commits a queued candidate.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum HandoffOutcome {
    /// Not a summary submission, or a rejection the model may still correct.
    Continue,
    /// A candidate awaits durable application; inference yields.
    Queued,
    /// Nothing was installed and no further correction is expected.
    NotInstalled(String),
}

pub(super) fn handoff_outcome(
    tool_name: Option<&str>,
    tool_arguments: Option<&serde_json::Value>,
    caller_id: uuid::Uuid,
    result: &ToolResult,
) -> HandoffOutcome {
    let self_generation = tool_name == Some("GenerateCompaction")
        && tool_arguments
            .and_then(|arguments| arguments["session_id"].as_str())
            .is_none_or(|target| target.trim().parse::<uuid::Uuid>() == Ok(caller_id));
    if tool_name != Some("inject_summary") && !self_generation {
        return HandoffOutcome::Continue;
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
        Ok(value) if value["status"] == "queued" => HandoffOutcome::Queued,
        Ok(value) if value["status"] == "rejected" => HandoffOutcome::Continue,
        Ok(value) if value["status"] == "rejected_final" => HandoffOutcome::NotInstalled(
            value["error"]
                .as_str()
                .map_or_else(|| text.clone(), str::to_owned),
        ),
        _ => HandoffOutcome::NotInstalled(text),
    }
}
