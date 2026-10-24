use codelet_rpc_types::{SessionId, StreamChunk};

use super::state::App;

impl App {
    pub(crate) fn handle_live_transcript_wake(&mut self, session: &SessionId, chunk: &StreamChunk) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.apply_provider_waiting(chunk);
        }
        self.maybe_push_error_dialog_for_chunk(chunk);
        self.request_transcript(session);
    }

    pub(crate) fn handle_chunk_received(&mut self, session: &SessionId, chunk: &StreamChunk) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.record_chunk(chunk);
        }
        if let StreamChunk::CommittedCompaction { snapshot } = chunk {
            self.apply_committed_compaction(session, snapshot);
        } else {
            self.agent_view_store
                .apply_chunk_to_token_state(session, chunk);
        }
        self.handle_stream_chunk_state_updates(session, chunk);
        self.maybe_push_error_dialog_for_chunk(chunk);
        if matches!(chunk, StreamChunk::ResponseAttemptAborted)
            && self.agent_view_store.current_session() == Some(session)
        {
            self.navigator
                .agent
                .dismiss_retracted_turn_modal(&self.agent_view_store);
        }
    }
}

pub(super) fn transcript_push_action(
    session: SessionId,
    chunk: StreamChunk,
) -> crate::components::Action {
    if is_transcript_content(&chunk) {
        crate::components::Action::Transcript(super::transcript_sync::TranscriptAction::Live(
            session, chunk,
        ))
    } else {
        crate::components::Action::ChunkReceived(session, chunk)
    }
}

pub(super) fn is_transcript_content(chunk: &StreamChunk) -> bool {
    matches!(
        chunk,
        StreamChunk::Text { .. }
            | StreamChunk::Thinking { .. }
            | StreamChunk::UserInput { .. }
            | StreamChunk::ToolCall { .. }
            | StreamChunk::ToolResult { .. }
            | StreamChunk::ToolProgress { .. }
            | StreamChunk::Done
            | StreamChunk::Error { .. }
            | StreamChunk::Interrupted { .. }
            | StreamChunk::InterruptedWithReason { .. }
            | StreamChunk::ResponseAttemptStarted
            | StreamChunk::ResponseAttemptAborted
            | StreamChunk::UserNotification { .. }
            | StreamChunk::IncomingMessage { .. }
    )
}
