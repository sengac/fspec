use crate::session::Session;
use codelet_providers::codex::{history::CheckpointError, websocket::WebSocketUpgradeRejected};

pub(super) fn block_terminal_request_failure(session: &mut Session, error: &anyhow::Error) -> bool {
    let terminal = error.chain().any(|source| {
        source.downcast_ref::<CheckpointError>().is_some()
            || source.downcast_ref::<WebSocketUpgradeRejected>().is_some()
    });
    if terminal {
        session
            .block_compaction_inference(format!("selected request requires resolution: {error:#}"));
    }
    terminal
}
