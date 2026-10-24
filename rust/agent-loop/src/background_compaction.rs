use super::BackgroundOutput;
use codelet_core::persistence::compaction_boundary::CompactionIdentity;
use codelet_rpc_types::{
    CommittedContextSnapshot, CompactionResult, RequestContextSnapshot, SessionState,
    SessionStatus, StreamChunk,
};

impl BackgroundOutput {
    pub(super) fn publish_compaction_context(
        &self,
        identity: &CompactionIdentity,
        snapshot: CommittedContextSnapshot,
        context: RequestContextSnapshot,
    ) -> Result<(), String> {
        if identity.session_id != self.session.id
            || u64::try_from(snapshot.generation).ok() != Some(identity.generation)
            || context.generation != snapshot.generation
        {
            return Err("committed publication target or generation differs".into());
        }
        self.session
            .handle_output(StreamChunk::CommittedCompaction { snapshot });
        self.session
            .handle_output(StreamChunk::ContextSnapshot { snapshot: context });
        Ok(())
    }

    pub(super) fn complete_compaction(&self, result: CompactionResult) {
        self.session.set_status(SessionStatus::Idle);
        self.session
            .handle_output(StreamChunk::session_state_change(SessionState::Idle));
        self.complete_committed_compaction(result);
    }

    pub(super) fn complete_committed_compaction(&self, result: CompactionResult) {
        self.session.set_compaction_progress(None);
        self.session
            .handle_output(StreamChunk::compaction_complete(result));
    }
}
