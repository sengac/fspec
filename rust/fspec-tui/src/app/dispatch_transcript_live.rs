use codelet_rpc_types::{SessionId, StreamChunk};

use super::state::App;

impl App {
    /// BUG-648: a journaled chunk arrived live. Its state and transcript
    /// are applied from the transcript pages (in journal order, exactly
    /// once, the same as on any attach); here it wakes the page fetch and
    /// runs the one-shot effects that belong to the moment it arrives.
    pub(crate) fn handle_live_transcript_wake(&mut self, session: &SessionId, chunk: &StreamChunk) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.apply_live_interrupt_queue(chunk);
        }
        self.maybe_push_error_dialog_for_chunk(chunk);
        match chunk {
            StreamChunk::FspecCommandRequest { fspec_request } => {
                self.spawn_fspec_command_runner(session.clone(), fspec_request.clone());
            }
            StreamChunk::CompactionComplete { compaction_result } => {
                self.show_compaction_reduction(session, compaction_result);
            }
            _ => {}
        }
        self.request_transcript(session);
    }

    /// BUG-648: apply the state a journaled chunk carries, as a transcript
    /// page delivers it. The one-shot effects (an fspec command request, the
    /// error dialog, the compaction badge) are never re-run from a page.
    pub(crate) fn apply_journaled_state(&mut self, session: &SessionId, chunk: &StreamChunk) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.apply_provider_waiting(chunk);
        }
        if let StreamChunk::CommittedCompaction { snapshot } = chunk {
            self.apply_committed_compaction(session, snapshot);
        } else {
            self.agent_view_store
                .apply_chunk_to_token_state(session, chunk);
        }
        match chunk {
            StreamChunk::FspecCommandRequest { .. } => {}
            StreamChunk::CompactionComplete { compaction_result } => {
                self.record_compaction_complete(session, compaction_result);
            }
            _ => self.handle_stream_chunk_state_updates(session, chunk),
        }
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

/// BUG-648: a live chunk the session journals only wakes its transcript
/// pages; a kind it never journals (workspace chrome) is applied now.
pub(super) fn transcript_push_action(
    session: SessionId,
    chunk: StreamChunk,
) -> crate::components::Action {
    if chunk.is_journaled() {
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
