use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use std::sync::Arc;

pub(super) struct IdleOnDropGuard {
    pub session: Arc<BackgroundSession>,
    pub armed: bool,
}

impl Drop for IdleOnDropGuard {
    fn drop(&mut self) {
        if self.armed {
            tracing::warn!(
                "AMGR-016: IdleOnDropGuard fired (panic or early exit) — forcing session {} to Idle",
                self.session.id
            );
            self.session.set_status(SessionStatus::Idle);
            self.session.handle_output(StreamChunk::done());
        }
    }
}
