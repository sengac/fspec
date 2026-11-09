use crate::session::Session;
use codelet_providers::codex::{history::CheckpointError, websocket::WebSocketUpgradeRejected};

pub fn block_terminal_request_failure(session: &mut Session, error: &anyhow::Error) -> bool {
    let checkpoint = error
        .chain()
        .any(|source| source.downcast_ref::<CheckpointError>().is_some());
    let terminal = checkpoint
        || error
            .chain()
            .any(|source| source.downcast_ref::<WebSocketUpgradeRejected>().is_some());
    if terminal {
        // BUG-683: a provider-state failure is resolved by reloading the
        // saved state, which closing and resuming the session does.
        let instruction = if checkpoint {
            ". Close this session and resume it to reload its saved state"
        } else {
            ""
        };
        session.block_compaction_inference(format!(
            "selected request requires resolution: {}{instruction}",
            distinct_causes(error)
        ));
    }
    terminal
}

/// BUG-683: the error chain once per cause. A layer whose text already
/// appears in the previous kept layer only repeats it, as rig's
/// `RequestError` wrappers do, so it is left out.
fn distinct_causes(error: &anyhow::Error) -> String {
    let mut causes: Vec<String> = Vec::new();
    for cause in error.chain().map(ToString::to_string) {
        if causes.last().is_none_or(|kept| !kept.contains(&cause)) {
            causes.push(cause);
        }
    }
    causes.join(": ")
}
