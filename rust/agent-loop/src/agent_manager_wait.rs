use codelet_rpc_types::{SessionState, StreamChunk};
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

pub(super) async fn wait(
    mut receiver: Receiver<StreamChunk>,
    is_idle: impl Fn() -> Option<bool>,
) -> AwaitOutcome {
    loop {
        codelet_tools::profile_scope!("handle_await_idle::per_session_recv_loop");
        match receiver.recv().await {
            Ok(StreamChunk::SessionStateChange {
                state: SessionState::Idle,
            }) => return AwaitOutcome::Idle,
            Ok(_) => {}
            Err(RecvError::Closed) => return AwaitOutcome::Destroyed,
            Err(RecvError::Lagged(_)) => {
                codelet_tools::profile_scope!("handle_await_idle::lagged_continue");
                match is_idle() {
                    Some(true) => return AwaitOutcome::Idle,
                    None => return AwaitOutcome::Destroyed,
                    Some(false) => {}
                }
            }
        }
    }
}
