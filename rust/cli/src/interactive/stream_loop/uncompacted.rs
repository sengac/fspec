//! BUG-641: a threshold compaction whose builder fails while the context
//! still fits the usable window carries on uncompacted (product owner,
//! 2026-11-08, BUG-614 Q3). Only a context that does not fit holds the turn.

use super::super::output::StreamOutput;
use crate::{compaction_operation::CompactionFailure, session::Session};
use codelet_core::estimate_messages_tokens;
use rig::message::Message;

/// What the turn occupies: the larger of the tracked count and the estimate
/// of the payload about to be sent, the same measure the compaction hook uses.
pub(super) fn occupied_tokens(tracked: u64, history: &[Message], prompt: &str) -> u64 {
    let mut payload = history.to_vec();
    payload.push(Message::user(prompt));
    tracked.max(estimate_messages_tokens(&payload) as u64)
}

/// Carry on uncompacted after a failed threshold compaction, or say exactly
/// why the turn is held. On `Ok` the session is released, the next attempt
/// has a fresh budget and the turn is running again (the failure's warning
/// has already been shown); for the rest of the turn compaction triggers
/// only past `usable_window`.
pub(super) fn carry_on<Output: StreamOutput>(
    session: &mut Session,
    output: &Output,
    failure: CompactionFailure,
    occupied: u64,
    usable_window: u64,
) -> anyhow::Result<()> {
    // A missing builder is a wiring fault, not a failed attempt: it fails
    // closed (CMPCT-079), like commit, publication and cancellation failures.
    if !failure.is_builder_failure() || matches!(failure, CompactionFailure::Unavailable) {
        return Err(failure.into());
    }
    if occupied > usable_window {
        // The failure stays the error's source, so callers still see it.
        let held = format!(
            "context of {occupied} tokens does not fit the usable window of {usable_window} tokens: {failure}"
        );
        return Err(anyhow::Error::new(failure).context(held));
    }
    if !session.release_failed_threshold_compaction() {
        return Err(failure.into());
    }
    tracing::warn!(occupied, usable_window, %failure,
        "BUG-641: threshold compaction failed; the turn carries on uncompacted");
    output.emit_compaction_continuing();
    Ok(())
}
