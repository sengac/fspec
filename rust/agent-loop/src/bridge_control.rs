use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;

pub(crate) fn clear_history(session: &BackgroundSession) {
    if let Err(error) = session.clear_history() {
        tracing::warn!(%error, "Bridge conversation clear failed");
        session.handle_output(StreamChunk::Error {
            error: format!("Failed to clear conversation: {error}"),
        });
    }
}
