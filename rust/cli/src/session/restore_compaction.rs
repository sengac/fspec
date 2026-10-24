use super::Session;
use codelet_core::persistence::compaction_boundary::validate_committed_boundary;
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::SessionManifest;

impl Session {
    pub fn preflight_compaction_restore(
        &self,
        manifest: &SessionManifest,
        envelopes: &[String],
    ) -> Result<(), CompactionCommitError> {
        validate_committed_boundary(manifest).map_err(CompactionCommitError::Rejected)?;
        codelet_core::persistence::envelopes_for_manifest(manifest)
            .map_err(CompactionCommitError::Rejected)?;
        if let Some(boundary) = &manifest.compaction_commit {
            if !self.matches_persisted_selection(&manifest.provider, &boundary.checkpoint.model) {
                return Err(CompactionCommitError::Rejected(
                    "restored selected identity differs".into(),
                ));
            }
            let metadata = serde_json::from_value(boundary.checkpoint.payload.clone())
                .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
            self.provider_session
                .for_compaction_checkpoint(self.current_provider_name(), &metadata)
                .validate_committed_restore(boundary.identity.generation, &metadata, envelopes)
                .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
        } else {
            self.provider_session
                .validate_restore_for_provider(&manifest.provider, envelopes)
                .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
            if manifest.provider != self.current_provider_name() {
                self.provider_session
                    .validate_restore_for_provider(self.current_provider_name(), envelopes)
                    .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn restore_compaction_boundary(
        &mut self,
        manifest: &SessionManifest,
    ) -> Result<(), CompactionCommitError> {
        self.block_compaction_inference("restoring authoritative compaction boundary".into());
        validate_committed_boundary(manifest).map_err(CompactionCommitError::Rejected)?;
        let envelopes = codelet_core::persistence::envelopes_for_manifest(manifest)
            .map_err(CompactionCommitError::Rejected)?;
        self.preflight_compaction_restore(manifest, &envelopes)?;
        if let Some(boundary) = &manifest.compaction_commit {
            if !self.matches_persisted_selection(&manifest.provider, &boundary.checkpoint.model) {
                return Err(CompactionCommitError::Rejected(
                    "restored selected identity differs".into(),
                ));
            }
            let metadata = serde_json::from_value(boundary.checkpoint.payload.clone())
                .map_err(|error| CompactionCommitError::Rejected(error.to_string()))?;
            let provider_session = self
                .provider_session
                .for_compaction_checkpoint(self.current_provider_name(), &metadata);
            provider_session
                .install_committed_generation(boundary.identity.generation, &metadata)
                .map_err(|error| CompactionCommitError::Publication {
                    identity: boundary.identity.clone(),
                    reason: error.to_string(),
                })?;
            provider_session
                .restore_for_provider(self.current_provider_name(), &envelopes)
                .map_err(|error| CompactionCommitError::Publication {
                    identity: boundary.identity.clone(),
                    reason: error.to_string(),
                })?;
        }
        self.pending_compaction_commit = manifest
            .compaction_commit
            .as_ref()
            .filter(|boundary| boundary.publication_pending)
            .map(|boundary| boundary.identity.clone());
        self.restore_durable_tokens(manifest);
        if self.pending_compaction_commit.is_none() {
            self.compaction_inference_block = None;
        }
        Ok(())
    }

    pub(super) fn restore_durable_tokens(&mut self, manifest: &SessionManifest) {
        let saved = &manifest.token_usage;
        self.token_tracker.input_accounting = codelet_core::compaction::InputAccounting::Restored;
        self.token_tracker.input_tokens = saved.current_context_tokens;
        self.token_tracker.output_tokens = saved.cumulative_billed_output;
        self.token_tracker.cumulative_billed_input = saved.cumulative_billed_input;
        self.token_tracker.cumulative_billed_output = saved.cumulative_billed_output;
        self.token_tracker.cache_read_input_tokens = Some(saved.cache_read_tokens);
        self.token_tracker.cache_creation_input_tokens = Some(saved.cache_creation_tokens);
        self.token_tracker.reasoning_tokens = saved.reasoning_tokens;
    }
}
