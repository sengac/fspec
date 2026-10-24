use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::compaction_boundary::CompactionIdentity;
use codelet_rpc_types::{CommittedContextSnapshot, RequestContextSnapshot};
use std::time::Duration;

pub fn advance_clock() {
    let runtime = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        runtime.block_on(tokio::time::advance(Duration::from_secs(61)));
    })
    .join()
    .expect("advance paused publication clock");
}

pub struct ExpiringOutput(pub BackgroundOutput);

impl StreamOutput for ExpiringOutput {
    fn emit(&self, event: StreamEvent) {
        self.0.emit(event);
    }

    fn publish_committed_context(
        &self,
        identity: &CompactionIdentity,
        snapshot: CommittedContextSnapshot,
        context: RequestContextSnapshot,
    ) -> Result<(), String> {
        advance_clock();
        self.0
            .publish_committed_context(identity, snapshot, context)
    }
}
