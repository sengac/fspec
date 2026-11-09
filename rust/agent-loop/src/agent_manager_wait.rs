use codelet_rpc_types::{RequestRoute, RequestStatus, SessionState, StreamChunk};
use codelet_tools::agent_manager::AwaitOutcome;
use tokio::sync::broadcast::{error::RecvError, Receiver};

pub(super) fn subscribe_before_check<Chunk>(
    subscribe: impl FnOnce() -> Receiver<Chunk>,
    is_idle: impl FnOnce() -> bool,
) -> Option<Receiver<Chunk>> {
    let receiver = subscribe();
    if is_idle() {
        None
    } else {
        Some(receiver)
    }
}

/// How one awaited session's wait ended.
#[derive(Debug)]
pub(super) enum WaitEnd {
    Outcome(AwaitOutcome),
    /// AMGR-021: a request at or below the session concerns the caller
    /// (`needs_input` or `waiting_on_user`).
    Request(AwaitOutcome, RequestRoute),
}

/// Wait until the session behind `receiver` is idle or gone, or (AMGR-021)
/// a request raised, escalated or passed up at or below it concerns the
/// caller (`concerns`). When chunks were lost, `is_idle` and `held` re-read
/// the state.
pub(super) async fn wait(
    mut receiver: Receiver<StreamChunk>,
    is_idle: impl Fn() -> Option<bool>,
    concerns: impl Fn(&RequestRoute) -> Option<AwaitOutcome>,
    held: impl Fn() -> Option<(AwaitOutcome, RequestRoute)>,
) -> WaitEnd {
    loop {
        codelet_tools::profile_scope!("handle_await_idle::per_session_recv_loop");
        match receiver.recv().await {
            Ok(StreamChunk::SessionStateChange {
                state: SessionState::Idle,
            }) => return WaitEnd::Outcome(AwaitOutcome::Idle),
            Ok(StreamChunk::InteractionRequestChanged {
                route,
                status: RequestStatus::Pending,
            }) => {
                if let Some(outcome) = concerns(&route) {
                    return WaitEnd::Request(outcome, route);
                }
            }
            Ok(_) => {}
            Err(RecvError::Closed) => return WaitEnd::Outcome(AwaitOutcome::Destroyed),
            Err(RecvError::Lagged(_)) => {
                codelet_tools::profile_scope!("handle_await_idle::lagged_continue");
                if let Some((outcome, route)) = held() {
                    return WaitEnd::Request(outcome, route);
                }
                match is_idle() {
                    Some(true) => return WaitEnd::Outcome(AwaitOutcome::Idle),
                    None => return WaitEnd::Outcome(AwaitOutcome::Destroyed),
                    Some(false) => {}
                }
            }
        }
    }
}
