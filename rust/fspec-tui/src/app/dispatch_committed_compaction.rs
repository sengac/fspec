use super::state::App;
use crate::components::Action;
use codelet_rpc_types::{CommittedContextSnapshot, SessionId, StreamChunk};

impl App {
    pub(crate) fn apply_committed_compaction(
        &mut self,
        session: &SessionId,
        snapshot: &CommittedContextSnapshot,
    ) {
        if self
            .agent_view_store
            .token_state_for(session)
            .is_some_and(|state| {
                state
                    .compaction_generation
                    .is_some_and(|generation| generation >= snapshot.generation)
                    || state
                        .committed_generation
                        .is_some_and(|generation| generation > snapshot.generation)
            })
        {
            return;
        }
        self.agent_view_store.apply_chunk_to_token_state(
            session,
            &StreamChunk::CommittedCompaction {
                snapshot: snapshot.clone(),
            },
        );
        self.agent_view_store.clear_compaction_progress(session);
        if snapshot.reconciled {
            return;
        }
        self.agent_view_store.set_compaction_reduction(
            session.clone(),
            snapshot.reduction_percentage().round() as i32,
        );
        let sequence = self
            .agent_view_store
            .bump_compaction_reduction_seq(session.clone());
        self.arm_compaction_hide(session.clone(), sequence);
        let coverage = snapshot.covered_turns.map_or_else(
            || "coverage unknown".to_owned(),
            |turns| format!("{turns} turns summarised"),
        );
        let basis = if snapshot.estimated {
            "estimated"
        } else {
            "measured"
        };
        let text = format!(
            "[compaction] generation {}: {:.1}% reduction ({} → {} tokens, {basis}, {coverage}; {:.1}% of trigger, capacity {} tokens)",
            snapshot.generation, snapshot.reduction_percentage(), snapshot.original_tokens,
            snapshot.context_fill.effective_tokens, snapshot.context_fill.fill_percentage,
            snapshot.context_fill.context_window,
        );
        let _ = self
            .action_tx
            .send(Action::EmitSessionNotice(session.clone(), text));
    }
}
