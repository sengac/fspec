use super::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolResultEvent};
use std::sync::Arc;

struct SettlementEmitter {
    output: BackgroundOutput,
    _ownership: tokio::sync::OwnedMutexGuard<()>,
}

impl StreamOutput for SettlementEmitter {
    fn emit(&self, event: StreamEvent) {
        self.output.emit(event);
    }

    fn emit_tool_cancellation(&self, result: ToolResultEvent) {
        self.output.emit_tool_cancellation(result);
    }
}

impl BackgroundOutput {
    pub(super) fn owned_settlement_emitter(&self) -> Result<Arc<dyn StreamOutput>, String> {
        Ok(Arc::new(SettlementEmitter {
            _ownership: self.session.begin_settlement()?,
            output: self.clone(),
        }))
    }
}
