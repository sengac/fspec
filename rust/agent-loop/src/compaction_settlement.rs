use rig::completion::{CompletionError, CompletionModel};
use rig::message::{Message, UserContent};

#[derive(Debug, thiserror::Error)]
pub enum SettlementError {
    #[error("Compactor handoff settlement failed: {0}")]
    Provider(#[from] CompletionError),
}

pub fn reconcile<M: CompletionModel>(
    model: &M,
    history: &[Message],
) -> Result<Vec<Message>, SettlementError> {
    let handoff = model.settle_tool_results(history)?;
    if handoff.is_empty() {
        return Ok(handoff);
    }
    let call_ids: Vec<_> = handoff
        .iter()
        .filter_map(|message| match message {
            Message::User { content } => Some(content),
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|content| match content {
            UserContent::ToolResult(result) => result.call_id.as_deref(),
            _ => None,
        })
        .collect();
    tracing::warn!(
        operation_id = ?codelet_cli::compaction_timing::operation_id(),
        phase = "settlement",
        pending_call_ids = ?call_ids,
        "Compactor handoff records unknown execution outcomes without retrying tools"
    );
    Ok(handoff)
}
