//! Builder context derived only from a validated durable manifest, never marker text.
use super::Session;
use codelet_core::persistence::SessionManifest;

#[derive(Debug)]
pub(crate) struct DurableSummary {
    session_id: uuid::Uuid,
    revision: u64,
    generation: u64,
    content: String,
    last_compacted_turn: usize,
}

impl DurableSummary {
    pub(super) fn builder_dag(&self) -> Result<(String, usize), String> {
        let current = codelet_core::persistence::load_session(self.session_id)?;
        codelet_core::persistence::compaction_boundary::validate_committed_boundary(&current)?;
        let generation = current
            .compaction_commit
            .as_ref()
            .map_or(0, |commit| commit.identity.generation);
        let pending = current
            .compaction_commit
            .as_ref()
            .is_some_and(|commit| commit.publication_pending);
        let summary_matches = current.compaction.as_ref().is_some_and(|state| {
            state.summary == self.content
                && state.compacted_before_index.saturating_sub(1) == self.last_compacted_turn
        });
        if current.revision < self.revision
            || generation != self.generation
            || pending
            || !summary_matches
        {
            return Err("durable summary identity changed; restore the authoritative boundary before building".into());
        }
        tracing::debug!(session_id=%self.session_id,revision=current.revision,
            generation=self.generation,"Preparing builder from validated durable summary");
        Ok((self.content.clone(), self.last_compacted_turn))
    }
}

impl Session {
    /// Call only after boundary/provider restore or durable reconciliation succeeds.
    /// This is prompt context, not a commit certificate: promotion still verifies
    /// authoritative source identity, generation, ranges and preservation obligations.
    pub(super) fn install_durable_summary(&mut self, manifest: &SessionManifest) {
        self.durable_summary = manifest.compaction.as_ref().map(|state| DurableSummary {
            session_id: manifest.id,
            revision: manifest.revision,
            generation: manifest
                .compaction_commit
                .as_ref()
                .map_or(0, |commit| commit.identity.generation),
            content: state.summary.clone(),
            last_compacted_turn: state.compacted_before_index.saturating_sub(1),
        });
    }
}
